// Tests of a site that carries both modules, the one built without threads and the one built with
// them, in headless Chrome: which module its host page loads, and that a page load never requests
// both (scripts/browser/threads/ferroui-loader.js; docs/porting/browser-render-worker.md, "B2.8").
//
//   scripts/browser/setup.sh --threads && source .tools/env.sh
//   scripts/build-browser.sh control-catalog-browser --both
//   node scripts/browser/tests/site_loader.test.mjs [<site directory>]     default: target/browser-both/control-catalog-browser
//
// The site is one `scripts/browser/combine-site.mjs` composed, of the ControlCatalog or of the
// themed_view example (`scripts/build-browser.sh themed_view --both`). The checks:
//
// - the layout: the module with threads in its own directory, with the one line that re-exports the
//   script module of the platform; the host page preloads neither module and says the site has both;
// - served with the headers of cross-origin isolation, the page loads the module with threads, the
//   frames are drawn by a render thread, the page is loaded once and no service worker is registered;
// - served without them, the page registers the service worker, is reloaded once and does the same;
//   a second visit is isolated at once, without a reload;
// - `?Threads=false`: the module without threads, drawn by the thread of the page, with the headers
//   and without them, where no service worker is registered and the page is not reloaded;
// - a browser that cannot transfer a canvas to a worker (the function is removed before the page
//   runs), and one whose workers do not answer the probe: the module without threads, no service
//   worker, no reload; the answer of the probe is kept for the session;
// - `?Threads=true` skips what the browser is asked: the module with threads although the probe fails;
// - a window without service workers and without the headers: the module without threads, and no
//   message in the page;
// - a page that is still not isolated after the one reload: no second reload, the module without
//   threads, recorded for the session.
//
// Every check asserts on the requests of the page, over all its loads: the script and the
// WebAssembly file of the module it expects are requested, the WebAssembly file once and through the
// preload the loader adds, and no file of the other module is; and nothing is logged as an error.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, sleep } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = path.resolve(process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-both", "control-catalog-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser --both`);
    process.exit(2);
}

// What the host page says of the site (written by combine-site.mjs, read by the loader).
const hostPage = fs.readFileSync(path.join(site, "index.html"), "utf8");
const marker = /<meta name="ferroui-modules" content='([^']*)'>/.exec(hostPage);
if (marker === null) {
    console.error(`${site} is not a site with both modules (its host page has no ferroui-modules element): compose it with scripts/build-browser.sh <application> --both`);
    process.exit(2);
}
const modules = JSON.parse(marker[1]);
const threadsDirectory = modules.threads.replace(/^\.\//, "");
const script = fs.readdirSync(site).find((name) => name.endsWith(".js") && fs.existsSync(path.join(site, threadsDirectory, name)) && name !== "ferroui.js");

// The applications a site can be of: the global the host page keeps the module in, and the export
// that reports where the frames are drawn (a line of `name=value` pairs separated by `;`).
const APPLICATIONS = {
    "control_catalog_browser.wasm": { global: "controlCatalog", rendering: "controlCatalog.catalogRendering()" },
    "themed_view.wasm": { global: "themedView", rendering: "themedView.themedViewRendering()" }
};
const application = APPLICATIONS[modules.wasm];
if (!application || !script) {
    console.error(`${site}: the test knows the sites of ${Object.keys(APPLICATIONS).join(" and ")}, not of ${modules.wasm}`);
    process.exit(2);
}

// Start-up includes compiling the WebAssembly module, which takes a while on a cold CI runner.
const START_TIMEOUT = 180_000;
const SERVICE_WORKERS = "navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)";
const pairs = (line) => Object.fromEntries(line.split(";").map((pair) => { const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)]; }));

