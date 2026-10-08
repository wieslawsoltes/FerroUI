// Behaviour tests of the browser storage provider on the storage_view example, in headless Chrome.
//
//   scripts/build-browser.sh storage_view
//   node scripts/browser/tests/storage_view.test.mjs [<site directory>]     default: target/browser/storage_view
//
// Scenarios run in the page through the `storageViewRun` export of the example, which uses the storage
// provider of the top-level as an application would; their outcome is read with `storageViewResult`.
// The pickers of the page are replaced by functions that hand out handles of the origin private file
// system (installed before the storage bundle is first imported, which is when the polyfill looks for
// the native pickers), except in the polyfill checks: there the polyfill shows its own file input, which
// the test answers through the DevTools protocol, and saves through a download: a blob link without the
// service worker of the platform, a response streamed by that worker when it is registered
// (`?RegisterServiceWorker=true`).
//
// The same checks run against a site built with threads, which is recognised by the file the build
// adds to such a site (ferroui-threads.js) and served cross-origin isolated:
//
//   scripts/build-browser.sh storage_view --threads
//   node scripts/browser/tests/storage_view.test.mjs target/browser-threads/storage_view
//
// Three things differ there: the memory of the module is fixed, so the large write does not grow it;
// the service worker is registered as `ferroui-sw.js?coi=1`; and one more check opens the page
// without the headers, where that one worker has to isolate the page and stream the download.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, sleep } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser", "storage_view");
const threaded = fs.existsSync(path.join(site, "ferroui-threads.js"));
// The address the service worker is registered with (interop/ferro_module.rs, ferroui-threads.js).
const SERVICE_WORKER = threaded ? "ferroui-sw.js?coi=1" : "ferroui-sw.js";

// Installs the pickers of the tests and a fresh origin private file system:
// input.txt ("first file"), second.txt ("second file"), saved.txt (empty) and the folder work/.
const STUBS = `(async () => {
    const root = await navigator.storage.getDirectory();
    for await (const [name] of root.entries()) { await root.removeEntry(name, { recursive: true }); }
    const write = async (dir, name, text) => {
        const handle = await dir.getFileHandle(name, { create: true });
        const writable = await handle.createWritable(); await writable.write(text); await writable.close();
        return handle;
    };
    const input = await write(root, "input.txt", "first file");
    const second = await write(root, "second.txt", "second file");
    const saved = await root.getFileHandle("saved.txt", { create: true });
    const work = await root.getDirectoryHandle("work", { create: true });
    globalThis.pickerCalls = [];
    globalThis.pickerResult = { open: [input], save: saved, folder: work, cancel: false };
    const record = (kind, options) => {
        const { startIn, ...rest } = options ?? {};
        pickerCalls.push({ kind, startIn: typeof startIn === "string" ? startIn : typeof startIn, ...rest });
        if (pickerResult.cancel) { throw new DOMException("The user aborted a request.", "AbortError"); }
    };
    globalThis.showOpenFilePicker = async (options) => { record("open", options); return pickerResult.open; };
    globalThis.showSaveFilePicker = async (options) => { record("save", options); return pickerResult.save; };
    globalThis.showDirectoryPicker = async (options) => { record("folder", options); return pickerResult.folder; };
    globalThis.opfsText = async (...names) => {
        let dir = await navigator.storage.getDirectory();
        for (const name of names.slice(0, -1)) { dir = await dir.getDirectoryHandle(name); }
        return await (await (await dir.getFileHandle(names[names.length - 1])).getFile()).text();
    };
    globalThis.opfsHandle = async (name) => (await navigator.storage.getDirectory()).getFileHandle(name);
    return true;
})()`;

