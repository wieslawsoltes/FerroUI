const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

const out = document.getElementById("out");
const result = document.getElementById("result");

// The check of the threaded mode, which `scripts/build-browser.sh --threads` copies next to the
// page. A build without the option has no such file: the module could not start a thread anyway.
const threads = await import("./ferroui-threads.js").catch(() => null);
if (threads === null) {
    result.textContent = "state=failed error=the site was built without threads: scripts/build-browser.sh thread_spawn --threads";
} else {
    // `?ThreadsServiceWorker=false` leaves out the service worker that isolates the page on a host
    // without the headers, to see the message of a page that is not isolated.
    const register = new URLSearchParams(globalThis.location.search).get("ThreadsServiceWorker") !== "false";
    // Resolves to false with a message in the page, or reloads the page once and does not resolve.
    if (await threads.ensureCrossOriginIsolated({ register, element: out })) {
        // Imported only now: creating a module with shared memory fails in a page that is not isolated.
        const { default: createRuntime } = await import("./thread_spawn.js");
        const runtime = await createRuntime();

        // For the behaviour tests and for inspection from the console.
        globalThis.threadSpawn = runtime;

        runtime.threadSpawnStart();
        // The main thread does not wait for the thread: it asks again from a timer.
        const poll = () => {
            const state = runtime.threadSpawnState();
            result.textContent = state;
            if (state.includes("state=running")) { setTimeout(poll, 20); }
        };
        poll();
    }
}