// Removes what a browser that cannot transfer a canvas does not have.
const NO_CANVAS_TRANSFER = "delete HTMLCanvasElement.prototype.transferControlToOffscreen;";
// Makes the probe worker of the loader fail and leaves the workers of the module alone: the probe
// is the only worker made from a blob.
const NO_PROBE_ANSWER = `(() => {
    const Original = globalThis.Worker;
    globalThis.Worker = function (address, options) {
        if (String(address).startsWith("blob:")) { throw new Error("no worker for the probe"); }
        return new Original(address, options);
    };
})();`;
const NO_SERVICE_WORKERS = "delete Navigator.prototype.serviceWorker;";
// A page that never becomes isolated, whatever the headers of its responses say.
const NEVER_ISOLATED = "Object.defineProperty(globalThis, 'crossOriginIsolated', { get: () => false, configurable: true });";

/** Waits until the application has started and a frame was drawn; resolves to where the frames are drawn. */
async function started(page) {
    try {
        await page.waitFor(`globalThis.${application.global} && Number(/frames=(\\d+)/.exec(${application.rendering})[1]) > 0`, START_TIMEOUT);
    } catch (error) {
        // What the page is when the application did not draw: which document this is, whether it
        // was restored or reloaded, what the loader decided and recorded, and what the view reports.
        const state = await page.evaluate(`JSON.stringify({
            visibility: document.visibilityState, isolated: self.crossOriginIsolated,
            navigation: performance.getEntriesByType("navigation").map((n) => n.type).join(","),
            timeOrigin: Math.round(performance.timeOrigin), now: Math.round(performance.now()),
            module: globalThis.ferrouiModule ? { threads: ferrouiModule.threads, reason: ferrouiModule.reason } : null,
            application: typeof globalThis.${application.global},
            rendering: (() => { try { return ${application.rendering}; } catch (e) { return "error: " + e.message; } })(),
            canvases: document.querySelectorAll("canvas").length,
            canvas: (() => { const c = document.querySelector("canvas"); return c ? c.width + "x" + c.height : null; })(),
            splash: !!document.querySelector(".ferroui-splash"),
            controller: navigator.serviceWorker && navigator.serviceWorker.controller ? navigator.serviceWorker.controller.scriptURL : null,
            session: Object.fromEntries(Object.keys(sessionStorage).map((k) => [k, sessionStorage.getItem(k)])),
            scripts: performance.getEntriesByType("resource").filter((e) => /\\.(js|wasm)$/.test(new URL(e.name).pathname)).map((e) => new URL(e.name).pathname + ":" + e.responseStatus) })`)
            .catch((failure) => `the page did not answer: ${failure.message}`);
        throw new Error(`${error.message.split("\n")[0]}\n      the page: ${state}\n      navigations: ${page.navigations.join(", ")}\n      errors: ${page.errors.join(" | ")}\n${error.message.split("\n").slice(1).join("\n")}`);
    }
    return pairs(await page.evaluate(application.rendering));
}

/** The requests of the page, over all its loads, for files of the site (paths relative to it). */
const requested = (page) => page.requests.filter((address) => address.startsWith(new URL(page.url).origin))
    .map((address) => decodeURIComponent(new URL(address).pathname.slice(1)));

