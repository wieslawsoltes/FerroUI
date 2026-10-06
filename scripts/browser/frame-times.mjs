// Frame times of a page of the ControlCatalog site in headless Chromium, driven with real input events.
//
//   node scripts/browser/frame-times.mjs <site directory> [options]
//
//   --mode <mode>           rendering mode of the page (default WebGL2; Software2D)
//   --page <entry>          drawer entry of the page to open (default TableView); "Home" stays on the home page
//   --scenario <name>       wheel: a wheel event of --delta pixels each frame over the middle of the content,
//                           down for half of the frames and up for the other half (default);
//                           hover: the pointer moves to the next button of the page each frame;
//                           move: the pointer moves by one pixel each frame inside the first button;
//                           idle: nothing happens
//   --frames <n>            number of input events, one per animation frame (default 300)
//   --delta <px>            wheel delta in CSS pixels (default 20)
//   --size <w>x<h>          size of the viewport (default 1280x800)
//   --cpu-profile <file>    write a CPU profile (.cpuprofile) of the measured frames
//   --top <n>               print the n functions with the most self time of the CPU profile (default 25)
//   --screenshot <file>     write a PNG of the page after the measured frames
//   --json                  print the result as JSON
//
// The browser is started without its frame rate limit (`--disable-frame-rate-limit`,
// `--disable-gpu-vsync`), so that animation frames follow as fast as the page can produce them: the
// frame rate measures the cost of a frame, not the refresh rate of a display. A script added to the
// page records the time of every animation frame and the duration of the callbacks of the page;
// each input event is sent and the next animation
// frame awaited before the next one is sent, as a pointing device that reports once per frame does.
// The page-side cost per frame is read from the performance metrics of the page (script and task
// time of the main thread). WebGL runs on SwiftShader, which is a "major performance caveat", so the
// script lets a WebGL context be created although the page asks for failIfMajorPerformanceCaveat
// (as first-frame.mjs does); nothing else of the page is changed.
//
// For names of WebAssembly functions in the CPU profile, link the module with its name section
// (`-C link-arg=--profiling-funcs`, see docs/porting/browser-platform.md).
import fs from "node:fs";
import path from "node:path";
import { open, sleep } from "./harness.mjs";

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const valued = new Set(["--mode", "--page", "--scenario", "--frames", "--delta", "--size", "--cpu-profile", "--top", "--screenshot"]);
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/frame-times.mjs <site directory> [--mode m] [--page p] [--scenario wheel|hover|move|idle] [--frames n] [--delta px] [--size WxH] [--cpu-profile f] [--top n] [--json]");
    process.exit(2);
}
const mode = option("--mode", "WebGL2");
const pageEntry = option("--page", "TableView");
const scenario = option("--scenario", "wheel");
const frames = Number(option("--frames", "300"));
const delta = Number(option("--delta", "20"));
const [width, height] = option("--size", "1280x800").split("x").map(Number);
const cpuProfileFile = option("--cpu-profile");
const top = Number(option("--top", "25"));
const screenshotFile = option("--screenshot");
const json = args.includes("--json");
if (!["wheel", "hover", "move", "idle"].includes(scenario)) { console.error(`unknown scenario ${scenario}`); process.exit(2); }

const initScript = `(() => {
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, attrs, ...rest) {
        if (attrs && typeof attrs === "object" && attrs.failIfMajorPerformanceCaveat) attrs = { ...attrs, failIfMajorPerformanceCaveat: false };
        const context = getContext.call(this, type, attrs, ...rest);
        if (context) globalThis.__ferroContextType = type;
        return context;
    };
    // The time of every animation frame, and the duration of every animation frame callback of the
    // page itself (the render timer of the framework renders in them).
    const raf = requestAnimationFrame.bind(globalThis);
    globalThis.__ferroFrames = [];
    globalThis.__ferroCallbacks = [];
    globalThis.requestAnimationFrame = (callback) => raf((time) => {
        const start = performance.now();
        try { callback(time); } finally { globalThis.__ferroCallbacks.push(performance.now() - start); }
    });
    const frame = (time) => { globalThis.__ferroFrames.push(time); raf(frame); };
    raf(frame);
})();`;

