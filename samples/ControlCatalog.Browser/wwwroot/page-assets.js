// The asset bundles of the pages of the catalog (not in the managed original, whose runtime downloads
// every resource before Main).
//
// The build script of the sample splits the pictures and fonts by the pages that use them and lists the
// bundles in control-catalog.assets.json: the start-up bundle, which main.js registers before the
// application starts, and the bundles of each page (by the header of its entry in the page list). Before
// the catalog creates a page it calls ensurePageAssets (through the module, page_assets_browser.rs) and
// waits until the bundles of the page are fetched and registered. After the first frame the others are
// fetched in the background, one at a time while the page is idle, so that most navigations find their
// bundles there already.
//
// For the behaviour tests and for inspection, globalThis.controlCatalogAssets.requests lists every bundle
// requested, why ("start-up", "page" or "prefetch") and when (milliseconds from the start of navigation).

const requests = [];
globalThis.controlCatalogAssets = { requests };

let runtime = null;
let manifest = null;
// Bundle name -> { loaded, promise }.
const bundles = new Map();

/** Fetches a bundle of the site as bytes. */
export function fetchBundle(name, reason) {
    requests.push({ name, reason, start: performance.now() });
    return fetch(`./${name}`).then((response) => {
        if (!response.ok) throw new Error(`${name}: ${response.status} ${response.statusText}`);
        return response.arrayBuffer();
    });
}

/** Starts to serve the bundles of `manifest_` to the module `runtime_`, whose start-up bundle is registered. */
export function attach(runtime_, manifest_) {
    runtime = runtime_;
    manifest = manifest_;
    bundles.set(manifest.startup, { loaded: true, promise: Promise.resolve() });
}

function load(name, reason) {
    let entry = bundles.get(name);
    if (!entry) {
        entry = { loaded: false };
        entry.promise = fetchBundle(name, reason).then((buffer) => {
            // Deviation (browser-platform.md section 14): a page's bundle is registered on demand.
            runtime.registerPageAssetBundle(new Uint8Array(buffer));
            entry.loaded = true;
        }, (error) => {
            // A later navigation to a page that needs the bundle tries again.
            bundles.delete(name);
            throw error;
        });
        bundles.set(name, entry);
    }
    return entry;
}

/**
 * Called by the module before the catalog creates the page with the header `page`: null when the bundles of
 * the page are registered, otherwise a promise that settles once they are (or that rejects when one cannot
 * be fetched).
 */
export function ensurePageAssets(page) {
    const pending = (manifest?.pages[page] ?? []).map((name) => load(name, "page")).filter((entry) => !entry.loaded);
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

/** After the first frame, fetches the bundles no page has asked for yet, in the order of the manifest. */
export async function prefetchAfterFirstFrame() {
    await firstFrame();
    performance.mark("first frame");
    for (const name of manifest.prefetch) {
        await idle();
        try {
            await load(name, "prefetch").promise;
        } catch (error) {
            console.warn(`prefetching ${name} failed: ${error.message}`);
        }
    }
    performance.mark("assets prefetched");
}
