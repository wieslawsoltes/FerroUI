// Frame times of a page of the ControlCatalog site in headless Chromium, driven with real input events,
// and what the thread of the page and the thread that renders do meanwhile.
//
//   node scripts/browser/frame-times.mjs <site directory> [options]
//
//   --mode <mode>           rendering mode of the page (default WebGL2; Software2D)
//   --page <entry>          drawer entry of the page to open (default TableView); "Home" stays on the home page
//   --section <entry>       drawer entry of the section of the page (known for the pages named below)
//   --scenario <name>       wheel: a wheel event of --delta pixels each frame over the middle of the content,
//                           down for half of the frames and up for the other half (default);
//                           hover: the pointer moves to the next button of the page each frame;
//                           move: the pointer moves by one pixel each frame inside the first button;
//                           idle: nothing happens (a page that animates goes on drawing);
//                           navigate: the drawer entries --page and --other are clicked in turn, one every
//                           1.2 s, so that the view runs a page transition each time (needs --seconds);
//                           start: the first --seconds of the page from the start of its navigation, with
//                           nothing sent: the animation frames the thread that renders was given while the
//                           view drew its first frames (see below; implies --frame-rate-limit)
//   --other <entry>         the second page of the scenario navigate, of the same section (default Slider
//                           for Buttons)
//   --click <text>          click the element of the page with this text before measuring (a tab of the page,
//                           a button that starts an animation)
//   --frames <n>            number of input events, one per animation frame (default 300)
//   --seconds <n>           for idle and navigate: measure for this long and send nothing per frame
//   --delta <px>            wheel delta in CSS pixels (default 20)
//   --size <w>x<h>          size of the viewport (default 1280x800)
//   --query <text>          more of the query string of the page, e.g. "&RenderThread=false"
//   --isolated              serve the site cross-origin isolated (a site built with threads)
//   --angle <backend>       what WebGL runs on: swiftshader (the default) or a backend of ANGLE with the GPU (metal)
//   --frame-rate-limit      keep the frame rate limit of the browser (see below)
//   --trace                 record a trace of the browser during the measurement (see below)
//   --cpu-profile <file>    write a CPU profile (.cpuprofile) of the measured frames
//   --top <n>               print the n functions with the most self time of the CPU profile (default 25;
//                           0 leaves the profiler off, which a measurement of times should)
//   --screenshot <file>     write a PNG of the page after the measured frames
//   --json                  print the result as JSON
//
// The browser is started without its frame rate limit (`--disable-frame-rate-limit`,
// `--disable-gpu-vsync`), so that animation frames follow as fast as the page can produce them: the
// frame rate measures the cost of a frame, not the refresh rate of a display. With --frame-rate-limit
// the limit stays, and the times are those a user of a display of that rate would get: use it for the
// gaps between animation frames and for how long the thread of the page is blocked.
//
// A script added to the page records the time of every animation frame of the page, the start and the
// duration of the animation frame callbacks of the page itself (the render timer of the framework renders
// in them, when the thread of the page renders), the gaps of a timer that fires every 4 ms (a gap longer
// than that is time the thread of the page could not run anything: `heartbeat`) and the long tasks the
// browser reports (over 50 ms). In a module built with threads the render timer runs in a worker: the
// same record of animation frame callbacks is installed in every worker of the page, and
// `renderingFrames` is that of the thread that renders. `framesDrawn` are the frames the view drew
// (`catalogRendering`). With --trace, the tasks of the thread of the page and of the workers are taken
// from a trace of the browser: their processor time, their wall-clock time and their longest.
//
// The scenario start records a trace from before the page exists and takes the animation frames of every
// thread from it. A browser paces the animation frames of a worker by what the worker's canvas has
// presented, and after frames that are expensive to present it withholds them: the gaps between the
// animation frames of the thread that renders are reported with, for each gap over 100 ms, how much of it
// that thread was busy itself (the rest it waited for an animation frame it had asked for), and the same
// for the animation frames of the page.
//
// Each input event is sent and the next animation frame awaited before the next one is sent, as a
// pointing device that reports once per frame does. The page-side cost per frame is read from the
// performance metrics of the page (script and task time of the main thread). WebGL runs on SwiftShader
// unless --angle says otherwise; SwiftShader is a "major performance caveat", so the script lets a WebGL
// context be created although the page asks for failIfMajorPerformanceCaveat (as first-frame.mjs does);
// nothing else of the page is changed.
//
// For names of WebAssembly functions in the CPU profile, link the module with its name section
// (`-C link-arg=--profiling-funcs`, see docs/porting/browser-platform.md).
import fs from "node:fs";
import path from "node:path";
import { open, sleep, distribution, threadTimes, animationFrames } from "./harness.mjs";
import { DRAWER_EDGE, drive, inContent, inDrawer, visit, waitUntilReady } from "./catalog-pages.mjs";

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const valued = new Set(["--mode", "--page", "--section", "--scenario", "--other", "--click", "--frames", "--seconds", "--delta", "--size", "--query", "--angle", "--cpu-profile", "--top", "--screenshot"]);
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/frame-times.mjs <site directory> [--mode m] [--page p] [--section s] [--scenario wheel|hover|move|idle|navigate|start] [--other p] [--frames n] [--seconds n] [--delta px] [--size WxH] [--query q] [--isolated] [--angle a] [--frame-rate-limit] [--trace] [--cpu-profile f] [--top n] [--json]");
    process.exit(2);
}
const mode = option("--mode", "WebGL2");
const pageEntry = option("--page", "TableView");
const scenario = option("--scenario", "wheel");
const frames = Number(option("--frames", "300"));
const seconds = option("--seconds") === undefined ? null : Number(option("--seconds"));
const delta = Number(option("--delta", "20"));
const [width, height] = option("--size", "1280x800").split("x").map(Number);
const query = option("--query", "");
const angle = option("--angle");
const cpuProfileFile = option("--cpu-profile");
const top = Number(option("--top", "25"));
const screenshotFile = option("--screenshot");
const json = args.includes("--json");
const trace = args.includes("--trace");
const limited = args.includes("--frame-rate-limit") || scenario === "start";
if (!["wheel", "hover", "move", "idle", "navigate", "start"].includes(scenario)) { console.error(`unknown scenario ${scenario}`); process.exit(2); }
if (seconds !== null && !["idle", "navigate", "start"].includes(scenario)) { console.error("--seconds is for the scenarios idle, navigate and start"); process.exit(2); }
if ((scenario === "navigate" || scenario === "start") && seconds === null) { console.error(`the scenario ${scenario} needs --seconds`); process.exit(2); }
const SECTIONS = { TableView: "Collections & Data", Buttons: "Basic Input", Slider: "Basic Input", ScrollViewer: "Layout", TextBlock: "Text", Composition: "Media & Graphics" };
const section = option("--section", SECTIONS[pageEntry]);
const other = option("--other", "Slider");
const clickText = option("--click");

