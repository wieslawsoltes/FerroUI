import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";

const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

// The script of the module is imported by the loader the build copies next to the page
// (scripts/browser/threads/ferroui-loader.js). A site has the module built without threads, the one
// built with threads (`scripts/build-browser.sh themed_view --threads`), or both (`--both`). A module
// built with threads can only be created in a cross-origin isolated page: on a site with that module
// alone the loader isolates the page through the service worker (the page is reloaded once, and the
// promise does not resolve) or shows a message and answers null; on a site with both it chooses the
// module before either is requested, and falls back to the one without threads without a message.
// `?Threads=true|false` forces either there.
const loaded = await loadModule({ script: "./themed_view.js", element: document.getElementById("out") });

if (loaded) {
    const runtime = await loaded.createRuntime();

    // The script side resolves the exports of the framework through the module.
    FerroExports.attach(runtime);

    // For the behaviour tests and for inspection from the console.
    globalThis.themedView = runtime;

    runtime.runMain(globalThis.location.search);
}