/** Asserts that the page loaded the module with (`threads`) or without threads, and nothing of the other one. */
async function expectModule(page, threads) {
    const rendering = await started(page);
    const loaded = await page.evaluate("({ threads: ferrouiModule.threads, reason: ferrouiModule.reason, site: ferrouiModule.site })");
    assert(loaded.site === "both", `the loader took the site for one with ${loaded.site} module`);
    assert(loaded.threads === threads, `the loader chose the module ${loaded.threads ? "with" : "without"} threads: ${loaded.reason}`);
    const chosen = threads ? threadsDirectory : "";
    const other = threads ? "" : threadsDirectory;
    const files = requested(page);
    for (const name of [script, modules.wasm]) {
        assert(files.includes(chosen + name), `${chosen + name} was not requested; requested: ${files.join(", ")}`);
        assert(!files.includes(other + name), `${other + name}, a file of the module that was not chosen, was requested`);
    }
    if (!threads) { assert(!files.some((name) => name.startsWith(threadsDirectory)), `files of ${threadsDirectory} were requested: ${files.filter((name) => name.startsWith(threadsDirectory)).join(", ")}`); }
    // A preload that does not match the request of the script of the module would download the file twice.
    const downloads = JSON.parse(await page.evaluate(`JSON.stringify(performance.getEntriesByType("resource")
        .filter((e) => new URL(e.name).pathname.endsWith(${JSON.stringify("/" + chosen + modules.wasm)})).map((e) => e.initiatorType))`));
    assert(downloads.length === 1 && downloads[0] === "link", `${chosen + modules.wasm} was downloaded ${downloads.length} times (${downloads.join(", ")}), expected once by the preload of the loader`);
    if (threads) {
        assert(await page.evaluate("self.crossOriginIsolated") === true, "the page is not cross-origin isolated");
        assert(rendering.on_render_thread === "true" && rendering.other_thread === "true" && rendering.frame_thread === rendering.render_thread
            && rendering.frame_thread !== rendering.page_thread, `the frames are not drawn by a render thread: ${JSON.stringify(rendering)}`);
    } else {
        assert(rendering.on_render_thread === "false" && rendering.other_thread === "false" && rendering.render_thread === "0",
            `the frames are not drawn by the thread of the page: ${JSON.stringify(rendering)}`);
    }
    assert(await page.evaluate("document.getElementById('ferroui-threads-message')") === null, "the page shows the message of a page that cannot be isolated");
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
    return loaded;
}

const loads = (page, expected) => assert(page.navigations.length === expected,
    `the page was loaded ${page.navigations.length} times, expected ${expected}: ${page.navigations.join(", ")}`);

const checks = [];
/** Adds a check that runs `body` on a page of the site; the end of the log of the page is added to a failure. */
const check = (name, body, options = {}) => checks.push([name, async () => {
    const page = await open(site, { width: 800, height: 600, network: true, ...options });
    try { await body(page); } catch (error) {
        error.message += `\n${page.log.slice(-8).join("\n")}`;
        throw error;
    } finally { await page.close(); }
}]);

checks.push(["the site has the module with threads in its own directory and its page preloads neither module", async () => {
    for (const name of [script, modules.wasm]) {
        assert(fs.existsSync(path.join(site, name)), `the site has no ${name}`);
        assert(fs.existsSync(path.join(site, threadsDirectory, name)), `the site has no ${threadsDirectory}${name}`);
        assert(!new RegExp(`<link\\b[^>]*href=["'](?:\\./)?${name.replace(/[.]/g, "\\.")}["']`).test(hostPage), `the host page has a preload element for ${name}`);
    }
    assert(fs.readFileSync(path.join(site, threadsDirectory, script), "utf8").includes("ferrouiThreads"), `${threadsDirectory}${script} was not built with threads`);
    assert(!fs.readFileSync(path.join(site, script), "utf8").includes("ferrouiThreads"), `${script} was built with threads`);
    assert(/export \* from "\.\.\/ferroui\.js";/.test(fs.readFileSync(path.join(site, threadsDirectory, "ferroui.js"), "utf8")),
        `${threadsDirectory}ferroui.js does not re-export the script module of the site`);
    for (const name of ["ferroui.js", "ferroui-loader.js", "ferroui-sw.js", "storage.js"]) { assert(fs.existsSync(path.join(site, name)), `the site has no ${name}`); }
    assert(!fs.existsSync(path.join(site, "ferroui-threads.js")), "the site has ferroui-threads.js, the check of a site with the module with threads alone");
}]);

check("isolated by the headers of the server, the page loads the module with threads and a render thread draws", async (page) => {
    await expectModule(page, true);
    loads(page, 1);
    assert(await page.evaluate(SERVICE_WORKERS) === 0, "a service worker was registered although the server sends the headers");
}, { isolated: true });