// The record of the animation frame callbacks of a thread (the page or a worker): the start and the
// duration of each. Evaluated again it empties the record.
const recordAnimationFrames = `(() => {
    if (globalThis.__ferroRaf) { globalThis.__ferroRaf.starts.length = 0; globalThis.__ferroRaf.durations.length = 0; return true; }
    const raf = globalThis.requestAnimationFrame.bind(globalThis);
    const record = globalThis.__ferroRaf = { starts: [], durations: [] };
    globalThis.requestAnimationFrame = (callback) => raf((time) => {
        const start = performance.now();
        try { callback(time); } finally { record.starts.push(start); record.durations.push(performance.now() - start); }
    });
    return true;
})()`;

const initScript = `(() => {
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, attrs, ...rest) {
        if (attrs && typeof attrs === "object" && attrs.failIfMajorPerformanceCaveat) attrs = { ...attrs, failIfMajorPerformanceCaveat: false };
        const context = getContext.call(this, type, attrs, ...rest);
        if (context) globalThis.__ferroContextType = type;
        return context;
    };
    // The time of every animation frame of the page (a loop of its own, before the record below is
    // installed), and the animation frame callbacks of the page itself.
    const raf = requestAnimationFrame.bind(globalThis);
    globalThis.__ferroFrames = [];
    const frame = (time) => { globalThis.__ferroFrames.push(time); raf(frame); };
    raf(frame);
    ${recordAnimationFrames};
    // A timer every 4 ms: a longer gap is time the thread of the page could run nothing.
    globalThis.__ferroBeats = [];
    let last = performance.now();
    setInterval(() => { const now = performance.now(); globalThis.__ferroBeats.push(now - last); last = now; }, 4);
    globalThis.__ferroLongTasks = [];
    try { new PerformanceObserver((list) => { for (const entry of list.getEntries()) globalThis.__ferroLongTasks.push(entry.duration); }).observe({ type: "longtask" }); } catch { }
})();`;

