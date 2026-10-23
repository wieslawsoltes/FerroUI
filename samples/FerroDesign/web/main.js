import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import { openDocumentStorage } from "./document-store.js";

// The host only loads the native application. The editor and canvas are rendered by FerroUI.
const splash = document.querySelector(".ferroui-splash");
const message = document.getElementById("loading-message");
const errors = [];
globalThis.ferroDesignErrors = errors;

function reportError(error) {
    const detail = error?.stack || error?.message || String(error);
    errors.push(detail);
    console.error("FerroDesign", error);
    splash.classList.remove("splash-close");
    splash.classList.add("failed");
    message.textContent = "The workspace could not start";
    const output = document.getElementById("loading-error");
    output.hidden = false;
    output.textContent = detail;
}

try {
    performance.mark("ferrodesign-load-start");
    // Open storage while the Wasm module loads. Rust constructs the document
    // synchronously, so its complete saved text must be ready before runMain.
    const [storage, loaded] = await Promise.all([
        openDocumentStorage(),
        loadModule({ script: "./ferrodesign-app.js", element: document.getElementById("out") }),
    ]);
    if (!loaded) throw new Error("The browser could not initialize the WebAssembly runtime.");
    globalThis.ferroDesignStorage = storage;
    storage.attachLifecycle(globalThis, document);
    message.textContent = "Preparing the design canvas";
    const runtime = await loaded.createRuntime();
    FerroExports.attach(runtime);
    globalThis.ferroDesign = runtime;
    globalThis.ferrodesign = runtime;
    globalThis.ferroDesignRenderPanics = () => [...FerroExports.renderThreadPanics];
    performance.mark("ferrodesign-runtime-ready");
    runtime.runMain(globalThis.location.search);
    storage.setDraftCommitter(() => runtime.ferroDesignCommand("commit-edit"));
    let lastStorageError = "";
    storage.setStatusListener((status) => {
        globalThis.ferroDesignSaveState = status;
        runtime.ferroDesignCommand?.("storage-status");
        if (status.error && ["error", "unavailable", "conflict"].includes(status.phase)
            && status.error !== lastStorageError) {
            lastStorageError = status.error;
            runtime.ferroDesignHostError?.(status.error);
        }
        if (status.phase === "saved") lastStorageError = "";
        globalThis.dispatchEvent(new CustomEvent("ferrodesign-storage", { detail: status }));
    });
    performance.mark("ferrodesign-app-ready");
    globalThis.ferroDesignReady = true;
    globalThis.dispatchEvent(new CustomEvent("ferrodesign-ready"));
} catch (error) {
    reportError(error);
}
