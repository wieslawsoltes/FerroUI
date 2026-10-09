// Loads the script of the module for a host page, and on a site that carries both modules decides
// which one, before anything of either is requested.
//
//   import { loadModule } from "./ferroui-loader.js";
//   const loaded = await loadModule({ script: "./<application>.js", element: document.getElementById("out") });
//   if (loaded) { const runtime = await loaded.createRuntime(); ... }
//
// `scripts/build-browser.sh` copies this file next to every host page. It has no imports and uses
// nothing of the platform. What it does depends on the site:
//
// - A site with one module (`scripts/build-browser.sh <application>`, with or without `--threads`)
//   has no `ferroui-modules` element in its page. The script of the module is imported as the page
//   named it. When that script says it was built with threads (its export `ferrouiThreads`), the
//   page has to be cross-origin isolated before the module is created, and there is no other module
//   to fall back to: the check of ferroui-threads.js runs, which isolates the page through the
//   service worker or shows a message, and `loadModule` then resolves to null.
// - A site with both modules (`scripts/build-browser.sh <application> --both`, composed by
//   scripts/browser/combine-site.mjs) has the module without threads where a site with one module
//   has it and the module with threads in a directory of its own, and its page says so:
//
//     <meta name="ferroui-modules" content='{"threads":"./threads/","wasm":"<name>.wasm"}'>
//
//   The module with threads is chosen when the browser can run it and the page is, or can be made,
//   cross-origin isolated; the module without threads otherwise, without a message: see `choose`.
//   Only then is the WebAssembly file of the chosen module preloaded and its script imported, so
//   that a page load never downloads both.
//
// `?Threads=false` forces the module without threads and `?Threads=true` the one with threads
// (which still needs an isolated page), for tests. `?RenderThread=false` is another thing: it is
// read by the application and keeps a module built with threads on the thread of the page.
//
// What was decided is kept in `globalThis.ferrouiModule` (`threads`, `reason`, `site`), for the
// tests and for inspection from the console. See docs/porting/browser-render-worker.md, "B2.8".

// The service worker of the platform with the part that adds the headers of cross-origin
// isolation: the address a module built with threads registers itself (ferroui-threads.js,
// `resolve_service_worker_path`), so that the registrations are one worker.
const SERVICE_WORKER = "./ferroui-sw.js?coi=1";

// Keys of the session storage.
// Set for the one reload that follows the registration of the service worker, so that a page that
// is still not isolated after it loads the module without threads instead of reloading for ever.
const RELOADED = "ferroui-loader-reloaded";
// The page could not be isolated: nothing is tried again in this session.
const NOT_ISOLATED = "ferroui-loader-not-isolated";
// The answer of the probe worker ("1" or "0"), so that the probe runs once in a session.
const WORKER_FRAMES = "ferroui-loader-worker-frames";
// The module with threads was chosen and could not be created.
const THREADS_FAILED = "ferroui-loader-threads-failed";

// The lock a document holds from the moment it asks for the reload: see `reload`.
const RELOAD_LOCK = "ferroui-loader-reload";

// How long the probe worker may take to answer, and the service worker to become active.
const PROBE_TIMEOUT = 2000;
const SERVICE_WORKER_TIMEOUT = 10000;

// What a dedicated worker has to have: animation frames (the render thread draws from them) and
// the canvas it is handed. The page cannot see either from its own thread.
const PROBE = "postMessage(typeof requestAnimationFrame === \"function\" && typeof OffscreenCanvas === \"function\");";

// Session storage is not available in every mode of every browser.
function session(action) {
    try { return action(globalThis.sessionStorage); } catch { return null; }
}
const read = (key) => session((storage) => storage.getItem(key));
const write = (key, value) => session((storage) => { storage.setItem(key, value); return true; }) === true;
const forget = (key) => { session((storage) => storage.removeItem(key)); };

const resolve = (address) => new URL(address, globalThis.document.baseURI).href;

// What the page says of a site with both modules, or null for a site with one.
function siteModules() {
    const content = globalThis.document.querySelector("meta[name=\"ferroui-modules\"]")?.getAttribute("content");
    if (!content) { return null; }
    try {
        const site = JSON.parse(content);
        return typeof site?.threads === "string" && typeof site.wasm === "string" ? site : null;
    } catch { return null; }
}

// `?Threads=true|false` of the address of the page: true, false, or null without the parameter.
function forced() {
    const match = /(?:^|[?&])threads=(true|false)(?:&|$)/i.exec(globalThis.location.search);
    return match === null ? null : match[1].toLowerCase() === "true";
}

