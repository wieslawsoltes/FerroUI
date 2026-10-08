const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

const out = document.getElementById("out");
const view = document.getElementById("view");
const result = document.getElementById("result");

// The check of the threaded mode, which `scripts/build-browser.sh --threads` copies next to the
// page. A build without the option has no such file: the module could not start a thread anyway.
const threads = await import("./ferroui-threads.js").catch(() => null);
if (threads === null) {
    result.textContent = "state=failed error=the site was built without threads: scripts/build-browser.sh render_worker_clear --threads";
} else {
    // `?ThreadsServiceWorker=false` leaves out the service worker that isolates the page on a host
    // without the headers, to see the message of a page that is not isolated.
    const register = new URLSearchParams(globalThis.location.search).get("ThreadsServiceWorker") !== "false";
    // Resolves to false with a message in the page, or reloads the page once and does not resolve.
    if (await threads.ensureCrossOriginIsolated({ register, element: out })) {
        // Imported only now: creating a module with shared memory fails in a page that is not isolated.
        const { default: createRuntime } = await import("./render_worker_clear.js");
        const { FerroExports } = await import("./ferroui.js");
        const runtime = await createRuntime();

        // The script side resolves the exports of the framework, and the workers of the threads,
        // through the module.
        FerroExports.attach(runtime);

        // For the behaviour tests and for inspection from the console.
        globalThis.renderWorkerClear = runtime;

        // The size of the canvas in device pixels: the thread sets it on the transferred canvas.
        const scale = globalThis.devicePixelRatio;
        runtime.renderWorkerClearStart(Math.round(view.clientWidth * scale), Math.round(view.clientHeight * scale));
        // The main thread does not wait for the thread: it asks again from a timer.
        const poll = () => {
            let state = runtime.renderWorkerClearState();
            if (state.includes("state=worker_ready")) {
                // The worker of the thread takes messages now: the canvas is created and its
                // control transferred.
                runtime.renderWorkerClearCreateSurface(view);
                state = runtime.renderWorkerClearState();
            }
            result.textContent = state;
            if (!/state=(done|failed)/.test(state)) { setTimeout(poll, 20); }
        };
        poll();
    }
}
