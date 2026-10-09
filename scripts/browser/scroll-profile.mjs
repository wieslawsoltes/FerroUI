// Processor time of scrolling a page of the ControlCatalog site in headless Chrome, per thread, with a CPU profile.
//
//   node scripts/browser/scroll-profile.mjs <site directory> [options]
//
//   --group <text>          entry of the navigation drawer that holds the page (default "Collections & Data")
//   --page <text>           entry of the page below it, and its header (default "TableView")
//   --steps <n>             wheel events, the first half down and the second half up (default 60)
//   --delta <pixels>        distance of one wheel event (default 120)
//   --interval <ms>         time between two wheel events (default 50)
//   --query <text>          more of the query string of the page, e.g. "&RenderThread=false"
//   --isolated              serve the site cross-origin isolated (a site built with threads)
//   --angle <backend>       what WebGL runs on: swiftshader (the default) or a backend of ANGLE with the GPU (metal)
//   --trace                 measure with a trace of the browser and not with its sampling profiler (below)
//   --latency               measure the delay from each wheel event to the frame that shows it (below)
//   --cpu-profile <file>    write the CPU profile (.cpuprofile) of the scrolling: of the thread of the page, and
//                           of each worker next to it (<file>.worker-<n>.cpuprofile)
//   --screenshot <file>     write a PNG of the page after the scrolling
//   --json                  print the result as JSON
//
// The site is the one scripts/build-browser.sh control-catalog-browser writes, built with or without threads.
// The page is opened through the drawer with real pointer events (the `catalogState` export of the host names
// what is shown, and nothing is clicked before the view has drawn it), and the wheel events are sent at the
// middle of the content.
//
// Without --trace the profiler of the browser samples the thread of the page and every worker of the page (a
// thread of a module built with threads is a worker; the render thread is one), and the time each was neither
// idle nor inside the browser itself is printed: the time of the module and of its script.
//
// With --trace a trace of the browser is recorded instead, which does not slow the threads down as sampling
// does and has every task of every thread: the processor time and the wall-clock time of the tasks of the
// thread of the page, of the workers and of the main thread of the GPU process; the time the thread of the
// page was busy from one wheel event to the next (how long it could not take other input), and its longest
// task.
//
// With --latency nothing is profiled: after each wheel event the view is asked for a frame with everything the
// application changed (`catalogRequestFrame`), and the time from the wheel event (when the page dispatched it)
// to the moment that frame was drawn (`catalogFrameDrawn`, asked without pause) is taken.
//
// For a profile with the names of the functions, link the module with its name section (`cargo rustc --profile
// browser --target wasm32-unknown-emscripten -p control-catalog-browser --bin control-catalog-browser --
// -Clink-arg=--profiling-funcs`) and put it, with its script, in place of the module of the site. The figures
// depend on the machine and its load: compare two sites by measuring them alternately. No dependencies.
import fs from "node:fs";
import path from "node:path";
import { open, sleep, distribution, threadTimes, dispatched } from "./harness.mjs";
import { DRAWER_EDGE, drive, navigate, waitUntilReady } from "./catalog-pages.mjs";

const args = process.argv.slice(2);
const valued = new Set(["--group", "--page", "--steps", "--delta", "--interval", "--query", "--angle", "--cpu-profile", "--screenshot"]);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/scroll-profile.mjs <site directory> [--group g] [--page p] [--steps n] [--delta d] [--interval ms] [--query q] [--isolated] [--angle a] [--trace] [--latency] [--cpu-profile f] [--screenshot f] [--json]");
    process.exit(2);
}
const group = option("--group", "Collections & Data");
const name = option("--page", "TableView");
const steps = Number(option("--steps", "60"));
const delta = Number(option("--delta", "120"));
const interval = Number(option("--interval", "50"));
const query = option("--query", "");
const angle = option("--angle");
const profileFile = option("--cpu-profile");
const screenshotFile = option("--screenshot");
const trace = args.includes("--trace");
const latency = args.includes("--latency");

const SIZE = { width: 1280, height: 900 };
const round = (value, digits = 1) => Number(value.toFixed(digits));

