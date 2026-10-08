import createRuntime, * as moduleScript from "./storage_view.js";
import { FerroExports } from "./ferroui.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

// A module built with threads (`scripts/build-browser.sh storage_view --threads`) says so in its
// script, and it can only be created in a cross-origin isolated page. The check, which that build
// copies next to the page, resolves to true in such a page; otherwise it registers the service
// worker that isolates the page and reloads the page once (it does not resolve then), or shows a
// message and resolves to false. The worker is the one the application registers with
// `?RegisterServiceWorker=true`, under the same address.
let ready = true;
if (moduleScript.ferrouiThreads === true) {
    const { ensureCrossOriginIsolated } = await import("./ferroui-threads.js");
    ready = await ensureCrossOriginIsolated({ element: document.getElementById("out") });
}

if (ready) {
    const runtime = await createRuntime();

    // The script side resolves the exports of the framework through the module.
    FerroExports.attach(runtime);

    // For the behaviour tests and for inspection from the console.
    globalThis.storageView = runtime;

    runtime.runMain(globalThis.location.search);
}
