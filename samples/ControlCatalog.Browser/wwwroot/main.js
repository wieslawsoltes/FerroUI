import createRuntime from "./control-catalog-browser.js";
import { FerroExports } from "./ferroui.js";
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
// The steps of the start-up are marked on the performance timeline of the page
// (scripts/browser/first-frame.mjs --phases reports them).
const runtime = await createRuntime();
performance.mark("module instantiated");

// The script side resolves the exports of the framework through the module.
FerroExports.attach(runtime);

pageAssets.attach(runtime, await manifest);
const files = await startup;
performance.mark("start-up assets downloaded");
// Deviation (browser-platform.md section 14): only the start-up files are registered before the start.
for (const [asset, content] of files) pageAssets.register(asset, content);

// For the behaviour tests (`catalogState`) and for inspection from the console.
globalThis.controlCatalog = runtime;

performance.mark("runMain start");
runtime.runMain(globalThis.location.href);
performance.mark("runMain end");

// `?PrefetchAssets=false` leaves the files of the pages to the navigations that need them (for the
// behaviour tests and measurements).
const prefetch = !/(^|[?&])prefetchassets=false(&|$)/i.test(globalThis.location.search);
if (prefetch) pageAssets.prefetchAfterFirstFrame();