// The time of every node of a profile but the idle time and the time inside the browser itself, in milliseconds.
function busyOf(profile) {
    const names = new Map(profile.nodes.map((node) => [node.id, node.callFrame.functionName]));
    let total = 0, busy = 0;
    profile.samples.forEach((id, i) => {
        const microseconds = profile.timeDeltas[i];
        total += microseconds;
        if (!["(idle)", "(program)", "(root)"].includes(names.get(id))) { busy += microseconds; }
    });
    return { busyMs: busy / 1000, totalMs: total / 1000 };
}

// The moment the page dispatched the last wheel event, and the wait for the frame that shows it.
const initScript = `(() => {
    globalThis.__ferroWheelAt = 0;
    addEventListener("wheel", () => { globalThis.__ferroWheelAt = performance.now(); }, { capture: true, passive: true });
    globalThis.__ferroFrameAfterWheel = () => new Promise((resolve) => {
        const start = globalThis.__ferroWheelAt;
        controlCatalog.catalogRequestFrame();
        const channel = new MessageChannel();
        const check = () => {
            const now = performance.now();
            if (controlCatalog.catalogFrameDrawn()) { resolve(now - start); } else if (now - start > 5000) { resolve(null); } else { channel.port2.postMessage(0); }
        };
        channel.port1.onmessage = check;
        check();
    });
})();`;

