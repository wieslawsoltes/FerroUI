import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import * as pageAssets from "./page-assets.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

// The pictures and fonts of the catalog are not in the module: they are plain files of the site, one per
// asset, listed with the pages that use them (page-assets.js). The list and the files the start-up needs
// are fetched while the module is compiled (the page preloads them); the files of the pages are fetched
// on demand.
const manifest = pageAssets.fetchManifest();
const startup = manifest.then((list) => Promise.all(list.startup.map((asset) =>
    pageAssets.fetchFile(list, asset, "start-up").then((content) => [asset, content]))));

// The script of the module is imported by the loader the build copies next to the page
// (scripts/browser/threads/ferroui-loader.js), not by an import of this file. A site has the module built
// without threads, the one built with threads (`scripts/build-browser.sh control-catalog-browser --threads`),
// or both (`--both`, the published site). A module built with threads can only be created in a cross-origin
// isolated page and in a browser that can render from a worker, so on a site with both the loader decides
// before either is requested: the one with threads when the page is isolated, or can be made so by the
// service worker (the page is reloaded once, and the promise does not resolve), and the browser has what
// the render thread needs; the other one otherwise. `?Threads=true|false` forces either. On a site with the
// module with threads alone a page that cannot be isolated shows a message, and the loader answers null.
// A site with one module makes no request it did not make before, except for the loader itself.
const loaded = await loadModule({ script: "./control-catalog-browser.js", element: document.getElementById("out") });

if (loaded) {
    // The steps of the start-up are marked on the performance timeline of the page
    // (scripts/browser/first-frame.mjs --phases reports them).
    const runtime = await loaded.createRuntime();
    performance.mark("module instantiated");

    // The script side resolves the exports of the framework through the module.
    FerroExports.attach(runtime);

    pageAssets.attach(runtime, await manifest);
    const files = await startup;
    performance.mark("start-up assets downloaded");
    // Deviation (browser-platform.md section 14): only the start-up files are registered before the start.
    for (const [asset, content] of files) pageAssets.register(asset, content);

    // For the behaviour tests (`catalogState`, `catalogRendering`, `catalogMemory`) and for inspection
    // from the console.
    globalThis.controlCatalog = runtime;
    // The panics of the render thread the platform reported to this page, oldest first.
    globalThis.controlCatalogPanics = () => FerroExports.renderThreadPanics;

    performance.mark("runMain start");
    runtime.runMain(globalThis.location.href);
    performance.mark("runMain end");

    // `?MemoryReport=true` samples the memory of the module ten times a second, so that the peak
    // `catalogMemory` reports is not only the peak of the moments somebody asked (for the measurement
    // of the memory a site built with threads needs: its memory is fixed).
    if (/(^|[?&])memoryreport=true(&|$)/i.test(globalThis.location.search)) {
        setInterval(() => runtime.catalogMemory(), 100);
    }

    // `?PrefetchAssets=false` leaves the files of the pages to the navigations that need them (for the
    // behaviour tests and measurements).
    const prefetch = !/(^|[?&])prefetchassets=false(&|$)/i.test(globalThis.location.search);
    if (prefetch) pageAssets.prefetchAfterFirstFrame();
}
