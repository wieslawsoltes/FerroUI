// The asset files of the pages of the catalog (not in the managed original, whose runtime downloads every
// resource before Main).
//
// The build script of the sample copies each picture and font of the catalog to the site as it is, under
// assets/ControlCatalog/ and the path of the asset, and lists them in assets/ControlCatalog.json: the
// start-up files, which main.js registers before the application starts, and the files of each page (by
// the header of its entry in the page list). Before the catalog creates a page it calls ensurePageAssets
// (through the module, page_assets_browser.rs) and waits until the files of the page are fetched and
// registered. After the first frame the others are fetched in the background while the page is idle, so
// that most navigations find their files there already.
//
// For the behaviour tests and for inspection, globalThis.controlCatalogAssets.requests lists every file
// requested (its asset path), why ("start-up", "page" or "prefetch") and when (milliseconds from the start
// of navigation).

/** The list of the asset files, next to the directory of the files of the assembly. */
const MANIFEST = "./assets/ControlCatalog.json";

/** How many files the background prefetch fetches at the same time. */
const PREFETCH_CONNECTIONS = 4;

const requests = [];
globalThis.controlCatalogAssets = { requests };

let runtime = null;
let manifest = null;
// Asset path -> { loaded, promise }.
const files = new Map();

/** Fetches the list of the asset files. */
export function fetchManifest() {
    return fetch(MANIFEST).then((response) => {
        if (!response.ok) throw new Error(`${MANIFEST}: ${response.status} ${response.statusText}`);
        return response.json();
    });
}

/** Fetches the file of the asset with the rooted path `asset` (`/Assets/x.png`) as bytes. */
export function fetchFile(list, asset, reason) {
    requests.push({ name: asset, reason, start: performance.now() });
    const url = `./${list.directory}${asset.split("/").map(encodeURIComponent).join("/")}`;
    return fetch(url).then((response) => {
        if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
        return response.arrayBuffer();
    });
}

/** Registers the content of the asset `asset` with the asset loader of the module, under its URI. */
export function register(asset, content) {
    // Deviation (browser-platform.md section 14): assets registered one by one, after the start too.
    runtime.registerAsset(`ferres://${manifest.assembly}${asset}`, new Uint8Array(content));
    let entry = files.get(asset);
    if (!entry) files.set(asset, entry = { promise: Promise.resolve() });
    entry.loaded = true;
}

/** Starts to serve the files of `manifest_` to the module `runtime_`. */
export function attach(runtime_, manifest_) {
    runtime = runtime_;
    manifest = manifest_;
}

function load(asset, reason) {
    let entry = files.get(asset);
    if (!entry) {
        entry = { loaded: false };
        entry.promise = fetchFile(manifest, asset, reason).then((content) => register(asset, content), (error) => {
            // A later navigation to a page that needs the file tries again.
            files.delete(asset);
            throw error;
        });
        files.set(asset, entry);
    }
    return entry;
}

/**
 * Called by the module before the catalog creates the page with the header `page`: null when the files of
 * the page are registered, otherwise a promise that settles once they are (or that rejects when one cannot
 * be fetched).
 */
export function ensurePageAssets(page) {
    const pending = (manifest?.pages[page] ?? []).map((asset) => load(asset, "page")).filter((entry) => !entry.loaded);
    if (pending.length === 0) return null;
    return Promise.all(pending.map((entry) => entry.promise)).then(() => null);
}

/** Resolves once the first frame of the view is on screen: when the splash screen of the host page closes. */
function firstFrame() {
    return new Promise((resolve) => {
        const splash = document.querySelector("#out .ferroui-splash");
        if (!splash) {
            requestAnimationFrame(() => requestAnimationFrame(resolve));
            return;
        }
        const closed = () => splash.classList.contains("splash-close");
        if (closed()) { resolve(); return; }
        const observer = new MutationObserver(() => { if (closed()) { observer.disconnect(); resolve(); } });
        observer.observe(splash, { attributes: true, attributeFilter: ["class"] });
    });
}

function idle() {
    return new Promise((resolve) => globalThis.requestIdleCallback
        ? requestIdleCallback(() => resolve(), { timeout: 2000 })
        : setTimeout(resolve, 50));
}

/**
 * After the first frame, fetches the files no page has asked for yet, in the order of the manifest, a few
 * at a time, each when the page is idle.
 */
export async function prefetchAfterFirstFrame() {
    await firstFrame();
    performance.mark("first frame");
    const queue = [...manifest.prefetch];
    const connection = async () => {
        for (let asset = queue.shift(); asset !== undefined; asset = queue.shift()) {
            if (files.has(asset)) continue;
            await idle();
            try {
                await load(asset, "prefetch").promise;
            } catch (error) {
                console.warn(`prefetching ${asset} failed: ${error.message}`);
            }
        }
    };
    await Promise.all(Array.from({ length: PREFETCH_CONNECTIONS }, connection));
    performance.mark("assets prefetched");
}
