import createRuntime from "./control-catalog-browser.js";
import { FerroExports } from "./ferroui.js";
import * as pageAssets from "./page-assets.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

// The pictures and fonts of the catalog are not in the module: they are asset bundles the build script of
// the sample writes, split by the pages that use them (page-assets.js). The start-up bundle and the list of
// the bundles are fetched while the module is compiled (the page preloads them); the bundles of the pages
// are fetched on demand.
const manifest = fetch("./control-catalog.assets.json").then((response) => {
    if (!response.ok) throw new Error(`control-catalog.assets.json: ${response.status} ${response.statusText}`);
    return response.json();
});
const startup = pageAssets.fetchBundle("control-catalog.assets", "start-up");
// The steps of the start-up are marked on the performance timeline of the page
// (scripts/browser/first-frame.mjs --phases reports them).
const runtime = await createRuntime();
performance.mark("module instantiated");

// The script side resolves the exports of the framework through the module.
FerroExports.attach(runtime);

const bundle = new Uint8Array(await startup);
performance.mark("asset bundle downloaded");
// Deviation (browser-platform.md section 14): only the start-up bundle is registered before the start.
runtime.registerAssetBundle(bundle);
pageAssets.attach(runtime, await manifest);

// For the behaviour tests (`catalogState`) and for inspection from the console.
globalThis.controlCatalog = runtime;

performance.mark("runMain start");
runtime.runMain(globalThis.location.href);
performance.mark("runMain end");

// `?PrefetchAssets=false` leaves the bundles of the pages to the navigations that need them (for the
// behaviour tests and measurements).
const prefetch = !/(^|[?&])prefetchassets=false(&|$)/i.test(globalThis.location.search);
if (prefetch) pageAssets.prefetchAfterFirstFrame();