// `isolated`: whether the server sends the headers of cross-origin isolation (default: to a threaded
// site, which cannot start without them unless its service worker adds them).
async function start({ query = "", stubs = true, isolated = threaded } = {}) {
    const page = await open(site, { query, width: 320, height: 200, isolated });
    await page.waitForView(200);
    if (stubs) { await page.evaluate(STUBS); }
    // Runs a scenario and resolves to its outcome as an object of name/value pairs.
    page.scenario = async (name, argument = "", { timeout = 30000, userGesture = false, failing = false } = {}) => {
        const call = `storageView.storageViewRun(${JSON.stringify(name)}, ${JSON.stringify(argument)})`;
        await page.send("Runtime.evaluate", { expression: call, userGesture });
        await page.waitFor(`storageView.storageViewResult(${JSON.stringify(name)}) !== ""`, timeout);
        const line = await page.evaluate(`storageView.storageViewResult(${JSON.stringify(name)})`);
        const result = Object.fromEntries(line.split(";").map((pair) => {
            const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)];
        }));
        assert(failing || result.error === undefined, `scenario ${name} failed: ${line}`);
        return result;
    };
    page.calls = async () => JSON.parse(await page.evaluate("JSON.stringify(pickerCalls)"));
    return page;
}

const checks = [];
const check = (name, body, options) => checks.push([name, async () => {
    const page = await start(options);
    try { await body(page); } catch (error) { error.message += "\n" + page.log.slice(-8).join("\n"); throw error; } finally { await page.close(); }
}]);
const expect = (actual, expected, what) => assert(actual === expected, `${what}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);

check("the provider can open, save and pick folders", async (page) => {
    const result = await page.scenario("capabilities");
    expect(result.can_open, "true", "can_open"); expect(result.can_save, "true", "can_save"); expect(result.can_pick_folder, "true", "can_pick_folder");
});

check("the open picker returns the picked file, whose stream reads its content", async (page) => {
    const result = await page.scenario("open_read");
    expect(result.count, "1", "files"); expect(result.names, "input.txt", "names"); expect(result.paths, "input.txt", "relative path");
    expect(result.content, "first file", "content"); expect(result.sizes, "Some(10)/true", "size and modification date");
    const [call] = await page.calls();
    expect(call.kind, "open", "picker"); expect(call.multiple, false, "multiple");
    expect(call.excludeAcceptAllOption, true, "a filter without the any-file type excludes the accept-all option");
    expect(JSON.stringify(call.types), JSON.stringify([
        { description: "Plain Text", accept: { "text/plain": [".txt"] } },
        { description: "Data", accept: { "application/octet-stream": [".bin"] } }
    ]), "accept types");
    expect(call._preferPolyfill, false, "polyfill preference");
});

check("the open picker returns several files when multiple selection is allowed", async (page) => {
    await page.evaluate("(async () => { pickerResult.open = [await opfsHandle('input.txt'), await opfsHandle('second.txt')]; })()");
    const result = await page.scenario("open_read", "multiple");
    expect(result.count, "2", "files"); expect(result.content, "first file|second file", "contents");
    expect((await page.calls())[0].multiple, true, "multiple");
});

check("a canceled picker gives no files and no error", async (page) => {
    await page.evaluate("pickerResult.cancel = true");
    expect((await page.scenario("open_read")).count, "0", "files after cancel");
    expect((await page.scenario("save_write", "report")).saved, "none", "saved file after cancel");
});

check("the save picker suggests the name with its extension and the written stream reaches the file", async (page) => {
    const result = await page.scenario("save_write", "report");
    expect(result.saved, "saved.txt", "saved file");
    const [call] = await page.calls();
    expect(call.kind, "save", "picker"); expect(call.suggestedName, "report.txt", "suggested name");
    await page.waitFor(`opfsText("saved.txt").then((text) => text === "Hello, storage")`, 10000);
});

// The size is read from the memory object, not from a view of the module: the object answers with
// the current buffer whichever thread grew the memory.
const MEMORY = "storageView.wasmMemory.buffer.byteLength";
check(threaded
    ? "a large file written through the fixed, shared memory of a threaded module reads back unchanged"
    : "a large file written while the module memory grows reads back unchanged", async (page) => {
    const length = 3000000;
    expect(await page.evaluate("storageView.wasmMemory instanceof WebAssembly.Memory"), true, "the module exports its memory");
    expect(await page.evaluate("Object.prototype.toString.call(storageView.wasmMemory.buffer)"),
        threaded ? "[object SharedArrayBuffer]" : "[object ArrayBuffer]", "the buffer of the memory");
    const memoryBefore = await page.evaluate(MEMORY);
    expect((await page.scenario("large", String(length), { timeout: 60000 })).written, String(length), "written");
    const memoryAfter = await page.evaluate(MEMORY);
    if (threaded) {
        // scripts/build-browser.sh links a threaded module with a memory that does not grow.
        expect(memoryAfter, memoryBefore, "size of the fixed memory after the write");
    } else {
        assert(memoryAfter > memoryBefore, `the module memory did not grow (${memoryBefore} bytes)`);
    }
    await page.evaluate("(async () => { pickerResult.open = [await opfsHandle('saved.txt')]; })()");
    const result = await page.scenario("read_large", String(length), { timeout: 60000 });
    expect(result.length, String(length), "length read back"); expect(result.equal, "true", "content read back");
    const size = await page.evaluate("opfsHandle('saved.txt').then((h) => h.getFile()).then((f) => f.size)");
    expect(size, length, "size of the file");
});

// A file handle over input.txt whose writable stream writes nothing and whose close fails
// (`close: "fail"`) or never settles (`close: "hang"`).
const FAULTY_HANDLE = (close) => `(async () => {
    const real = await opfsHandle("input.txt");
    const handle = {
        kind: "file", name: "faulty.txt",
        getFile: () => real.getFile(),
        isSameEntry: async (other) => other === handle,
        createWritable: async () => ({
            write: async () => {},
            close: () => ${JSON.stringify(close)} === "fail" ? Promise.reject(new Error("The disk is full")) : new Promise(() => {})
        })
    };
    pickerResult.open = [handle, real];
})()`;

check("a file written through the synchronous stream reads back its new content", async (page) => {
    expect((await page.scenario("write_then_read", "same")).content, "written", "content read after the write");
});

check("a failed close of a written file is reported by the next opening of that file", async (page) => {
    await page.evaluate(FAULTY_HANDLE("fail"));
    const result = await page.scenario("write_then_read", "same", { failing: true });
    assert((result.error ?? "").includes("The disk is full"), `expected the failed close, got ${JSON.stringify(result)}`);
});

check("a close that never settles does not keep other files from opening", async (page) => {
    await page.evaluate(FAULTY_HANDLE("hang"));
    expect((await page.scenario("write_then_read", "other")).content, "first file", "content of the other file");
});

check("a folder creates, lists, finds and deletes its items", async (page) => {
    const result = await page.scenario("folder");
    expect(result.folder, "work", "folder"); expect(result.items, "a.txt:file,sub:folder", "items");
    expect(result.a, "A", "content of a created file"); expect(result.missing, "true", "a missing file is not found");
    expect(result.mismatch, "true", "a file is not found as a folder"); expect(result.sub, "b.txt", "items of the sub folder");
    expect(result.after_delete, "sub", "items after the delete");
    expect(await page.evaluate("opfsText('work', 'sub', 'b.txt')"), "B", "nested file on disk");
});

check("an item moves to another folder", async (page) => {
    const result = await page.scenario("move");
    expect(result.moved, "moved.txt", "moved item"); expect(result.target, "moved.txt", "items of the target");
    expect(result.left, "true", "the item left its folder");
});

check("a bookmark survives a reload, opens only as its kind and is released", async (page) => {
    const saved = await page.scenario("bookmark_save");
    expect(saved.can_bookmark, "true", "can bookmark");
    assert(saved.bookmark.length > 0, "no bookmark");
    await page.evaluate("location.reload()");
    await page.waitForView(200);
    await page.waitFor("globalThis.storageView && storageView.storageViewResult");
    const opened = await page.scenario("bookmark_open", saved.bookmark);
    expect(opened.file, "input.txt", "bookmarked file"); expect(opened.content, "first file", "content of the bookmarked file");
    expect(opened.as_folder, "false", "a file bookmark opens as a folder");
    expect((await page.scenario("bookmark_open", "not a bookmark")).file, "none", "an invalid bookmark");
    expect((await page.scenario("bookmark_release", saved.bookmark)).reopened, "false", "the released bookmark reopened");
});

check("a well-known folder is a start location of the pickers", async (page) => {
    const result = await page.scenario("well_known");
    expect(result.name, "documents", "name"); expect(result.path_lookup_none, "true", "lookup by path");
    expect((await page.scenario("pick_folder_start_in_documents")).count, "1", "folders");
    expect((await page.calls()).pop().startIn, "documents", "start location");
});

check("a file dropped on the view is a storage file the application reads", async (page) => {
    const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-drop-")), "dropped.txt");
    fs.writeFileSync(file, "dropped content");
    const data = { items: [], files: [file], dragOperationsMask: 1 };
    await page.send("Input.dispatchDragEvent", { type: "dragEnter", x: 100, y: 100, data });
    await page.send("Input.dispatchDragEvent", { type: "dragOver", x: 100, y: 100, data });
    await page.send("Input.dispatchDragEvent", { type: "drop", x: 100, y: 100, data });
    await page.waitFor(`storageView.storageViewResult("drop") !== ""`, 30000);
    const line = await page.evaluate(`storageView.storageViewResult("drop")`);
    expect(line, "count=1;names=dropped.txt;content=dropped content", "dropped files");
});

check("with the polyfill preferred, the open picker reads a file chosen in a file input", async (page) => {
    const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-storage-")), "upload.txt");
    fs.writeFileSync(file, "from disk");
    await page.send("Page.setInterceptFileChooserDialog", { enabled: true });
    const chooser = new Promise((resolve) => {
        const poll = setInterval(async () => {
            const node = await page.evaluate(`(() => { const input = document.querySelector("input[type=file]"); return input ? input.multiple : null; })()`);
            if (node !== null) { clearInterval(poll); resolve(node); }
        }, 100);
    });
    const running = page.scenario("open_read", "", { userGesture: true });
    const multiple = await chooser;
    expect(multiple, false, "multiple selection of the file input");
    const { root: document } = await page.send("DOM.getDocument", {});
    const { nodeId } = await page.send("DOM.querySelector", { nodeId: document.nodeId, selector: "input[type=file]" });
    await page.send("DOM.setFileInputFiles", { files: [file], nodeId });
    const result = await running;
    expect(result.count, "1", "files"); expect(result.names, "upload.txt", "names"); expect(result.content, "from disk", "content");
}, { query: "?PreferPolyfill=true", stubs: false });

check("with the polyfill preferred, the save picker downloads the written file", async (page) => {
    const downloads = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-downloads-"));
    await page.send("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: downloads });
    expect((await page.scenario("save_write", "report")).saved, "report.txt", "saved file");
    const file = path.join(downloads, "report.txt");
    for (let i = 0; i < 100 && !(fs.existsSync(file) && fs.readFileSync(file, "utf8") === "Hello, storage"); i++) { await sleep(100); }
    expect(fs.existsSync(file) ? fs.readFileSync(file, "utf8") : null, "Hello, storage", "downloaded content");
}, { query: "?PreferPolyfill=true", stubs: false });

check("without the service worker none is registered, and the polyfill saves through a blob link", async (page) => {
    expect(await page.evaluate("navigator.serviceWorker.getRegistration().then((r) => r === undefined)"), true, "no registration");
    // The polyfill clicks a link that is not in the document: the clicks of links are recorded.
    await page.evaluate(`(globalThis.links = [], ((click) => { HTMLAnchorElement.prototype.click = function () { links.push(this.href); return click.call(this); }; })(HTMLAnchorElement.prototype.click), true)`);
    const downloads = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-downloads-"));
    await page.send("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: downloads });
    expect((await page.scenario("save_write", "report")).saved, "report.txt", "saved file");
    await page.waitFor("links.length > 0", 10000);
    const links = JSON.parse(await page.evaluate("JSON.stringify(links)"));
    assert(links.every((link) => link.startsWith("blob:")), `the download went through ${JSON.stringify(links)}`);
}, { query: "?PreferPolyfill=true", stubs: false });

// The worker of the platform is registered for the directory of the site, and a file saved through
// the polyfill reaches the download as a response the worker streams.
async function expectStreamedSave(page) {
    const registration = JSON.parse(await page.evaluate(`navigator.serviceWorker.ready.then((r) => JSON.stringify({
        scope: r.scope, script: r.active && r.active.scriptURL, state: r.active && r.active.state }))`));
    const origin = await page.evaluate("location.origin");
    expect(registration.scope, `${origin}/`, "scope");
    expect(registration.script, `${origin}/${SERVICE_WORKER}`, "script");
    expect(registration.state, "activated", "state");
    expect(await page.evaluate("navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)"), 1, "registrations");

    const downloads = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-downloads-"));
    await page.send("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: downloads });
    expect((await page.scenario("save_write", "report")).saved, "report.txt", "saved file");
    const file = path.join(downloads, "report.txt");
    for (let i = 0; i < 100 && !(fs.existsSync(file) && fs.readFileSync(file, "utf8") === "Hello, storage"); i++) { await sleep(100); }
    expect(fs.existsSync(file) ? fs.readFileSync(file, "utf8") : null, "Hello, storage", "downloaded content");
    // The polyfill took the path of the worker: it navigates a hidden frame to an address in the scope of the
    // worker, which the site does not have (the server answers 404): the worker answered with the stream.
    expect(await page.evaluate(`Array.from(document.querySelectorAll("iframe[hidden]")).map((f) => f.src).join()`), `${origin}/report.txt`, "download frame");
    expect(await page.evaluate(`fetch("/report.txt").then((r) => r.status)`), 404, "status of the address on the server");
}

check("the service worker is registered at the root of the site and streams the polyfill's download", async (page) => {
    await expectStreamedSave(page);
}, { query: "?PreferPolyfill=true&RegisterServiceWorker=true", stubs: false });

// A threaded site on a host that cannot send the headers (a static host): the host page registers
// the service worker to isolate itself and reloads once, and the application registers the same
// address. One worker has to do both jobs, or the second registration would take the isolation,
// and with it the shared memory of the module, away.
if (threaded) {
    check("without the headers one service worker isolates the page and streams the polyfill's download", async (page) => {
        expect(await page.evaluate("self.crossOriginIsolated"), true, "the page is cross-origin isolated");
        expect(page.navigations.length, 2, `loads of the page (${page.navigations.join(", ")})`);
        expect(await page.evaluate("navigator.serviceWorker.controller && navigator.serviceWorker.controller.scriptURL"),
            `${await page.evaluate("location.origin")}/${SERVICE_WORKER}`, "the worker that controls the page");
        await expectStreamedSave(page);
        // The registration of the application did not replace the worker that isolates the page:
        // a new load is still isolated, and the module starts again.
        await page.evaluate("location.reload()");
        await page.waitForView(200);
        await page.waitFor("globalThis.storageView && storageView.storageViewResult");
        expect(await page.evaluate("self.crossOriginIsolated"), true, "the page is cross-origin isolated after another load");
        expect(await page.evaluate("navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)"), 1, "registrations after another load");
    }, { query: "?PreferPolyfill=true&RegisterServiceWorker=true", stubs: false, isolated: false });
}

await run(checks);
