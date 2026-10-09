// Time to first frame of a browser site in headless Chromium, and a check that the page renders.
//
//   node scripts/browser/first-frame.mjs <site directory> [options]
//
//   --query <string>        query string of the page, e.g. "?RenderingMode=Software2D"
//   --runs <n>              number of page loads, each with a fresh browser profile (default 3)
//   --screenshot <file>     write a PNG of the page after the last load
//   --compare <file>        compare the page with a PNG written earlier; fails on any difference
//   --expect-mode <type>    fail unless the first canvas context is of this type (webgl2, 2d)
//   --encoding <br|gzip>    serve the precompressed .br or .gz file next to a file when it exists
//   --cpu-profile <file>    write a CPU profile (.cpuprofile) of the last load, up to the first frame
//   --throttle <Mbps>,<ms>  emulate a network: download and upload bandwidth in Mbit/s and round-trip
//                           latency in milliseconds (e.g. 50,40); serve compressed files (--encoding) so
//                           that the transferred bytes are those of the published site
//   --phases                also print the phases of each load (see below)
//   --isolated              serve the site cross-origin isolated (the two headers a site built with
//                           threads needs); without it such a site isolates itself through its service
//                           worker and reloads once, and the times are counted from the first navigation
//   --trace                 record a trace of the browser up to the first frame and report what the threads did
//                           (see below)
//   --angle <backend>       what WebGL runs on: swiftshader (the default, the software rasteriser) or a
//                           backend of ANGLE that uses the GPU of the machine (metal, d3d11, vulkan, gl)
//   --wasm-profile <file>   for a module instrumented with `wasm-split --instrument` (binaryen): write
//                           the profile of the functions that ran up to the first frame of the last
//                           load (input of `wasm-split --profile`, and of wasm-size-report.py --profile)
//   --json                  print the result as JSON
//
// The site (target/browser/<example> as scripts/build-browser.sh writes it) is served from a local
// HTTP server with the right media types (application/wasm, so that streaming compilation is used).
// A script added to every document records, relative to the start of navigation:
//   wasm:   the end of the response of the WebAssembly module (resource timing);
//   draw:   the first draw call of the framework (a WebGL draw call, or putImageData on a 2D canvas);
//   frame:  the first animation frame after that draw call, when the drawn frame is on screen;
//   splash: the moment the splash screen of the host page is closed;
//   rendered: the moment the module reports its first frame drawn (the `frames` of the export
//           `catalogRendering` or `themedViewRendering` of the host, asked every 4 ms by the thread of the
//           page). This is the first frame of a view drawn by a render thread, whose draw calls are made
//           in a worker and are not seen by the page: draw and frame are empty then. A site whose host has
//           neither export has no such time;
//   requests: the number of requests the page started before the first frame, the document included
//             (resource timing).
// With --phases, also: the end of the response of every file of the site (resource timing); when
// WebAssembly.instantiateStreaming was called, when the module was compiled and when it was
// instantiated (the call is replaced by compileStreaming and instantiate, which is what it does);
// and the performance marks the page sets (the catalog host marks the start and end of the calls
// into the module, see samples/ControlCatalog.Browser/wwwroot/main.js).
// Long tasks: the tasks of the thread of the page of 50 ms and more that the browser reports up to the
// first frame (their number, their sum and the longest): the time the page could not answer input.
// With --trace, from the tasks of a trace of the browser, from the start of the browser to the first
// frame: the processor time of the thread of the page, of the workers of the page (the threads of a
// module built with threads) and of the main thread of the GPU process, and the longest task of the
// thread of the page and of the workers.
// A page that reloads itself before it creates the module (a site built with threads that is served
// without the headers registers its service worker and reloads once) is followed over the reload: every
// time is counted from the start of the first navigation, and the phase "navigation of the load that ran
// the module" is where the last load started.
// The browser is Chromium (CHROME, or the Playwright build under /opt/pw-browsers); WebGL runs on
// SwiftShader unless --angle names another backend. SwiftShader is a "major performance caveat", so the
// script lets a WebGL context be created although the page asks for failIfMajorPerformanceCaveat; nothing
// else of the page is changed. No dependencies.
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import zlib from "node:zlib";
import { spawn } from "node:child_process";
import { rasteriserArgs, threadTimes } from "./harness.mjs";

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const valued = new Set(["--query", "--runs", "--screenshot", "--compare", "--expect-mode", "--encoding", "--cpu-profile", "--throttle", "--wasm-profile", "--angle"]);
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/first-frame.mjs <site directory> [--query q] [--runs n] [--screenshot f] [--compare f] [--expect-mode t] [--encoding br|gzip] [--cpu-profile f] [--wasm-profile f] [--isolated] [--angle a] [--phases] [--trace] [--json]");
    process.exit(2);
}
const query = option("--query", "");
const runs = Number(option("--runs", "3"));
const screenshotFile = option("--screenshot");
const compareFile = option("--compare");
const expectMode = option("--expect-mode");
const encoding = option("--encoding");
const cpuProfileFile = option("--cpu-profile");
const wasmProfileFile = option("--wasm-profile");
const json = args.includes("--json");
const phases = args.includes("--phases");
const isolated = args.includes("--isolated");
const trace = args.includes("--trace");
const angle = option("--angle");
const throttle = option("--throttle")?.split(",").map(Number);
if (throttle && (throttle.length !== 2 || throttle.some((v) => !(v >= 0)))) {
    console.error("--throttle takes <Mbit/s>,<latency ms>, e.g. 50,40");
    process.exit(2);
}