// A shared memory can be made. The constructor `SharedArrayBuffer` is hidden in a page that is not
// isolated yet, which is the page that asks; a shared memory of WebAssembly can be made there all
// the same when the browser has shared memory at all.
function hasSharedMemory() {
    if (typeof SharedArrayBuffer === "function") { return true; }
    try {
        const buffer = new WebAssembly.Memory({ initial: 1, maximum: 1, shared: true }).buffer;
        return Object.prototype.toString.call(buffer) === "[object SharedArrayBuffer]";
    } catch { return false; }
}

// What the thread of the page can see is missing for a module with threads, or null.
function missingFeature() {
    if (typeof WebAssembly !== "object" || typeof Worker !== "function") { return "the browser has no WebAssembly or no workers"; }
    if (typeof Atomics !== "object") { return "the browser has no Atomics"; }
    if (typeof HTMLCanvasElement !== "function" || typeof HTMLCanvasElement.prototype.transferControlToOffscreen !== "function") {
        return "the browser cannot transfer a canvas to a worker";
    }
    if (!hasSharedMemory()) { return "the browser has no shared memory"; }
    return null;
}

// Whether a dedicated worker that is a script module (what a thread of the module is) has
// animation frames and `OffscreenCanvas`. Asked of a worker once in a session; a worker that does
// not start, does not answer in time or cannot be a script module answers no.
function workerHasFrames() {
    const cached = read(WORKER_FRAMES);
    if (cached === "1" || cached === "0") { return Promise.resolve(cached === "1"); }
    return new Promise((resolveAnswer) => {
        let worker; let address; let timer; let answered = false;
        const answer = (value) => {
            if (answered) { return; }
            answered = true;
            clearTimeout(timer);
            try { worker?.terminate(); } catch { }
            if (address) { URL.revokeObjectURL(address); }
            write(WORKER_FRAMES, value ? "1" : "0");
            resolveAnswer(value);
        };
        try {
            address = URL.createObjectURL(new Blob([PROBE], { type: "text/javascript" }));
            // A browser without workers that are script modules never reads the option.
            let modules = false;
            worker = new Worker(address, { get type() { modules = true; return "module"; } });
            if (!modules) { answer(false); return; }
            worker.onmessage = (event) => answer(event.data === true);
            worker.onerror = () => answer(false);
            timer = setTimeout(() => answer(false), PROBE_TIMEOUT);
        } catch { answer(false); }
    });
}

// Makes the page cross-origin isolated. Resolves to null when it is; to the reason when it cannot
// be; and does not resolve when the service worker was registered and the page is being reloaded,
// which happens once: the reload is counted as ferroui-threads.js counts it, and a page that is
// still not isolated after it is recorded for the session.
async function isolate() {
    if (globalThis.crossOriginIsolated) {
        forget(RELOADED);
        forget(NOT_ISOLATED);
        return null;
    }
    if (read(NOT_ISOLATED) !== null) { return "the page could not be isolated earlier in this session"; }
    if (!globalThis.isSecureContext) { return "the page is not a secure context, so it has no service worker to isolate it"; }
    if (!("serviceWorker" in globalThis.navigator)) { return "this window has no service workers to isolate the page"; }
    if (read(RELOADED) !== null) {
        forget(RELOADED);
        write(NOT_ISOLATED, "1");
        return "the page is still not isolated after the reload that follows the registration of the service worker";
    }
    try {
        await globalThis.navigator.serviceWorker.register(SERVICE_WORKER);
        // Resolves once a worker of the scope is active; the next load of the page goes through it.
        let timer;
        const active = await Promise.race([
            globalThis.navigator.serviceWorker.ready.then(() => true),
            new Promise((expired) => { timer = setTimeout(() => expired(false), SERVICE_WORKER_TIMEOUT); })
        ]);
        clearTimeout(timer);
        if (!active) {
            write(NOT_ISOLATED, "1");
            return "the service worker that isolates the page did not become active";
        }
    } catch (error) {
        write(NOT_ISOLATED, "1");
        return `the service worker that isolates the page could not be registered (${error?.message ?? error})`;
    }
    // Without session storage the reload cannot be counted and could repeat for ever.
    if (!write(RELOADED, "1")) { return "the reload that isolates the page cannot be counted without session storage"; }
    return reload();
}

