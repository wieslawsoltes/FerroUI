import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";

try {
    const loaded = await loadModule({ script: "./editor-demo-browser.js", element: document.getElementById("out") });
    if (loaded) {
        const runtime = await loaded.createRuntime();
        FerroExports.attach(runtime);
        // Registry construction compiles grammar regexes synchronously. Complete
        // the real scanner initialization before the Rust view can be created.
        await runtime.initializeScanner();
        globalThis.editorDemo = runtime;
        runtime.runMain(globalThis.location.href);
        document.documentElement.dataset.editorReady = "true";
    }
} catch (error) {
    document.documentElement.dataset.editorError = String(error);
    const splash = document.querySelector(".ferroui-splash");
    if (splash) {
        splash.dataset.failed = "true";
        splash.setAttribute("role", "alert");
        splash.textContent = `The editor could not start.\n${error}`;
    }
    console.error(error);
}