function findChrome() {
    const candidates = [process.env.CHROME];
    try {
        for (const d of fs.readdirSync("/opt/pw-browsers").filter((d) => d.startsWith("chromium-")).sort().reverse()) {
            candidates.push(path.join("/opt/pw-browsers", d, "chrome-linux", "chrome"));
        }
    } catch { }
    candidates.push("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser", "/usr/bin/chromium");
    return candidates.find((p) => p && fs.existsSync(p));
}
const chromePath = findChrome();
if (!chromePath) { console.error("no Chromium found: set CHROME"); process.exit(2); }

const types = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css",
    ".wasm": "application/wasm", ".map": "application/json", ".json": "application/json", ".png": "image/png" };
const served = new Map();
const server = http.createServer((req, res) => {
    const url = new URL(req.url, "http://localhost");
    const name = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname.slice(1));
    const file = path.resolve(site, name);
    if (!file.startsWith(path.resolve(site) + path.sep)) { res.statusCode = 403; res.end(); return; }
    if (!fs.existsSync(file) || !fs.statSync(file).isFile()) { res.statusCode = 404; res.end(); return; }
    res.setHeader("content-type", types[path.extname(file)] ?? "application/octet-stream");
    res.setHeader("cache-control", "no-store");
    if (isolated) {
        res.setHeader("cross-origin-opener-policy", "same-origin");
        res.setHeader("cross-origin-embedder-policy", "require-corp");
    }
    const accepted = String(req.headers["accept-encoding"] ?? "");
    const extension = { br: ".br", gzip: ".gz" }[encoding];
    if (extension && accepted.includes(encoding) && fs.existsSync(file + extension)) {
        res.setHeader("content-encoding", encoding);
        served.set(name, encoding);
        res.end(fs.readFileSync(file + extension));
        return;
    }
    served.set(name, "identity");
    res.end(fs.readFileSync(file));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;

const instrumentation = `(() => {
    const t = globalThis.__ferroTiming = { contexts: [], draw: null, frame: null, splash: null,
        compileStart: null, compiled: null, instantiated: null, rendered: null, rendering: null, offset: 0, pageLoads: 1, longTasks: [] };
    try { new PerformanceObserver((list) => { for (const entry of list.getEntries()) t.longTasks.push([entry.startTime, entry.duration]); }).observe({ type: "longtask" }); } catch { }
    // A page that reloads itself: the start of this load, counted from the start of the first one.
    try {
        const first = sessionStorage.getItem("__ferroFirstNavigation");
        if (first === null) { sessionStorage.setItem("__ferroFirstNavigation", String(performance.timeOrigin)); }
        else { t.offset = performance.timeOrigin - Number(first); }
        t.pageLoads = Number(sessionStorage.getItem("__ferroLoads") ?? "0") + 1;
        sessionStorage.setItem("__ferroLoads", String(t.pageLoads));
    } catch { }
    // The first frame the module reports, whichever thread drew it.
    const poll = () => {
        let line = null;
        try { line = globalThis.controlCatalog?.catalogRendering?.() ?? globalThis.themedView?.themedViewRendering?.() ?? null; } catch { }
        if (line !== null && Number(/(?:^|;)frames=(\\d+)/.exec(line)?.[1] ?? 0) > 0) { t.rendered = performance.now(); t.rendering = line; return; }
        setTimeout(poll, 1);
    };
    setTimeout(poll, 1);
    const wasm = WebAssembly;
    wasm.instantiateStreaming = async function (source, imports) {
        t.compileStart = performance.now();
        const module = await wasm.compileStreaming(source);
        t.compiled = performance.now();
        const instance = await wasm.instantiate(module, imports);
        t.instantiated = performance.now();
        return { module, instance };
    };
    const mark = () => {
        if (t.draw !== null) return;
        t.draw = performance.now();
        requestAnimationFrame(() => { t.frame = performance.now(); });
    };
    const getContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type, attrs, ...rest) {
        if (attrs && typeof attrs === "object" && attrs.failIfMajorPerformanceCaveat) attrs = { ...attrs, failIfMajorPerformanceCaveat: false };
        const context = getContext.call(this, type, attrs, ...rest);
        if (context && !context.__ferroTimed) {
            context.__ferroTimed = true;
            t.contexts.push(type);
            const names = type === "2d" ? ["putImageData"] : ["drawArrays", "drawElements", "drawArraysInstanced", "drawElementsInstanced", "drawRangeElements"];
            for (const name of names) {
                const f = context[name];
                if (typeof f === "function") context[name] = function (...a) { mark(); return f.apply(this, a); };
            }
        }
        return context;
    };
    new MutationObserver(() => {
        if (t.splash === null && document.querySelector(".splash-close")) t.splash = performance.now();
    }).observe(document, { subtree: true, attributes: true, attributeFilter: ["class"] });
})();`;

// With --wasm-profile: keeps the instance of the module, whose profile is read after the first frame.
const keepInstance = `(() => {
    for (const name of ["instantiateStreaming", "instantiate"]) {
        const original = WebAssembly[name];
        WebAssembly[name] = async function (...a) {
            const result = await original.apply(this, a);
            globalThis.__ferroInstance = result.instance ?? result;
            return result;
        };
    }
})();`;

// The profile of an instrumented module: its export __write_profile(address, size) writes it to the memory of the
// module and returns its size; it is written to fresh pages at the end of the memory.
const readWasmProfile = `(() => {
    const exports = globalThis.__ferroInstance?.exports;
    if (!exports?.__write_profile) return null;
    const memory = Object.values(exports).find((value) => value instanceof WebAssembly.Memory);
    const size = 16 * 1024 * 1024;
    const address = memory.grow(size / 65536) * 65536;
    const bytes = new Uint8Array(memory.buffer, address, exports.__write_profile(address, size));
    let text = "";
    for (let i = 0; i < bytes.length; i += 32768) text += String.fromCharCode.apply(null, bytes.subarray(i, i + 32768));
    return btoa(text);
})()`;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function load(takeScreenshot, profile_, wasmProfile) {
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-first-frame-"));
    const chrome = spawn(chromePath, ["--headless=new", "--no-first-run", "--no-default-browser-check", "--no-sandbox",
        ...rasteriserArgs(angle), "--ignore-gpu-blocklist",
        "--force-device-scale-factor=1", "--force-color-profile=srgb", "--hide-scrollbars",
        `--user-data-dir=${profile}`, "--remote-debugging-port=0", "--window-size=800,600", "about:blank"],
    { stdio: ["ignore", "ignore", "pipe"] });
    let chromeErrors = "";
    chrome.stderr.on("data", (chunk) => { chromeErrors = (chromeErrors + chunk).slice(-4000); });
    // The browser may still be writing to its profile while it ends: the removal is tried again.
    const finish = async () => { chrome.kill(); await sleep(300); fs.rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); };
    try {
        let debugPort;
        for (let i = 0; i < 600 && !debugPort; i++) {
            await sleep(100);
            try { debugPort = fs.readFileSync(path.join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]; } catch { }
        }
        if (!debugPort) throw new Error(`Chromium did not start within 60 s\n${chromeErrors}`);
        let pageTarget;
        for (let i = 0; i < 50 && !pageTarget; i++) {
            await sleep(100);
            try { pageTarget = (await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json()).find((t) => t.type === "page"); } catch { }
        }
        const ws = new WebSocket(pageTarget.webSocketDebuggerUrl);
        await new Promise((r) => ws.addEventListener("open", r));
        let id = 0; const pending = new Map(); const errors = []; const console_ = []; const traced = []; let traceComplete = () => { };
        ws.addEventListener("message", (ev) => {
            const m = JSON.parse(ev.data);
            if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result ?? { error: m.error }); pending.delete(m.id); } else if (m.method === "Runtime.exceptionThrown") {
                errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
            } else if (m.method === "Tracing.dataCollected") {
                traced.push(...m.params.value);
            } else if (m.method === "Tracing.tracingComplete") {
                traceComplete();
            } else if (m.method === "Runtime.consoleAPICalled") {
                const text = m.params.args.map((a) => a.value ?? a.description ?? "").join(" ");
                console_.push(`${m.params.type}: ${text}`);
                if (m.params.type === "error") errors.push(text);
            }
        });
        const send = (method, params = {}) => new Promise((r) => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
        const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result?.value;
        await send("Runtime.enable"); await send("Page.enable");
        await send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
        await send("Page.addScriptToEvaluateOnNewDocument", { source: instrumentation });
        if (throttle) {
            await send("Network.enable");
            const bytesPerSecond = (throttle[0] * 1_000_000) / 8;
            await send("Network.emulateNetworkConditions", { offline: false, latency: throttle[1],
                downloadThroughput: bytesPerSecond, uploadThroughput: bytesPerSecond });
        }
        if (wasmProfile) await send("Page.addScriptToEvaluateOnNewDocument", { source: keepInstance });
        if (trace) {
            await send("Tracing.start", { transferMode: "ReportEvents", traceConfig: {
                includedCategories: ["devtools.timeline", "disabled-by-default-devtools.timeline"], excludedCategories: ["*"] } });
        }
        if (profile_) {
            await send("Profiler.enable");
            await send("Profiler.setSamplingInterval", { interval: 1000 });
            await send("Profiler.start");
        }
        await send("Page.navigate", { url: `http://127.0.0.1:${port}/${query}` });

        let timing; let late = 0;
        for (let i = 0; i < 1200; i++) {
            await sleep(50);
            timing = await evaluate("globalThis.__ferroTiming && JSON.stringify(globalThis.__ferroTiming)");
            timing = timing ? JSON.parse(timing) : null;
            // A view drawn by a render thread makes no draw call the page sees: its first frame is the
            // one the module reports. A view drawn by the page has both, and the frame on screen is awaited.
            const drawn = timing?.frame != null || (timing?.rendered != null && timing.draw == null && /on_render_thread=true/.test(timing.rendering ?? ""));
            // The frame the module reports follows the first draw call of a view the page draws (a host
            // without the export never reports one: half a second is waited for it).
            const reported = timing?.rendered != null || (drawn && ++late > 10);
            if ((drawn && reported && timing?.splash != null) || errors.length > 0) break;
        }
        if (errors.length > 0) throw new Error(`the page reported errors:\n${errors.join("\n")}`);
        if (timing?.frame == null && timing?.rendered == null) throw new Error(`no frame within 60 s; console:\n${console_.join("\n")}`);
        const product = (await send("Browser.getVersion")).product;
        let threads = null;
        if (trace) {
            const complete = new Promise((resolve) => { traceComplete = resolve; });
            await send("Tracing.end");
            await complete;
            const times = threadTimes(traced);
            const longest = (thread) => (thread.durations.length ? Math.max(...thread.durations) : 0);
            threads = { mainCpuMs: times.main.cpuMs, mainWallMs: times.main.wallMs, mainLongestTaskMs: longest(times.main),
                workerCpuMs: times.workers.reduce((sum, thread) => sum + thread.cpuMs, 0), workerThreads: times.workers.length,
                workerLongestTaskMs: Math.max(0, ...times.workers.map(longest)), gpuMainCpuMs: times.gpu.cpuMs };
        }
        if (profile_) fs.writeFileSync(profile_, JSON.stringify((await send("Profiler.stop")).profile));
        if (wasmProfile) {
            const profile = await evaluate(readWasmProfile);
            if (!profile) throw new Error("the module has no __write_profile export: instrument it with `wasm-split --instrument`");
            fs.writeFileSync(wasmProfile, Buffer.from(profile, "base64"));
        }
        const wasm = await evaluate(`JSON.stringify(performance.getEntriesByType("resource").filter((e) => e.name.endsWith(".wasm")).map((e) => ({ end: e.responseEnd, transfer: e.transferSize, body: e.decodedBodySize })))`);
        const resources = JSON.parse(await evaluate(`JSON.stringify(Object.fromEntries([
            ["(document)", performance.getEntriesByType("navigation")[0]?.responseEnd],
            ...performance.getEntriesByType("resource").map((e) => [new URL(e.name).pathname.split("/").pop(), e.responseEnd])]))`));
        const marks = JSON.parse(await evaluate(`JSON.stringify(Object.fromEntries(performance.getEntriesByType("mark").map((m) => [m.name, m.startTime])))`));
        const requests = 1 + Number(await evaluate(`performance.getEntriesByType("resource").filter((e) => e.startTime < ${timing.frame ?? timing.rendered}).length`));
        // Counted from the start of the first navigation of a page that reloaded itself.
        const shift = (value) => (value == null ? value : value + timing.offset);
        const shifted = (object) => Object.fromEntries(Object.entries(object).map(([name, value]) => [name, shift(value)]));
        const reported = Object.fromEntries((timing.rendering ?? "").split(";").filter((pair) => pair.includes("=")).map((pair) => pair.split("=")));
        const module = JSON.parse(wasm)[0] ?? null;
        if (module) module.end = shift(module.end);
        // The long tasks that began before the first frame.
        const first = timing.rendered ?? timing.frame;
        const long = timing.longTasks.filter(([start]) => start < first).map(([, duration]) => duration);
        const longTasks = { count: long.length, sum: long.reduce((a, b) => a + b, 0), longest: Math.max(0, ...long) };
        const inWorker = reported.kind === "webgl" ? `webgl (OpenGL ES ${reported.gl}) in a worker` : reported.kind === "software" ? "2d in a worker" : undefined;
        const result = { requests, mode: timing.contexts[0] ?? inWorker, contexts: timing.contexts, wasm: module, product,
            draw: shift(timing.draw), frame: shift(timing.frame), splash: shift(timing.splash), rendered: shift(timing.rendered),
            framesReported: Number(reported.frames ?? 0), longTasks, threads, pageLoads: timing.pageLoads, renderThread: reported.on_render_thread === "true" && reported.other_thread === "true",
            phases: { ...shifted(resources), "compile start": shift(timing.compileStart), compiled: shift(timing.compiled),
                instantiated: shift(timing.instantiated), ...shifted(marks), "first draw": shift(timing.draw), "first frame": shift(timing.frame),
                "first frame reported by the module": shift(timing.rendered),
                ...(timing.pageLoads > 1 ? { "navigation of the load that ran the module": timing.offset } : {}) },
            streamingFailed: console_.some((l) => l.includes("wasm streaming compile failed")), console: console_ };

        if (takeScreenshot) {
            // Wait until two screenshots a second apart are the same: transitions of the theme are over.
            let previous = null; let shot = null;
            for (let i = 0; i < 20; i++) {
                await sleep(1000);
                shot = (await send("Page.captureScreenshot", { format: "png" })).data;
                if (shot === previous) break;
                previous = shot;
            }
            result.screenshot = Buffer.from(shot, "base64");
        }
        if (errors.length > 0) throw new Error(`the page reported errors:\n${errors.join("\n")}`);
        ws.close();
        return result;
    } finally {
        await finish();
    }
}