// Reloads the page for the service worker and never resolves. The document that asks is finished:
// it has no module and never will. The page that replaces it is isolated and this one is not, so
// the browser puts the two in different groups of contexts, and that makes this document one the
// back/forward cache may keep; Chrome has been seen to bring it back, frozen splash and all, as the
// answer to a later reload of the page. Two things against that, as ferroui-threads.js does: a lock
// that is never released, which keeps a document out of that cache (shared, so that two pages of
// the site do not wait for each other); and, where a browser keeps it all the same, a document that
// is shown again from the cache reloads, this time through the service worker.
function reload() {
    try { globalThis.navigator.locks?.request(RELOAD_LOCK, { mode: "shared" }, () => new Promise(() => { }))?.catch(() => { }); } catch { }
    globalThis.addEventListener("pageshow", (event) => { if (event.persisted) { globalThis.location.reload(); } });
    globalThis.location.reload();
    return new Promise(() => { });
}

// Decides for a site with both modules: `{ threads, reason }`. In this order, so that a browser
// that cannot run the module with threads registers no service worker and is never reloaded:
// the parameter of the address; an earlier failure of the session; what the thread of the page can
// see; what only a worker can see; and last the isolation, which may reload the page once.
async function choose() {
    const force = forced();
    if (force === false) { return { threads: false, reason: "Threads=false in the address of the page" }; }
    if (force === null) {
        if (read(THREADS_FAILED) !== null) { return { threads: false, reason: "the module with threads could not be created earlier in this session" }; }
        const missing = missingFeature();
        if (missing !== null) { return { threads: false, reason: missing }; }
        if (!globalThis.crossOriginIsolated && read(NOT_ISOLATED) !== null) {
            return { threads: false, reason: "the page could not be isolated earlier in this session" };
        }
        if (!await workerHasFrames()) { return { threads: false, reason: "a worker of the browser has no animation frames or no canvas, or did not answer" }; }
    }
    const notIsolated = await isolate();
    if (notIsolated !== null) { return { threads: false, reason: notIsolated }; }
    return { threads: true, reason: force === true ? "Threads=true in the address of the page" : "the page is cross-origin isolated and the browser can render from a worker" };
}

// Starts the download of the WebAssembly file of the chosen module, as the element of a page with
// one module does: the script of the module fetches it with credentials "same-origin", which
// `crossorigin` matches, so the download is used and not repeated.
function preload(address) {
    const link = globalThis.document.createElement("link");
    link.rel = "preload";
    link.as = "fetch";
    link.type = "application/wasm";
    link.crossOrigin = "anonymous";
    link.href = address;
    globalThis.document.head.appendChild(link);
}

function record(loaded) {
    globalThis.ferrouiModule = loaded;
    return loaded;
}

// Imports the script of the module of the page and resolves to `{ createRuntime, threads, reason,
// site }`: `createRuntime` is the factory of the module (the default export of its script),
// `threads` whether the module was built with threads, `site` "one" or "both". Resolves to null
// when a site whose only module needs threads could not be isolated; the message is then in
// `element` (default: the body). In a site with both modules it may reload the page once instead
// of resolving.
//
// `script` is the address of the script of the module without threads, relative to the page.
export async function loadModule({ script, element } = {}) {
    const site = siteModules();
    if (site === null) {
        const moduleScript = await import(resolve(script));
        if (moduleScript.ferrouiThreads !== true) {
            return record({ createRuntime: moduleScript.default, threads: false, reason: "the site has one module, built without threads", site: "one" });
        }
        const { ensureCrossOriginIsolated } = await import(resolve("./ferroui-threads.js"));
        if (!await ensureCrossOriginIsolated({ element })) { return null; }
        return record({ createRuntime: moduleScript.default, threads: true, reason: "the site has one module, built with threads", site: "one" });
    }

    const name = script.split("/").pop();
    const plain = async () => {
        preload(resolve(`./${site.wasm}`));
        return (await import(resolve(`./${name}`))).default;
    };
    const { threads, reason } = await choose();
    if (!threads) {
        return record({ createRuntime: await plain(), threads: false, reason, site: "both" });
    }
    preload(resolve(site.threads + site.wasm));
    const threaded = (await import(resolve(site.threads + name))).default;
    const loaded = record({ createRuntime: undefined, threads: true, reason, site: "both" });
    // A module with threads that cannot be created after all (its memory, which has its whole size
    // from the start, is refused, for one) is replaced by the other one: this is the only case in
    // which a page load requests both.
    loaded.createRuntime = async (...options) => {
        try {
            return await threaded(...options);
        } catch (error) {
            console.warn(`FerroUI: the module with threads could not be created (${error?.message ?? error}); loading the module without threads.`);
            write(THREADS_FAILED, "1");
            loaded.threads = false;
            loaded.reason = "the module with threads could not be created";
            return (await plain())(...options);
        }
    };
    return loaded;
}