const page = await open(site, {
    query: `?RenderingMode=${mode}${query}`, width, height, initScript, isolated: args.includes("--isolated"), angle, navigate: scenario !== "start",
    chromeArgs: [...(limited ? [] : ["--disable-frame-rate-limit", "--disable-gpu-vsync"]), "--ignore-gpu-blocklist"]
});
// The scenario start: the animation frames of the first seconds of the page, from a trace.
async function start() {
    const stopTrace = await page.trace();
    const began = Date.now();
    await page.navigate();
    await page.waitFor("globalThis.controlCatalog && /frames=[1-9]/.test(controlCatalog.catalogRendering())", 180_000);
    const firstFrameMs = Date.now() - began;
    await sleep(Math.max(0, seconds * 1000 - firstFrameMs));
    drive(page);
    const rendering = await page.rendering();
    const browser = await page.browser();
    const traced = await stopTrace();
    const onRenderThread = rendering.on_render_thread === "true" && rendering.other_thread === "true";
    const threads = threadTimes(traced);
    const frames = animationFrames(traced);
    // The thread that renders: the worker with the most animation frames, or the page.
    const worker = threads.workers.map((thread) => thread.key).sort((a, b) => (frames.get(b)?.length ?? 0) - (frames.get(a)?.length ?? 0))[0];
    const tasks = new Map([threads.main, ...threads.workers].map((thread) => [thread.key, thread]));
    // The gaps between the animation frames of a thread, and for those over 100 ms how long the thread
    // was in tasks of its own meanwhile.
    const gapsOf = (key) => {
        // Several callbacks of one animation frame (the page's and this script's) are one frame.
        const times = (frames.get(key) ?? []).filter((time, i, all) => i === 0 || time - all[i - 1] > 2); const thread = tasks.get(key);
        const gaps = times.slice(1).map((time, i) => time - times[i]);
        const long = [];
        gaps.forEach((gap, i) => {
            if (gap <= 100) { return; }
            let busy = 0;
            thread?.starts.forEach((begin, task) => { if (begin >= times[i] && begin < times[i + 1]) { busy += Math.min(thread.durations[task], times[i + 1] - begin); } });
            long.push({ afterMs: Math.round(times[i] - times[0]), gapMs: Math.round(gap), busyMs: Math.round(busy) });
        });
        return {
            animationFrames: times.length, gaps: distribution(gaps), gapsOver33: gaps.filter((g) => g > 33.4).length, gapsOver100: long.length,
            gapsOver250: gaps.filter((g) => g > 250).length, overdueMs: Math.round(gaps.reduce((sum, g) => sum + Math.max(0, g - 33.4), 0)),
            longMs: long.reduce((sum, g) => sum + g.gapMs, 0), longBusyMs: long.reduce((sum, g) => sum + g.busyMs, 0),
            longWaitedMs: long.reduce((sum, g) => sum + g.gapMs - g.busyMs, 0), longestWaitedMs: Math.max(0, ...long.map((g) => g.gapMs - g.busyMs)), long
        };
    };
    const result = {
        mode, page: "Home", scenario, seconds, browser: browser.product, angle: angle ?? process.env.FERROUI_BROWSER_ANGLE ?? "swiftshader",
        renderThread: onRenderThread, firstFrameMs, framesDrawn: Number(rendering.frames),
        renderingFrames: { thread: onRenderThread ? "worker" : "page", ...gapsOf(onRenderThread ? worker : threads.main.key) },
        // The animation frames of the page (the script of this measurement asks for one after another).
        pageFrames: gapsOf(threads.main.key),
        errors: page.errors
    };
    if (json) { console.log(JSON.stringify(result, null, 2)); return; }
    const line = (r) => `${r.animationFrames} animation frames, gaps ms: median ${r.gaps.median}, p95 ${r.gaps.p95}, max ${r.gaps.max}; ${r.gapsOver33} over 33 ms, ${r.gapsOver100} over 100 ms ` +
        `(${r.longMs} ms in all: the thread busy ${r.longBusyMs} ms, waiting ${r.longWaitedMs} ms, longest wait ${r.longestWaitedMs} ms): ${r.long.map((g) => `${g.gapMs} ms at ${g.afterMs} (busy ${g.busyMs})`).join(", ")}`;
    console.log(`${result.browser}, WebGL on ${result.angle}, drawn by ${onRenderThread ? "a render thread" : "the thread of the page"}: the first ${seconds} s of the page, ${mode}; first frame reported after ${firstFrameMs} ms, ${result.framesDrawn} frames drawn`);
    console.log(`  thread that renders (${result.renderingFrames.thread}): ${line(result.renderingFrames)}`);
    if (onRenderThread) { console.log(`  thread of the page: ${line(result.pageFrames)}`); }
    if (page.errors.length > 0) { console.log(`errors:\n${page.errors.join("\n")}`); }
}

