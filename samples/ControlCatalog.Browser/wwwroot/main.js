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
const runtime = await createRuntime();

// The script side resolves the exports of the framework through the module.
FerroExports.attach(runtime);

runtime.registerAssetBundle(new Uint8Array(await assets));

// For the behaviour tests (`catalogState`) and for inspection from the console.
globalThis.controlCatalog = runtime;

runtime.runMain(globalThis.location.href);
