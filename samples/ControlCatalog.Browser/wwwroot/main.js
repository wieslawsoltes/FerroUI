import createRuntime from "./control-catalog-browser.js";
import { FerroExports } from "./ferroui.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

const runtime = await createRuntime();

// The script side resolves the exports of the framework through the module.
FerroExports.attach(runtime);

// For the behaviour tests (`catalogState`) and for inspection from the console.
globalThis.controlCatalog = runtime;

runtime.runMain(globalThis.location.href);