const page = await open(site, { query: `?RenderingMode=WebGL2${query}`, ...SIZE, isolated: args.includes("--isolated"), angle, initScript });
try {
    await waitUntilReady(page, 180_000);
    drive(page);
    await navigate(page, group);
    await navigate(page, name);
    await sleep(1500);
    await page.drawn();

    const browser = await page.browser();
    const workers = [...await page.attachWorkers()];
    const before = await page.rendering();
    let stopTrace;
    if (trace) { stopTrace = await page.trace(); } else if (!latency) {
        for (const sessionId of [undefined, ...workers.map((worker) => worker.sessionId)]) {
            await page.send("Profiler.enable", {}, sessionId);
            await page.send("Profiler.setSamplingInterval", { interval: 200 }, sessionId);
            await page.send("Profiler.start", {}, sessionId);
        }
    }
    const started = Date.now();
    const x = Math.round((DRAWER_EDGE + SIZE.width) / 2), y = Math.round(SIZE.height / 2);
    const delays = [];
    for (let i = 0; i < steps; i++) {
        const next = Date.now() + interval;
        await page.send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY: i < steps / 2 ? delta : -delta, pointerType: "mouse" });
        if (latency) { delays.push(await page.evaluate("globalThis.__ferroFrameAfterWheel()")); }
        await sleep(Math.max(0, next - Date.now()));
    }
    await sleep(300);
    const elapsed = Date.now() - started;
    const after = await page.rendering();

    const result = {
        page: name, steps, delta, interval, browser: browser.product, angle: angle ?? process.env.FERROUI_BROWSER_ANGLE ?? "swiftshader",
        renderThread: after.on_render_thread === "true" && after.other_thread === "true",
        frames: Number(after.frames) - Number(before.frames), elapsedMs: elapsed
    };
    if (trace) {
        const events = await stopTrace();
        const wheels = dispatched(events, "wheel");
        if (wheels.length === 0) { throw new Error("the trace has no wheel event"); }
        // From the first wheel event until the page is quiet again.
        const window = { from: wheels[0], to: wheels[wheels.length - 1] + interval + 300 };
        const threads = threadTimes(events, window);
        // The time the thread of the page was busy from one wheel event to the next.
        const perEvent = wheels.map((from, i) => {
            const to = wheels[i + 1] ?? window.to;
            let busy = 0;
            threads.main.starts.forEach((start, task) => { if (start >= from && start < to) { busy += threads.main.durations[task]; } });
            return busy;
        });
        const workerCpu = threads.workers.reduce((sum, thread) => sum + thread.cpuMs, 0);
        const workerWall = threads.workers.reduce((sum, thread) => sum + thread.wallMs, 0);
        Object.assign(result, {
            wheelEvents: wheels.length,
            mainCpuMs: round(threads.main.cpuMs), mainWallMs: round(threads.main.wallMs),
            mainCpuPerEventMs: round(threads.main.cpuMs / wheels.length, 2), mainWallPerEventMs: round(threads.main.wallMs / wheels.length, 2),
            mainBusyPerEvent: distribution(perEvent), mainTasks: distribution(threads.main.durations),
            mainTasksOver16: threads.main.durations.filter((d) => d > 16.7).length, mainTasksOver50: threads.main.durations.filter((d) => d > 50).length,
            workerCpuMs: round(workerCpu), workerWallMs: round(workerWall), workerThreads: threads.workers.filter((thread) => thread.cpuMs >= 1).length,
            workerTasks: distribution(threads.workers[0]?.durations ?? []),
            gpuMainCpuMs: round(threads.gpu.cpuMs),
            totalCpuMs: round(threads.main.cpuMs + workerCpu)
        });
    } else if (latency) {
        const reached = delays.filter((d) => d !== null);
        Object.assign(result, { frameDelay: distribution(reached), framesNotDrawnIn5s: delays.length - reached.length, frameDelays: delays.map((d) => (d === null ? null : round(d))) });
    } else {
        const { profile } = await page.send("Profiler.stop");
        if (profileFile) { fs.writeFileSync(profileFile, JSON.stringify(profile)); }
        const main = busyOf(profile);
        let workerBusy = 0; const each = [];
        for (const [i, worker] of workers.entries()) {
            const stopped = await page.send("Profiler.stop", {}, worker.sessionId);
            if (!stopped.profile) { continue; }
            if (profileFile) { fs.writeFileSync(`${profileFile}.worker-${i + 1}.cpuprofile`, JSON.stringify(stopped.profile)); }
            const busy = busyOf(stopped.profile).busyMs;
            workerBusy += busy; each.push(round(busy));
        }
        Object.assign(result, {
            busyMs: round(main.busyMs), perStepMs: round(main.busyMs / steps, 2), totalMs: Math.round(main.totalMs),
            workerBusyMs: round(workerBusy), workerPerStepMs: round(workerBusy / steps, 2), workers: each
        });
    }
    if (screenshotFile) { await page.screenshot(screenshotFile); }
    if (page.errors.length > 0) { result.errors = page.errors; }

    if (args.includes("--json")) { console.log(JSON.stringify(result)); } else {
        const where = `${site}${query ? ` (${query})` : ""}: ${name}, ${steps} wheel events of ${delta} px, ${result.browser}, WebGL on ${result.angle}, drawn by ${result.renderThread ? "a render thread" : "the thread of the page"}, ${result.frames} frames`;
        if (trace) {
            console.log(`${where}\n  thread of the page: ${result.mainCpuMs} ms of processor time (${result.mainCpuPerEventMs} ms per event), ${result.mainWallMs} ms in tasks (${result.mainWallPerEventMs} ms per event); ` +
                `busy per event, ms: median ${result.mainBusyPerEvent.median}, p95 ${result.mainBusyPerEvent.p95}, max ${result.mainBusyPerEvent.max}; longest task ${result.mainTasks.max} ms, ` +
                `${result.mainTasksOver16} tasks over 16.7 ms, ${result.mainTasksOver50} over 50 ms\n  workers: ${result.workerCpuMs} ms of processor time, ${result.workerWallMs} ms in tasks, on ${result.workerThreads} threads; ` +
                `longest task of the busiest ${result.workerTasks.max ?? "-"} ms\n  main thread of the GPU process: ${result.gpuMainCpuMs} ms of processor time`);
        } else if (latency) {
            console.log(`${where}\n  from a wheel event to the frame that shows it, ms: median ${result.frameDelay.median}, p95 ${result.frameDelay.p95}, max ${result.frameDelay.max}` +
                (result.framesNotDrawnIn5s > 0 ? `; ${result.framesNotDrawnIn5s} frames were not drawn within 5 s` : ""));
        } else {
            console.log(`${where}\n  thread of the page: ${result.busyMs} ms of processor time (${result.perStepMs} ms per event) in ${result.totalMs} ms` +
                (workers.length > 0 ? `\n  workers: ${result.workerBusyMs} ms of processor time (${result.workerPerStepMs} ms per event): ${result.workers.join(", ")}` : ""));
        }
        if (page.errors.length > 0) { console.log(`errors:\n${page.errors.join("\n")}`); }
    }
} finally {
    await page.close();
}