const page = await open(site, {
    query: `?RenderingMode=${mode}`, width, height, initScript,
    chromeArgs: ["--disable-frame-rate-limit", "--disable-gpu-vsync", "--ignore-gpu-blocklist"]
});
try {
    await page.waitFor(`(() => {
        const canvas = document.querySelector("#out canvas");
        const splash = document.querySelector("#out .ferroui-splash");
        return canvas && canvas.width > 0 && (!splash || splash.classList.contains("splash-close"))
            && globalThis.controlCatalog && controlCatalog.catalogState() !== "null";
    })()`, 180_000);
    const state = async () => JSON.parse(await page.evaluate("controlCatalog.catalogState()"));
    const until = async (description, predicate, timeout = 30_000) => {
        let s;
        for (const end = Date.now() + timeout; Date.now() < end;) {
            s = await state();
            if (predicate(s)) { return s; }
            await sleep(100);
        }
        throw new Error(`timed out waiting until ${description}`);
    };
    const centre = (e) => ({ x: Math.round(e.x + e.width / 2), y: Math.round(e.y + e.height / 2) });
    const nextFrame = () => page.evaluate("new Promise((resolve) => requestAnimationFrame(() => resolve(0)))");

    // The drawer lists the sections; a section lists its pages.
    const drawerEdge = 260;
    const inDrawer = (e) => e.hit && e.x + e.width <= drawerEdge;
    // Clicks the entry `text` of the drawer and waits until the main view shows its page. The
    // navigation page ignores a navigation while it runs one (as upstream): wait until it has
    // finished and the title bar shows the header of the page.
    const navigate = async (text) => {
        const s = await until(`the entry ${text} is listed`, (x) => x.elements.some((e) => e.text === text && inDrawer(e)));
        const entry = s.elements.find((e) => e.text === text && inDrawer(e));
        await page.click(centre(entry).x, centre(entry).y);
        await until(`the page ${text} is shown`, (x) => x.page === text && !x.navigating
            && x.elements.some((e) => e.type === "TextBlock" && e.text === text && e.hit && e.y < 48 && e.x >= drawerEdge), 60_000);
    };
    if (pageEntry !== "Home") {
        const sections = { TableView: "Collections & Data", Buttons: "Basic Input", ScrollViewer: "Layout", TextBlock: "Text" };
        if (sections[pageEntry]) { await navigate(sections[pageEntry]); }
        await navigate(pageEntry);
    }
    await sleep(1000);
    const shown = await state();
    const content = shown.elements.filter((e) => e.hit && e.x >= drawerEdge && e.y > 48);
    const buttons = content.filter((e) => e.type === "Button" && e.y + e.height < height);
    const contentCentre = { x: Math.round((drawerEdge + width) / 2), y: Math.round((48 + height) / 2) };

    // Positions the pointer and lets the page settle.
    const first = scenario === "hover" || scenario === "move" ? centre(buttons[0] ?? { x: contentCentre.x, y: contentCentre.y, width: 0, height: 0 }) : contentCentre;
    if ((scenario === "hover" || scenario === "move") && buttons.length === 0) { throw new Error("the page shows no button"); }
    await page.mouseMove(first.x, first.y);
    for (let i = 0; i < 30; i++) { await nextFrame(); }

    const metric = async () => Object.fromEntries((await page.send("Performance.getMetrics")).metrics.map((m) => [m.name, m.value]));
    await page.send("Performance.enable");
    if (cpuProfileFile || top > 0) {
        await page.send("Profiler.enable");
        await page.send("Profiler.setSamplingInterval", { interval: 200 });
        await page.send("Profiler.start");
    }
    const before = await metric();
    const firstFrame = await page.evaluate("globalThis.__ferroFrames.length");
    const firstCallback = await page.evaluate("globalThis.__ferroCallbacks.length");
    const started = await page.evaluate("performance.now()");
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
    const ended = await page.evaluate("performance.now()");
    const after = await metric();
    const times = await page.evaluate(`globalThis.__ferroFrames.slice(${firstFrame})`);
    // The callbacks that rendered a frame: the others return at once (nothing changed).
    const callbacks = (await page.evaluate(`globalThis.__ferroCallbacks.slice(${firstCallback})`)).filter((d) => d >= 0.2).sort((a, b) => a - b);
    let profile;
    if (cpuProfileFile || top > 0) {
        profile = (await page.send("Profiler.stop")).profile;
        if (cpuProfileFile) { fs.writeFileSync(cpuProfileFile, JSON.stringify(profile)); }
    }

    if (screenshotFile) { await page.screenshot(screenshotFile); }
    const deltas = times.slice(1).map((t, i) => t - times[i]).sort((a, b) => a - b);
    const at = (q) => deltas[Math.min(deltas.length - 1, Math.round((deltas.length - 1) * q))];
    const elapsed = ended - started;
    const result = {
        mode, context: await page.evaluate("globalThis.__ferroContextType ?? null"), page: pageEntry, scenario, events: frames,
        animationFrames: times.length,
        framesPerSecond: Number((times.length / (elapsed / 1000)).toFixed(1)),
        msPerFrame: { median: Number(at(0.5).toFixed(2)), mean: Number((deltas.reduce((a, b) => a + b, 0) / deltas.length).toFixed(2)), p95: Number(at(0.95).toFixed(2)), max: Number(at(1).toFixed(2)) },
        // Main-thread time of the page per input event (one frame of the application each), from the
        // performance metrics of the page: script, and all tasks (script, style, layout of the page,
        // compositing of the browser's own frame).
        scriptMsPerEvent: Number((((after.ScriptDuration - before.ScriptDuration) * 1000) / frames).toFixed(2)),
        taskMsPerEvent: Number((((after.TaskDuration - before.TaskDuration) * 1000) / frames).toFixed(2)),
        // Animation frame callbacks of the page that took 0.2 ms or more (the frames it rendered).
        renderCallbacks: callbacks.length,
        renderMs: callbacks.length === 0 ? null : {
            median: Number(callbacks[Math.floor(callbacks.length / 2)].toFixed(2)),
            p95: Number(callbacks[Math.min(callbacks.length - 1, Math.round((callbacks.length - 1) * 0.95))].toFixed(2))
        },
        errors: page.errors
    };
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
        console.log(`${result.page}, ${result.scenario}, ${result.mode} (${result.context}): ${result.animationFrames} animation frames for ${frames} events, ` +
            `${result.framesPerSecond} frames per second; ms per frame: median ${result.msPerFrame.median}, mean ${result.msPerFrame.mean}, ` +
            `p95 ${result.msPerFrame.p95}, max ${result.msPerFrame.max}; main thread per event: script ${result.scriptMsPerEvent} ms, ` +
            `tasks ${result.taskMsPerEvent} ms; ${result.renderCallbacks} rendering animation frame callbacks, ms: median ${result.renderMs?.median}, p95 ${result.renderMs?.p95}`);
        for (const entry of result.top ?? []) { console.log(`  ${entry.ms.toFixed(1).padStart(8)} ms ${entry.share.toFixed(1).padStart(5)} %  ${entry.name}`); }
        if (page.errors.length > 0) { console.log(`errors:\n${page.errors.join("\n")}`); }
    }
} finally {
    await page.close();
}