check("without the headers the service worker isolates the page after one reload, and a second visit does not reload", async (page) => {
    await expectModule(page, true);
    loads(page, 2);
    assert(await page.evaluate("navigator.serviceWorker.controller?.scriptURL.endsWith('/ferroui-sw.js?coi=1')") === true, "the page is not controlled by ferroui-sw.js?coi=1");
    assert(await page.evaluate(SERVICE_WORKERS) === 1, "the site does not have one service worker");
    // The second visit: the same session, the worker in place. (Chrome can answer this reload with
    // the first document of the page, kept by its back/forward cache, unless the loader keeps that
    // document out of it: "B2.8" of the document, "The reload that brought back the first document".)
    await page.evaluate(`globalThis.${application.global} = undefined`);
    await page.send("Page.reload");
    await sleep(500);
    await expectModule(page, true);
    loads(page, 3);
    assert(await page.evaluate(SERVICE_WORKERS) === 1, "the site does not have one service worker after the second visit");
});

check("Threads=false loads the module without threads in an isolated page", async (page) => {
    await expectModule(page, false);
    loads(page, 1);
}, { isolated: true, query: "?Threads=false" });

check("Threads=false without the headers registers no service worker and does not reload", async (page) => {
    await expectModule(page, false);
    loads(page, 1);
    assert(await page.evaluate(SERVICE_WORKERS) === 0, "a service worker was registered for the module without threads");
}, { query: "?Threads=false" });

check("a browser that cannot transfer a canvas gets the module without threads, with no service worker and no reload", async (page) => {
    const loaded = await expectModule(page, false);
    assert(/transfer a canvas/.test(loaded.reason), `the reason is: ${loaded.reason}`);
    loads(page, 1);
    assert(await page.evaluate(SERVICE_WORKERS) === 0, "a service worker was registered although the browser cannot run the module with threads");
}, { initScript: NO_CANVAS_TRANSFER });

check("a browser whose worker does not answer the probe gets the module without threads, and the answer is kept for the session", async (page) => {
    await expectModule(page, false);
    loads(page, 1);
    assert(await page.evaluate("sessionStorage.getItem('ferroui-loader-worker-frames')") === "0", "the answer of the probe was not kept");
    assert(await page.evaluate(SERVICE_WORKERS) === 0, "a service worker was registered although the probe failed");
}, { isolated: true, initScript: NO_PROBE_ANSWER });

check("Threads=true loads the module with threads without asking the browser", async (page) => {
    const loaded = await expectModule(page, true);
    assert(/Threads=true/.test(loaded.reason), `the reason is: ${loaded.reason}`);
    assert(await page.evaluate("sessionStorage.getItem('ferroui-loader-worker-frames')") === null, "the probe ran although the module was forced");
    loads(page, 1);
}, { isolated: true, query: "?Threads=true", initScript: NO_PROBE_ANSWER });

check("a window without service workers and without the headers gets the module without threads, silently", async (page) => {
    const loaded = await expectModule(page, false);
    assert(/service workers/.test(loaded.reason), `the reason is: ${loaded.reason}`);
    loads(page, 1);
}, { initScript: NO_SERVICE_WORKERS });

check("a page that is still not isolated after the reload is not reloaded again and gets the module without threads", async (page) => {
    const loaded = await expectModule(page, false);
    assert(/after the reload/.test(loaded.reason), `the reason is: ${loaded.reason}`);
    loads(page, 2);
    assert(await page.evaluate("sessionStorage.getItem('ferroui-loader-not-isolated')") === "1", "the failure was not recorded for the session");
    assert(await page.evaluate("sessionStorage.getItem('ferroui-loader-reloaded')") === null, "the count of the reload was left behind");
    // Nothing is tried again in the session.
    await page.evaluate(`globalThis.${application.global} = undefined`);
    await page.send("Page.reload");
    await sleep(500);
    const again = await expectModule(page, false);
    assert(/earlier in this session/.test(again.reason), `the reason of the next load is: ${again.reason}`);
    loads(page, 3);
}, { initScript: NEVER_ISOLATED });

await run(checks);
