import createRuntime from "./control-catalog-browser.js";
import { FerroExports } from "./ferroui.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

// The pictures and fonts of the catalog are not in the module: they are the asset bundle the build
// script of the sample writes, fetched while the module is compiled.
const assets = fetch("./control-catalog.assets").then((response) => {
    if (!response.ok) throw new Error(`control-catalog.assets: ${response.status} ${response.statusText}`);
    return response.arrayBuffer();
});
// The steps of the start-up are marked on the performance timeline of the page
// (scripts/browser/first-frame.mjs --phases reports them).
const runtime = await createRuntime();
performance.mark("module instantiated");

// The script side resolves the exports of the framework through the module.
FerroExports.attach(runtime);

const bundle = new Uint8Array(await assets);
performance.mark("asset bundle downloaded");
runtime.registerAssetBundle(bundle);

// For the behaviour tests (`catalogState`) and for inspection from the console.
globalThis.controlCatalog = runtime;

performance.mark("runMain start");
runtime.runMain(globalThis.location.href);
performance.mark("runMain end");
