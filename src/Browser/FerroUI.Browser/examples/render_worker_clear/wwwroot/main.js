const is_browser = typeof window != "undefined";
if (!is_browser) throw new Error(`Expected to be running in a browser`);

const out = document.getElementById("out");
const view = document.getElementById("view");
const result = document.getElementById("result");
const frames = document.getElementById("frames");
const wait = document.getElementById("wait");

// The options of the page:
//   ?RenderingMode=Software2D|WebGL1|WebGL2   the render target of the canvas (default: WebGL2, else WebGL1)
//   ?Frames=true                              a frame loop on the thread, a new colour each frame
//   ?Wait=OutOfTurn|NextFrame                 the experiment: once the loop runs, the thread of the page
//                                             waits for a frame, with or without asking for one out of turn
//   ?ThreadsServiceWorker=false               see below
const query = new URLSearchParams(globalThis.location.search);
const MODES = { Software2D: 1, WebGL1: 2, WebGL2: 3 };
const mode = MODES[query.get("RenderingMode")] ?? 0;
const animated = query.get("Frames") === "true";
const waitFor = query.get("Wait");
// How long the thread of the page waits at most in the experiment, and after how many frames the
// experiment of the query runs.
const WAIT_TIMEOUT = 2000;
const WAIT_AFTER_FRAMES = 30;

// The check of the threaded mode, which `scripts/build-browser.sh --threads` copies next to the
// page. A build without the option has no such file: the module could not start a thread anyway.
const threads = await import("./ferroui-threads.js").catch(() => null);
if (threads === null) {
    result.textContent = "state=failed error=the site was built without threads: scripts/build-browser.sh render_worker_clear --threads";
} else {
    // `?ThreadsServiceWorker=false` leaves out the service worker that isolates the page on a host
    // without the headers, to see the message of a page that is not isolated.
    const register = query.get("ThreadsServiceWorker") !== "false";
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

        // Called by the module when the thread of the page is woken after a frame of the other
        // thread: the frame counter arrives here, nothing asks for it.
        let waited = false;
        const reporter = {
            frames(count, wakeUps) {
                frames.textContent = `frames=${count} wakeups=${wakeUps}`;
                if ((waitFor === "OutOfTurn" || waitFor === "NextFrame") && !waited && count >= WAIT_AFTER_FRAMES) {
                    waited = true;
                    // In a task of its own: this function is called from inside the module.
                    setTimeout(() => {
                        wait.textContent = "wait=running";
                        wait.textContent = "wait=" + waitFor + " " + runtime.renderWorkerClearWaitForFrame(waitFor === "OutOfTurn", WAIT_TIMEOUT);
                    }, 0);
                }
            }
        };

        // The size of the canvas in device pixels, until the observer of the canvas reports it:
        // the thread sets it on the transferred canvas before each frame.
        const scale = globalThis.devicePixelRatio;
        runtime.renderWorkerClearStart(Math.round(view.clientWidth * scale), Math.round(view.clientHeight * scale), scale, mode, animated, reporter);
        // The main thread does not wait for the thread: until the first frame it asks again from
        // a timer.
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