// Minimal PNG reader (8-bit RGB or RGBA, no interlace): the pixels as RGB.
function decodePng(buffer) {
    let offset = 8; let width = 0; let height = 0; let channels = 4; const idat = [];
    while (offset < buffer.length) {
        const length = buffer.readUInt32BE(offset); const type = buffer.toString("latin1", offset + 4, offset + 8);
        const body = buffer.subarray(offset + 8, offset + 8 + length);
        if (type === "IHDR") { width = body.readUInt32BE(0); height = body.readUInt32BE(4); channels = body[9] === 6 ? 4 : 3; }
        if (type === "IDAT") idat.push(body);
        offset += 12 + length;
    }
    const raw = zlib.inflateSync(Buffer.concat(idat)); const stride = width * channels; const pixels = Buffer.alloc(stride * height);
    for (let y = 0; y < height; y++) {
        const filter = raw[y * (stride + 1)];
        for (let x = 0; x < stride; x++) {
            const value = raw[y * (stride + 1) + 1 + x];
            const a = x >= channels ? pixels[y * stride + x - channels] : 0;
            const b = y > 0 ? pixels[(y - 1) * stride + x] : 0;
            const c = x >= channels && y > 0 ? pixels[(y - 1) * stride + x - channels] : 0;
            let predicted = 0;
            if (filter === 1) predicted = a; else if (filter === 2) predicted = b; else if (filter === 3) predicted = (a + b) >> 1;
            else if (filter === 4) { const p = a + b - c; const pa = Math.abs(p - a); const pb = Math.abs(p - b); const pc = Math.abs(p - c); predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
            pixels[y * stride + x] = (value + predicted) & 255;
        }
    }
    const rgb = Buffer.alloc(width * height * 3);
    for (let i = 0; i < width * height; i++) pixels.copy(rgb, i * 3, i * channels, i * channels + 3);
    return { width, height, rgb };
}

const results = [];
let exitCode = 0;
try {
    for (let i = 0; i < runs; i++) results.push(await load(i === runs - 1 && (screenshotFile || compareFile), i === runs - 1 ? cpuProfileFile : undefined,
        i === runs - 1 ? wasmProfileFile : undefined));
} catch (e) {
    console.error(String(e.message ?? e));
    server.close();
    process.exit(1);
}
server.close();

const last = results[results.length - 1];
const report = { site, query, runs, chromium: chromePath, product: last.product, angle: angle ?? process.env.FERROUI_BROWSER_ANGLE ?? "swiftshader",
    isolated, served: Object.fromEntries(served), mode: last.mode, renderThread: last.renderThread, pageLoads: last.pageLoads,
    streaming: !results.some((r) => r.streamingFailed) };
const median = (key) => { const v = results.map(key).filter((x) => x != null).sort((a, b) => a - b); return v.length ? v[Math.floor(v.length / 2)] : null; };
report.median = { wasm: median((r) => r.wasm?.end), draw: median((r) => r.draw), frame: median((r) => r.frame), splash: median((r) => r.splash),
    rendered: median((r) => r.rendered), requests: median((r) => r.requests) };
report.loads = results.map((r) => ({ wasm: r.wasm?.end, draw: r.draw, frame: r.frame, splash: r.splash, rendered: r.rendered, requests: r.requests, mode: r.mode,
    pageLoads: r.pageLoads, renderThread: r.renderThread, framesReported: r.framesReported, longTasks: r.longTasks, ...(r.threads ? { threads: r.threads } : {}),
    ...(phases ? { phases: r.phases } : {}) }));
if (phases) {
    const names = [...new Set(results.flatMap((r) => Object.keys(r.phases)))].filter((n) => results.some((r) => r.phases[n] != null));
    report.medianPhases = Object.fromEntries(names.map((n) => [n, median((r) => r.phases[n])]))
}
if (throttle) report.throttle = { mbps: throttle[0], latency: throttle[1] };

if (expectMode && last.mode !== expectMode) {
    report.failure = `the page rendered with a ${last.mode} context, expected ${expectMode}`;
    exitCode = 1;
}
if (last.screenshot) {
    const picture = decodePng(last.screenshot);
    const colours = new Set();
    for (let i = 0; i < picture.rgb.length; i += 3 * 7) colours.add(picture.rgb.readUIntBE(i, 3));
    report.distinctColours = colours.size;
    if (colours.size < 16) { report.failure = `the page looks empty (${colours.size} colours)`; exitCode = 1; }
    if (screenshotFile) fs.writeFileSync(screenshotFile, last.screenshot);
    if (compareFile) {
        const reference = decodePng(fs.readFileSync(compareFile));
        let differing = 0;
        report.size = `${picture.width}x${picture.height}`;
        if (reference.width !== picture.width || reference.height !== picture.height) differing = -1;
        else for (let i = 0; i < picture.rgb.length; i += 3) if (picture.rgb.compare(reference.rgb, i, i + 3, i, i + 3) !== 0) differing++;
        report.differingPixels = differing;
        if (differing !== 0) { report.failure = `the page differs from ${compareFile} in ${differing < 0 ? "size" : `${differing} pixels`}`; exitCode = 1; }
    }
}

if (json) {
    console.log(JSON.stringify(report, null, 2));
} else {
    const ms = (v) => (v == null ? "-" : `${Math.round(v)} ms`);
    console.log(`${report.product}, WebGL on ${report.angle}${isolated ? ", served cross-origin isolated" : ""}${report.renderThread ? ", drawn by a render thread" : ""}${report.pageLoads > 1 ? `, the page loaded ${report.pageLoads} times` : ""}`);
    console.log(`${site}${query}: ${runs} loads${throttle ? ` at ${throttle[0]} Mbit/s and ${throttle[1]} ms` : ""}, ${report.mode} context, streaming compilation ${report.streaming ? "used" : "FAILED"}, served ${JSON.stringify(report.served)}`);
    report.loads.forEach((l, i) => console.log(`  load ${i + 1}: wasm ${ms(l.wasm)}, first draw ${ms(l.draw)}, first frame ${ms(l.frame)}, first frame reported by the module ${ms(l.rendered)}, splash closed ${ms(l.splash)}, ${l.requests} requests before the first frame`));
    console.log(`  long tasks of the page before the first frame (median): ${Math.round(median((r) => r.longTasks.count))}, ${ms(median((r) => r.longTasks.sum))} in all, longest ${ms(median((r) => r.longTasks.longest))}`);
    if (trace) {
        const m = (key) => ms(median((r) => r.threads[key]));
        console.log(`  threads up to the first frame (median): thread of the page ${m("mainCpuMs")} of processor time, longest task ${m("mainLongestTaskMs")}; workers ${m("workerCpuMs")}, longest task ${m("workerLongestTaskMs")}; main thread of the GPU process ${m("gpuMainCpuMs")}`);
    }
    console.log(`  median: wasm ${ms(report.median.wasm)}, first draw ${ms(report.median.draw)}, first frame ${ms(report.median.frame)}, first frame reported by the module ${ms(report.median.rendered)}, splash closed ${ms(report.median.splash)}, ${report.median.requests} requests before the first frame`);
    if (report.medianPhases) {
        console.log("  phases (median of the loads, from the start of navigation):");
        for (const [n, v] of Object.entries(report.medianPhases).sort((a, b) => a[1] - b[1])) console.log(`    ${ms(v).padStart(9)}  ${n}`);
    }
    if (report.distinctColours != null) console.log(`  screenshot: ${report.distinctColours} colours${report.differingPixels != null ? `, ${report.differingPixels} pixels differ from ${compareFile}` : ""}`);
    if (report.failure) console.log(`FAIL ${report.failure}`);
}
process.exit(exitCode);