// Every other scenario: the page is opened, driven and measured while it is driven.
async function measure() {
    await waitUntilReady(page, 180_000);
    drive(page);
    const centre = (e) => ({ x: Math.round(e.x + e.width / 2), y: Math.round(e.y + e.height / 2) });
    const nextFrame = () => page.evaluate("new Promise((resolve) => requestAnimationFrame(() => resolve(0)))");

    // The drawer lists the sections; a section lists its pages. Nothing is clicked before the view has
    // drawn it (catalog-pages.mjs).
    if (pageEntry !== "Home") { await visit(page, { section, name: pageEntry }, height); }
    if (clickText) { await page.clickElement(await page.find(clickText, inContent)); }
    await sleep(1000);
    const shown = await page.state();
    const content = shown.elements.filter((e) => e.hit && e.x >= DRAWER_EDGE && e.y > 48);
    const buttons = content.filter((e) => e.type === "Button" && e.y + e.height < height);
    const contentCentre = { x: Math.round((DRAWER_EDGE + width) / 2), y: Math.round((48 + height) / 2) };
    const entries = scenario === "navigate" ? [centre(await page.find(pageEntry, inDrawer)), centre(await page.find(other, inDrawer))] : [];

    // Positions the pointer and lets the page settle.
    const first = scenario === "hover" || scenario === "move" ? centre(buttons[0] ?? { x: contentCentre.x, y: contentCentre.y, width: 0, height: 0 }) : contentCentre;
    if ((scenario === "hover" || scenario === "move") && buttons.length === 0) { throw new Error("the page shows no button"); }
    if (scenario !== "navigate") { await page.mouseMove(first.x, first.y); }
    for (let i = 0; i < 30; i++) { await nextFrame(); }

    const browser = await page.browser();
    const workers = [...await page.attachWorkers()];
    for (const worker of workers) { await page.evaluateIn(worker.sessionId, recordAnimationFrames); }
    await page.evaluate(recordAnimationFrames);

    const metric = async () => Object.fromEntries((await page.send("Performance.getMetrics")).metrics.map((m) => [m.name, m.value]));
    await page.send("Performance.enable");
    const profiled = Boolean(cpuProfileFile) || top > 0;
    if (profiled) {
        await page.send("Profiler.enable");
        await page.send("Profiler.setSamplingInterval", { interval: 200 });
        await page.send("Profiler.start");
    }
    const stopTrace = trace ? await page.trace() : null;
    const before = await metric();
    const renderingBefore = await page.rendering();
    const firstFrame = await page.evaluate("globalThis.__ferroFrames.length");
    const firstBeat = await page.evaluate("globalThis.__ferroBeats.length");
    const firstLongTask = await page.evaluate("globalThis.__ferroLongTasks.length");
    const started = await page.evaluate("performance.now()");
    let events = 0;
    if (seconds !== null) {
        const end = Date.now() + seconds * 1000;
        if (scenario === "navigate") {
            // The first click is on the other page: the view shows --page.
            for (let i = 1; Date.now() < end; i++) {
                const next = Date.now() + 1200;
                await page.click(entries[i % 2].x, entries[i % 2].y);
                events++;
                await sleep(Math.max(0, Math.min(next, end) - Date.now()));
            }
        } else { await sleep(seconds * 1000); }
    } else {
        for (let i = 0; i < frames; i++) {
            if (scenario === "wheel") {
                const deltaY = i < frames / 2 ? delta : -delta;
                await page.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: contentCentre.x, y: contentCentre.y, deltaX: 0, deltaY, pointerType: "mouse" });
            } else if (scenario === "hover") {
                const p = centre(buttons[(i + 1) % buttons.length]);
                await page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: p.x, y: p.y, pointerType: "mouse" });
            } else if (scenario === "move") {
                await page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: first.x + (i % 2), y: first.y, pointerType: "mouse" });
            }
            await nextFrame();
        }
        events = frames;
    }
    const ended = await page.evaluate("performance.now()");
    const after = await metric();
    const renderingAfter = await page.rendering();
    const traced = stopTrace ? await stopTrace() : null;
    const times = await page.evaluate(`globalThis.__ferroFrames.slice(${firstFrame})`);
    const beats = await page.evaluate(`globalThis.__ferroBeats.slice(${firstBeat})`);
    const longTasks = await page.evaluate(`globalThis.__ferroLongTasks.slice(${firstLongTask})`);
    // The animation frame callbacks of the thread that renders: the worker with the most of them when
    // a render thread draws, the page otherwise.
    const onRenderThread = renderingAfter.on_render_thread === "true" && renderingAfter.other_thread === "true";
    const records = [];
    for (const worker of workers) { try { records.push(await page.evaluateIn(worker.sessionId, "globalThis.__ferroRaf")); } catch { } }
    const pageRecord = await page.evaluate("globalThis.__ferroRaf");
    const record = onRenderThread ? records.sort((a, b) => b.starts.length - a.starts.length)[0] ?? { starts: [], durations: [] } : pageRecord;
    // The callbacks that rendered a frame: the others return at once (nothing changed).
    const callbacks = pageRecord.durations.filter((d) => d >= 0.2).sort((a, b) => a - b);
    let profile;
    if (profiled) {
        profile = (await page.send("Profiler.stop")).profile;
        if (cpuProfileFile) { fs.writeFileSync(cpuProfileFile, JSON.stringify(profile)); }
    }

    if (screenshotFile) { await page.screenshot(screenshotFile); }
    const deltas = times.slice(1).map((t, i) => t - times[i]).sort((a, b) => a - b);
    const at = (q) => deltas[Math.min(deltas.length - 1, Math.round((deltas.length - 1) * q))];
    const elapsed = ended - started;
    const gaps = record.starts.slice(1).map((t, i) => t - record.starts[i]);
    const perEvent = Math.max(1, events);
    const result = {
        mode, context: await page.evaluate("globalThis.__ferroContextType ?? null"), page: pageEntry, scenario, events,
        browser: browser.product, angle: angle ?? process.env.FERROUI_BROWSER_ANGLE ?? "swiftshader", frameRateLimit: limited,
        renderThread: onRenderThread, elapsedMs: Number(elapsed.toFixed(0)),
        animationFrames: times.length,
        framesPerSecond: Number((times.length / (elapsed / 1000)).toFixed(1)),
        msPerFrame: { median: Number(at(0.5).toFixed(2)), mean: Number((deltas.reduce((a, b) => a + b, 0) / deltas.length).toFixed(2)), p95: Number(at(0.95).toFixed(2)), max: Number(at(1).toFixed(2)) },
        // Main-thread time of the page per input event (one frame of the application each), from the
        // performance metrics of the page: script, and all tasks (script, style, layout of the page,
        // compositing of the browser's own frame).
        scriptMsPerEvent: Number((((after.ScriptDuration - before.ScriptDuration) * 1000) / perEvent).toFixed(2)),
        taskMsPerEvent: Number((((after.TaskDuration - before.TaskDuration) * 1000) / perEvent).toFixed(2)),
        // The same for the whole measurement, as a share of its duration.
        taskShare: Number((((after.TaskDuration - before.TaskDuration) * 1000) / elapsed).toFixed(3)),
        // Animation frame callbacks of the page that took 0.2 ms or more (the frames it rendered).
        renderCallbacks: callbacks.length,
        renderMs: callbacks.length === 0 ? null : {
            median: Number(callbacks[Math.floor(callbacks.length / 2)].toFixed(2)),
            p95: Number(callbacks[Math.min(callbacks.length - 1, Math.round((callbacks.length - 1) * 0.95))].toFixed(2))
        },
        // The frames the view drew, whichever thread drew them.
        framesDrawn: Number(renderingAfter.frames) - Number(renderingBefore.frames),
        framesDrawnPerSecond: Number(((Number(renderingAfter.frames) - Number(renderingBefore.frames)) / (elapsed / 1000)).toFixed(1)),
        // The animation frames of the thread that renders: the gaps between them and the duration of
        // their callbacks.
        renderingFrames: {
            thread: onRenderThread ? "worker" : "page", gaps: distribution(gaps), callbacks: distribution(record.durations),
            gapsOver33: gaps.filter((g) => g > 33.4).length, gapsOver100: gaps.filter((g) => g > 100).length, gapsOver250: gaps.filter((g) => g > 250).length,
            // The time by which the gaps exceed two frames of 60 Hz, summed: how long the view stood still.
            overdueMs: Number(gaps.reduce((sum, g) => sum + Math.max(0, g - 33.4), 0).toFixed(0))
        },
        // The timer of the page that fires every 4 ms.
        heartbeat: {
            gaps: distribution(beats), over20: beats.filter((b) => b > 20).length, over54: beats.filter((b) => b > 54).length,
            blockedMs: Number(beats.reduce((sum, b) => sum + Math.max(0, b - 5), 0).toFixed(0))
        },
        longTasks: { count: longTasks.length, sum: Number(longTasks.reduce((a, b) => a + b, 0).toFixed(0)), max: longTasks.length ? Number(Math.max(...longTasks).toFixed(0)) : 0 },
        errors: page.errors
    };
    if (traced) {
        const threads = threadTimes(traced);
        const workerCpu = threads.workers.reduce((sum, thread) => sum + thread.cpuMs, 0);
        const span = traced.filter((e) => e.name === "RunTask").reduce((range, e) => [Math.min(range[0], e.ts), Math.max(range[1], e.ts)], [Infinity, -Infinity]);
        const tracedMs = (span[1] - span[0]) / 1000;
        result.trace = {
            ms: Number(tracedMs.toFixed(0)),
            mainCpuMs: Number(threads.main.cpuMs.toFixed(1)), mainWallMs: Number(threads.main.wallMs.toFixed(1)),
            mainBusyShare: Number((threads.main.wallMs / tracedMs).toFixed(3)),
            mainTasks: distribution(threads.main.durations),
            mainTasksOver8: threads.main.durations.filter((d) => d > 8).length,
            mainTasksOver16: threads.main.durations.filter((d) => d > 16.7).length, mainTasksOver50: threads.main.durations.filter((d) => d > 50).length,
            workerCpuMs: Number(workerCpu.toFixed(1)), workerTasks: distribution(threads.workers[0]?.durations ?? []),
            gpuMainCpuMs: Number(threads.gpu.cpuMs.toFixed(1))
        };
    }
    if (profile && top > 0) {
        // Self time per function name over the measured frames.
        const self = new Map();
        const byId = new Map(profile.nodes.map((n) => [n.id, n]));
        let total = 0;
        profile.samples.forEach((id, i) => {
            const node = byId.get(id); const dt = profile.timeDeltas[i] ?? 0; total += dt;
            const name = node.callFrame.functionName || `(${node.callFrame.url ? path.basename(node.callFrame.url) : "anonymous"})`;
            self.set(name, (self.get(name) ?? 0) + dt);
        });
        result.profileMs = Number((total / 1000).toFixed(1));
        result.top = [...self.entries()].sort((a, b) => b[1] - a[1]).slice(0, top)
            .map(([name, us]) => ({ name, ms: Number((us / 1000).toFixed(1)), share: Number(((us / total) * 100).toFixed(1)) }));
    }
    if (json) {
        console.log(JSON.stringify(result, null, 2));
    } else {
        const r = result.renderingFrames; const h = result.heartbeat;
        console.log(`${result.browser}, WebGL on ${result.angle}, ${limited ? "with" : "without"} the frame rate limit, drawn by ${onRenderThread ? "a render thread" : "the thread of the page"}`);
        console.log(`${result.page}, ${result.scenario}, ${result.mode} (${result.context}): ${result.animationFrames} animation frames for ${events} events in ${result.elapsedMs} ms, ` +
            `${result.framesPerSecond} frames per second; ms per frame: median ${result.msPerFrame.median}, mean ${result.msPerFrame.mean}, ` +
            `p95 ${result.msPerFrame.p95}, max ${result.msPerFrame.max}; main thread per event: script ${result.scriptMsPerEvent} ms, ` +
            `tasks ${result.taskMsPerEvent} ms (${(result.taskShare * 100).toFixed(1)} % of the time); ${result.renderCallbacks} rendering animation frame callbacks, ms: median ${result.renderMs?.median}, p95 ${result.renderMs?.p95}`);
        console.log(`  frames drawn: ${result.framesDrawn} (${result.framesDrawnPerSecond} per second); animation frames of the thread that renders (${r.thread}): ${r.gaps.count + 1}, ` +
            `gaps ms: median ${r.gaps.median}, p95 ${r.gaps.p95}, p99 ${r.gaps.p99}, max ${r.gaps.max}; ${r.gapsOver33} over 33 ms, ${r.gapsOver100} over 100 ms, ${r.gapsOver250} over 250 ms, ${r.overdueMs} ms overdue; ` +
            `callbacks ms: median ${r.callbacks.median}, p95 ${r.callbacks.p95}, max ${r.callbacks.max}`);
        console.log(`  thread of the page: 4 ms timer gaps ms: median ${h.gaps.median}, p95 ${h.gaps.p95}, p99 ${h.gaps.p99}, max ${h.gaps.max}; ${h.over20} over 20 ms, ${h.over54} over 54 ms, blocked ${h.blockedMs} ms; ` +
            `long tasks: ${result.longTasks.count} (${result.longTasks.sum} ms, longest ${result.longTasks.max} ms)`);
        if (result.trace) {
            const t = result.trace;
            console.log(`  trace of ${t.ms} ms: thread of the page ${t.mainCpuMs} ms of processor time, ${t.mainWallMs} ms in tasks (${(t.mainBusyShare * 100).toFixed(1)} %), tasks ms: median ${t.mainTasks.median}, p95 ${t.mainTasks.p95}, p99 ${t.mainTasks.p99}, max ${t.mainTasks.max}; ` +
                `${t.mainTasksOver8} over 8 ms, ${t.mainTasksOver16} over 16.7 ms, ${t.mainTasksOver50} over 50 ms; workers ${t.workerCpuMs} ms of processor time, longest task ${t.workerTasks.max ?? "-"} ms; GPU process main thread ${t.gpuMainCpuMs} ms`);
        }
        for (const entry of result.top ?? []) { console.log(`  ${entry.ms.toFixed(1).padStart(8)} ms ${entry.share.toFixed(1).padStart(5)} %  ${entry.name}`); }
        if (page.errors.length > 0) { console.log(`errors:\n${page.errors.join("\n")}`); }
    }
}

try {
    if (scenario === "start") { await start(); } else { await measure(); }
} finally {
    await page.close();
}
