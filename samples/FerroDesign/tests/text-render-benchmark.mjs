// Usage (run from samples/FerroDesign):
//   FERRODESIGN_DIST=/path/to/build FERRODESIGN_CHROMIUM=/path/to/chromium \
//   FERRODESIGN_REPORT=/path/to/report node tests/text-render-benchmark.mjs
// Optional FERRODESIGN_BASELINE_DIST runs the identical fixture against both
// builds in one browser launch. Labels are recorded from FERRODESIGN_BENCHMARK_LABEL and
// FERRODESIGN_BASELINE_LABEL. No renderer, document, or camera test command is
// invoked: JSON imports are validated normally; real middle-button input pans.
// FERRODESIGN_TRACE_TEXT_PAN=1 adds a separate six-motion Chromium diagnostic
// after ordinary measurements. Its trace-enabled timings never enter the 36-sample statistics.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { startInputObservation, stopInputObservation } from "./input-observations.mjs";
import { boundedTextPanOperation, createTextPanBrowserOwner, runTextPanTraceDiagnostic, TEXT_PAN_TRACE_LIMITS } from "./text-pan-trace.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(process.env.FERRODESIGN_REPORT || resolve(root, "text-benchmark-report"));
const builds = [
    ...(process.env.FERRODESIGN_BASELINE_DIST ? [{ id: "baseline", path: resolve(process.env.FERRODESIGN_BASELINE_DIST),
        label: process.env.FERRODESIGN_BASELINE_LABEL || "baseline" }] : []),
    { id: "current", path: resolve(process.env.FERRODESIGN_DIST || resolve(root, "dist")),
        label: process.env.FERRODESIGN_BENCHMARK_LABEL || "current" },
];
const viewport = { width: 1600, height: 1100 };
const warmupCount = 12, measuredCount = 36, seed = 0x4f726269;
const traceRequested = process.env.FERRODESIGN_TRACE_TEXT_PAN === "1";
const browserTransport = traceRequested ? "owned-playwright-server-websocket" : "playwright-in-process";
const motions = [[4, 0], [4, 4], [0, 4], [-4, 4], [-4, 0], [-4, -4], [0, -4], [4, -4], [0, 0]];
const totalCounters = ["totalFullPrepares", "totalIncrementalPrepares", "totalPreparedNodes", "totalRebuiltVisuals",
    "layerProjectionBuilds", "layerProjectionReuses"];
const frameCounters = ["visibleNodes", "culledNodes", "cachedNodes", "preparedNodes", "rebuiltVisuals",
    "fullPrepares", "incrementalPrepares", "rootBoundsTests", "rootCandidates", "rootQueryReused"];
const hash = (value) => createHash("sha256").update(value).digest("hex");
const state = (page) => page.evaluate(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()));
const frameState = (page) => page.evaluate(() => JSON.parse((globalThis.ferroDesign.ferroDesignFrameState ||
    globalThis.ferroDesign.ferroDesignState)()));
const frames = (page) => page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
const pick = (value, keys) => Object.fromEntries(keys.map(key => [key, value[key] ?? null]));
const delta = (before, after) => Object.fromEntries(totalCounters.map(key => [key,
    Number.isFinite(before[key]) && Number.isFinite(after[key]) ? after[key] - before[key] : null]));
function statistics(values) {
    const sorted = [...values].sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return { count: sorted.length, medianMs: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
        p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1], minMs: sorted[0], maxMs: sorted.at(-1) };
}

// The standalone form of browser.mjs's validated fixture import/wait helpers.
// Minimal Node fields deliberately use the model's serde defaults, so the exact
// seeded input bytes stay the same across baseline/current builds.
function fixture(schemaVersion) {
    let random = seed;
    const next = () => { random = (Math.imul(random, 1664525) + 1013904223) >>> 0; return random; };
    const words = ["Orbit", "Curves", "Round", "Object", "Canvas", "Motion"];
    const nodes = {};
    for (let index = 0; index < 150; index++) {
        const id = index + 2, style = index % 4;
        const text = `${words[next() % words.length]} oOo Qq AV ${String(index).padStart(3, "0")} curves`;
        nodes[id] = {
            id, parent: 1, children: [], kind: "text", name: `Text benchmark ${String(index).padStart(3, "0")}`,
            x: 20 + (index % 5) * 240 + (next() % 4) * 0.25 + 0.125,
            y: 20 + Math.floor(index / 5) * 26 + (next() % 4) * 0.25 + 0.125,
            width: 228, height: 23, visible: true, locked: false, opacity: 1,
            fills: [{ paint: { type: "solid", r: 25, g: 31, b: 43, a: 255 }, opacity: 1, visible: true }],
            strokes: [], effects: [], text, font_family: "Inter", font_size: 12.75,
            font_weight: style & 1 ? 700 : 400, italic: Boolean(style & 2),
            line_height: 1.2, letter_spacing: 0, text_align: "left", text_vertical_align: "top",
            text_auto_resize: "width_and_height", text_decoration: "none", text_runs: [],
            font_features: [{ tag: "liga", value: 0 }, { tag: "clig", value: 0 }, { tag: "calt", value: 0 }],
        };
    }
    return { schema_version: schemaVersion, name: "Seeded150LabelTextBenchmark", nodes,
        pages: [{ id: 1, name: "Visible text workload", children: Object.values(nodes).map(node => node.id),
            background: { r: 255, g: 255, b: 255, a: 255 }, prototype_start: null }],
        styles: {}, variables: {}, variable_mode: "Default", comments: [], guides: {}, next_id: 152 };
}

async function importFixture(page, document) {
    const before = await state(page);
    assert.equal(await page.evaluate(json => globalThis.ferroDesign.ferroDesignImportJson(json), JSON.stringify(document)), true,
        "The normal native document import must accept the benchmark fixture");
    await page.waitForFunction(({ after, count }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.nodeCount === count && view.visibleNodes === count && view.renderedFrames > after;
    }, { after: before.renderedFrames, count: Object.keys(document.nodes).length }, { timeout: 120000 });
    // Drain the import's ordinary serialization/storage work before measuring.
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).serializationPending,
        undefined, { timeout: 30000 });
    await page.evaluate(async () => { await globalThis.ferroDesignStorage?.flush(); });
    await frames(page);
}

async function identifyBuild(build) {
    const names = (await readdir(build.path)).filter(name => /\.(?:wasm|data|js|html)$/.test(name)).sort();
    const files = [];
    for (const name of names) {
        const path = resolve(build.path, name), info = await stat(path);
        if (!info.isFile()) continue;
        const digest = createHash("sha256");
        for await (const chunk of createReadStream(path)) digest.update(chunk);
        files.push({ name, bytes: info.size, sha256: digest.digest("hex") });
    }
    return { ...build, files, fingerprint: hash(JSON.stringify(files)) };
}

const report = {
    schemaVersion: 2, created: new Date().toISOString(), benchmark: "native-text-pan-v2", viewport, seed,
    warmupCount, measuredCount, motions, builds: [], results: [], traceDiagnosticRequested: traceRequested,
    browserTransport,
    method: "Host input dispatch to observed native renderedFrames advance at the expected camera. Record dispatch, frame observation, native presentation and two subsequent animation callbacks separately. Timed polls use the constant-size frame bridge; trusted browser input and long tasks are bounded read-only observations.",
    limitations: [
        "Primary timings still include Playwright dispatch, compact bridge serialization and browser scheduling; they are not isolated renderer timings. Native presentation is the public FerroUI surface counter, not a GPU completion timestamp. Settling includes two subsequent animation callbacks.",
        "Browser event timestamps and performance/long-task observations use the page clock; host phase durations use the Node clock. They are reported separately and must not be subtracted across clock origins.",
        "An explicitly requested historical baseline without the compact frame bridge is marked legacy; timing comparisons across different bridge methods are suppressed.",
        "Native label visibility/preparation counters are observed. The bridge does not expose actual shaped/drawn glyph totals; exact input character counts describe the workload instead.",
        "The WebGL2 launch requests SwiftShader; the actual renderer string is recorded when available. This is not a hardware-GPU throughput measurement. No speed threshold or FPS assertion is made.",
        "Baseline/current order is sequential and not randomized; machine load and thermal/order effects can affect a paired comparison.",
        "If enabled, a separately labelled trace cohort follows each mode's ordinary measurements and adds between-run work; compare runs with matching diagnostic configuration.",
        "Trace-enabled runs use an owned Playwright BrowserServer and WebSocket connection for bounded process termination. Trace-off runs retain chromium.launch(); dispatch observations from different transport/trace configurations are not a transport-identical comparison.",
    ],
};
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png", ".svg": "image/svg+xml" };
const server = createServer(async (request, response) => {
    try {
        const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
        const match = builds.find(build => pathname.startsWith("/" + build.id + "/"));
        if (!match) { response.writeHead(404).end(); return; }
        let file = resolve(match.path, "." + pathname.slice(match.id.length + 1));
        if (file !== match.path && !file.startsWith(match.path + sep)) { response.writeHead(403).end(); return; }
        if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
        response.writeHead(200, { "content-type": types[extname(file)] || "application/octet-stream", "cache-control": "no-store" });
        createReadStream(file).on("error", () => response.destroy()).pipe(response);
    } catch { response.writeHead(404).end(); }
});
let browser, browserOwner, fixtureDocument, fixtureJson;
await mkdir(output, { recursive: true });
try {
    for (const build of builds) report.builds.push(await identifyBuild(build));
    await new Promise((done, reject) => { server.once("error", reject); server.listen(0, "127.0.0.1", done); });
    const base = "http://127.0.0.1:" + server.address().port;
    const launchOptions = { headless: true, executablePath: process.env.FERRODESIGN_CHROMIUM || undefined,
        args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
    if (traceRequested) {
        const ownedServer = await chromium.launchServer({ ...launchOptions, host: "127.0.0.1" });
        browserOwner = createTextPanBrowserOwner(ownedServer);
        browser = await chromium.connect(ownedServer.wsEndpoint(), { timeout: 30000 });
    } else browser = await chromium.launch(launchOptions);
    report.browser = browser.version();
    benchmarkRuns: for (const build of builds) for (const mode of ["Software2D", "WebGL2"]) {
        const result = { build: build.id, label: build.label, requestedMode: mode, status: "running", pageErrors: [], consoleErrors: [], samples: [],
            browserTransport, traceDiagnosticRequested: traceRequested };
        report.results.push(result);
        const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
        const page = await context.newPage();
        const normalPageError = error => result.pageErrors.push(error.stack || String(error));
        const normalConsoleError = message => { if (message.type() === "error") result.consoleErrors.push(message.text()); };
        page.on("pageerror", normalPageError);
        page.on("console", normalConsoleError);
        try {
            await page.goto(base + "/" + build.id + "/?RenderingMode=" + mode, { waitUntil: "domcontentloaded", timeout: 120000 });
            await page.waitForFunction(() => globalThis.ferroDesignReady || globalThis.ferroDesignErrors?.length,
                undefined, { timeout: 120000 });
            assert.deepEqual(await page.evaluate(() => globalThis.ferroDesignErrors || []), []);
            await page.waitForFunction(() => {
                const view = JSON.parse(globalThis.ferroDesign?.ferroDesignState?.() || "null");
                return view?.renderedFrames > 0 && view.nodeCount > 0;
            }, undefined, { timeout: 120000 });
            await page.locator(".ferroui-splash").waitFor({ state: "hidden", timeout: 120000 });
            result.environment = await page.evaluate(() => {
                const canvas = document.querySelector("canvas.ferroui-canvas");
                const gl = canvas?.getContext("webgl2"), debug = gl?.getExtension("WEBGL_debug_renderer_info");
                return { userAgent: navigator.userAgent, hardwareConcurrency: navigator.hardwareConcurrency,
                    devicePixelRatio, canvas: { width: canvas?.width, height: canvas?.height },
                    surface: gl ? "WebGL2" : canvas?.getContext("2d") ? "Software2D" : "unknown",
                    device: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null };
            });
            assert.equal(result.environment.surface, mode, "The requested rendering surface must be observed");
            if (!fixtureDocument) {
                const initial = await page.evaluate(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()));
                fixtureDocument = fixture(initial.schema_version);
                fixtureJson = JSON.stringify(fixtureDocument);
                const labels = Object.values(fixtureDocument.nodes);
                report.workload = { inputSha256: hash(fixtureJson), labels: labels.length,
                    inputCodePoints: labels.reduce((sum, node) => sum + [...node.text].length, 0),
                    inputNonSpaceCodePoints: labels.reduce((sum, node) => sum + [...node.text.replaceAll(" ", "")].length, 0),
                    actualRenderedGlyphCount: null, actualRenderedGlyphCountReason: "Not exposed by the native bridge; no glyph count is inferred from characters.",
                    font: "Inter", fontSize: 12.75, styles: ["regular", "bold", "italic", "bold italic"],
                    wrapping: "disabled", ligatures: "liga/clig/calt disabled", columns: 5, rows: 30 };
                await writeFile(resolve(output, "fixture.json"), fixtureJson);
            }
            await importFixture(page, fixtureDocument);
            const prepared = await state(page);
            assert.equal(prepared.renderer, "FerroUI Vello");
            assert.equal(prepared.visibleNodes, 150);
            for (const key of ["invalidNodes", "unsupportedEffects", "missingImages"]) assert.equal(prepared[key], 0, key);
            result.imported = { renderedFrames: prepared.renderedFrames, zoom: prepared.zoom, pan: prepared.pan,
                canvas: prepared.chrome.CanvasHost, counters: pick(prepared, [...frameCounters, ...totalCounters]) };
            result.metricBridge = await page.evaluate(() => ({
                kind: typeof globalThis.ferroDesign.ferroDesignFrameState === "function" ? "compact-v1" : "full-state-legacy",
            }));
            if (build.id === "current") assert.equal(result.metricBridge.kind, "compact-v1",
                "The current build must expose the compact, read-only frame sampler");
            const compact = await frameState(page);
            for (const key of ["documentRevision", "zoom", "pan", "visibleNodes", "invalidNodes"])
                assert.deepEqual(compact[key], prepared[key], `Compact metrics must agree with full state for ${key}`);
            if (result.metricBridge.kind === "compact-v1") assert.ok(Number.isSafeInteger(compact.presentedFrames),
                "The browser sampler must expose FerroUI's actual presentation counter");
            const canonical = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            result.documentSha256 = hash(canonical);
            await page.screenshot({ path: resolve(output, build.id + "-" + mode + "-text.png"), timeout: 60000 });
            const canvas = prepared.chrome.CanvasHost;
            const pointer = { x: Math.floor(canvas.x + canvas.width / 2), y: Math.floor(canvas.y + canvas.height / 2) };
            await page.mouse.move(pointer.x, pointer.y);
            await page.mouse.down({ button: "middle" });
            await frames(page);
            const original = await frameState(page);
            await startInputObservation(page, ["pointermove"], 256);
            const panStep = async (index) => {
                const [dx, dy] = motions[index % motions.length];
                const before = await page.evaluate(() => {
                    const readStarted = performance.now();
                    const raw = (globalThis.ferroDesign.ferroDesignFrameState || globalThis.ferroDesign.ferroDesignState)();
                    const view = JSON.parse(raw);
                    globalThis.__ferroDesignPanObservation = {
                        startedAtMs: performance.now(), initialReadMs: performance.now() - readStarted,
                        pollCount: 0, pollReadMs: 0, maxPollReadMs: 0, maxPayloadCharacters: raw.length,
                    };
                    return view;
                });
                const expected = { x: original.pan.x + dx, y: original.pan.y + dy };
                const started = performance.now();
                await page.mouse.move(pointer.x + dx, pointer.y + dy, { steps: 1 });
                const dispatched = performance.now();
                const observedHandle = await page.waitForFunction(({ after, x, y }) => {
                    const readStarted = performance.now();
                    const raw = (globalThis.ferroDesign.ferroDesignFrameState || globalThis.ferroDesign.ferroDesignState)();
                    const view = JSON.parse(raw), readMs = performance.now() - readStarted;
                    const probe = globalThis.__ferroDesignPanObservation;
                    probe.pollCount++; probe.pollReadMs += readMs;
                    probe.maxPollReadMs = Math.max(probe.maxPollReadMs, readMs);
                    probe.maxPayloadCharacters = Math.max(probe.maxPayloadCharacters, raw.length);
                    if (view.renderedFrames <= after || Math.abs(view.pan.x - x) >= 0.01 || Math.abs(view.pan.y - y) >= 0.01) return false;
                    probe.frameObservedAtMs = performance.now();
                    return view;
                }, { after: before.renderedFrames, ...expected }, { timeout: 30000 });
                const durationMs = performance.now() - started;
                const observed = await observedHandle.jsonValue();
                await observedHandle.dispose();
                let presentationMs = null;
                if (Number.isSafeInteger(before.presentedFrames)) {
                    if (observed.presentedFrames <= before.presentedFrames) {
                        // This is a distinct real outcome, using the remainder
                        // of the original observation budget, never a retry.
                        await page.waitForFunction((after) => JSON.parse(globalThis.ferroDesign.ferroDesignFrameState()).presentedFrames > after,
                            before.presentedFrames, { timeout: Math.max(1, 30000 - (performance.now() - dispatched)) });
                    }
                    presentationMs = performance.now() - started;
                }
                await frames(page);
                const settledObservationMs = performance.now() - started, after = await frameState(page);
                const browserObservation = await page.evaluate(() => {
                    const result = { ...globalThis.__ferroDesignPanObservation, endedAtMs: performance.now() };
                    delete globalThis.__ferroDesignPanObservation;
                    return result;
                });
                assert.equal(after.documentRevision, original.documentRevision, "Panning must not edit the document");
                assert.equal(after.visibleNodes, 150, "Every text label must remain visible throughout the workload");
                assert.equal(after.invalidNodes, 0);
                return { index, durationMs, dispatchMs: dispatched - started, frameObservationMs: durationMs - (dispatched - started),
                    presentationMs, settledObservationMs, browserObservation, pointerDelta: { x: dx, y: dy },
                    frameDelta: after.renderedFrames - before.renderedFrames,
                    presentedFrameDelta: Number.isSafeInteger(before.presentedFrames) ? after.presentedFrames - before.presentedFrames : null,
                    zoom: after.zoom, pan: after.pan, preparationDelta: delta(before, after), frameCounters: pick(after, frameCounters) };
            };
            try {
                const warmStart = performance.now();
                for (let index = 0; index < warmupCount; index++) await panStep(index);
                result.warmupMs = performance.now() - warmStart;
                const before = await frameState(page);
                for (let index = 0; index < measuredCount; index++) result.samples.push(await panStep(warmupCount + index));
                const after = await frameState(page);
                result.timing = statistics(result.samples.map(sample => sample.durationMs));
                result.settlingTiming = statistics(result.samples.map(sample => sample.settledObservationMs));
                result.dispatchTiming = statistics(result.samples.map(sample => sample.dispatchMs));
                result.frameObservationTiming = statistics(result.samples.map(sample => sample.frameObservationMs));
                if (result.samples.every(sample => sample.presentationMs !== null))
                    result.presentationTiming = statistics(result.samples.map(sample => sample.presentationMs));
                result.preparationDelta = delta(before, after);
                result.nativeFrameDelta = after.renderedFrames - before.renderedFrames;
            } finally {
                result.inputObservation = await stopInputObservation(page);
                result.lastIncompleteFrameObservation = await page.evaluate(() => {
                    const probe = globalThis.__ferroDesignPanObservation ?? null;
                    delete globalThis.__ferroDesignPanObservation;
                    return probe;
                });
                await page.mouse.up({ button: "middle" });
            }
            assert.equal(result.inputObservation.droppedEvents, 0, "The bounded input record must retain every measured motion");
            for (const sample of result.samples) {
                assert.ok(result.inputObservation.events.some(event => event.type === "pointermove" &&
                    event.phase === "capture" && event.trusted &&
                    event.observedAtMs >= sample.browserObservation.startedAtMs &&
                    event.observedAtMs <= sample.browserObservation.endedAtMs &&
                    event.x === pointer.x + sample.pointerDelta.x && event.y === pointer.y + sample.pointerDelta.y),
                    "Each timed sample must contain its actual trusted pointer motion");
                if (result.metricBridge.kind === "compact-v1") {
                    assert.ok(sample.browserObservation.maxPayloadCharacters <= 4096,
                        "The timed bridge must remain a small scalar projection");
                    assert.ok(sample.presentedFrameDelta > 0, "Every motion must reach native surface presentation");
                }
            }
            const fullAfter = await state(page), compactAfter = await frameState(page);
            for (const key of ["documentRevision", "zoom", "pan", "visibleNodes", "invalidNodes"])
                assert.deepEqual(compactAfter[key], fullAfter[key], `Compact final metrics must agree for ${key}`);
            assert.equal(hash(await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson())), result.documentSha256);
            result.renderPanics = await page.evaluate(() => globalThis.ferroDesignRenderPanics?.() || []);
            assert.deepEqual(result.renderPanics, []);
            assert.deepEqual(result.pageErrors, []);
            assert.deepEqual(result.consoleErrors, []);
            result.status = "passed";
            console.log(`${build.label} ${mode}: ${result.timing.medianMs.toFixed(2)} ms median / ${result.timing.p95Ms.toFixed(2)} ms p95 (${measuredCount} native pan observations)`);
            console.log(`${build.label} ${mode}: dispatch p95 ${result.dispatchTiming.p95Ms.toFixed(2)} ms; frame-observation p95 ${result.frameObservationTiming.p95Ms.toFixed(2)} ms; native-presentation p95 ${result.presentationTiming?.p95Ms.toFixed(2) ?? "unavailable"} ms (${result.metricBridge.kind})`);
            // Keep decisive timing evidence in decoded CI logs as well as the
            // full artifact; downloading an artifact is not always available.
            console.log(JSON.stringify({ benchmark: report.benchmark, label: build.label, mode,
                browserTransport, traceDiagnosticRequested: traceRequested,
                metricBridge: result.metricBridge, dispatch: result.dispatchTiming,
                frameObservation: result.frameObservationTiming, nativePresentation: result.presentationTiming,
                longTaskSupported: result.inputObservation.longTaskSupported,
                recordedLongTasks: result.inputObservation.longTasks.length,
                droppedLongTasks: result.inputObservation.droppedLongTasks,
                slowestSamples: [...result.samples].sort((a, b) => b.durationMs - a.durationMs).slice(0, 3).map(sample => ({
                    ...sample,
                    input: result.inputObservation.events.filter(event => event.observedAtMs >= sample.browserObservation.startedAtMs &&
                        event.observedAtMs <= sample.browserObservation.endedAtMs),
                    longTasks: result.inputObservation.longTasks.filter(task => task.startTimeMs <= sample.browserObservation.endedAtMs &&
                        task.startTimeMs + task.durationMs >= sample.browserObservation.startedAtMs),
                })),
            }));
            if (traceRequested) {
                // Normal statistics/assertions above are complete before tracing
                // starts. Keep optional diagnostic failures separately labelled.
                page.off("pageerror", normalPageError);
                page.off("console", normalConsoleError);
                try {
                    result.traceDiagnostic = await runTextPanTraceDiagnostic({ browser, browserOwner, page, output,
                        build: build.id, mode, pointer, documentSha256: result.documentSha256,
                        compactBridge: result.metricBridge.kind === "compact-v1" });
                } catch (error) {
                    result.traceDiagnostic = { status: "failed", normalSamplesIncluded: false, error: error.stack || String(error) };
                    console.error(`${build.label} ${mode}: optional trace diagnostic failed: ${result.traceDiagnostic.error}`);
                }
                if (result.traceDiagnostic.status === "failed") process.exitCode = 1;
                if (result.traceDiagnostic.fatalBrowserClosure || browserOwner.shutdownStarted || !browser.isConnected()) {
                    report.traceDiagnosticStoppedBrowser = true;
                    process.exitCode = 1;
                    break benchmarkRuns;
                }
            }
        } catch (error) {
            result.status = "failed"; result.error = error.stack || String(error); process.exitCode = 1;
            console.error(`${build.label} ${mode}: ${result.error}`);
        } finally {
            try {
                // A fatal trace shutdown has already taken ownership of all
                // contexts; never await a second close on its stalled transport.
                if (!browserOwner) await context.close();
                else if (!browserOwner.shutdownStarted) {
                    try {
                        await boundedTextPanOperation(() => context.close(), performance.now() + TEXT_PAN_TRACE_LIMITS.teardownCommandMs, "close trace-enabled benchmark context");
                    } catch (error) {
                        result.contextCleanupError = error.stack || String(error);
                        report.traceDiagnosticStoppedBrowser = true;
                        process.exitCode = 1;
                        report.browserShutdown = await browserOwner.close("Trace-enabled context cleanup did not complete");
                    }
                }
            } finally { await writeFile(resolve(output, "report.json"), JSON.stringify(report, null, 2)); }
        }
        if (report.traceDiagnosticStoppedBrowser) break benchmarkRuns;
    }
    report.comparisons = ["Software2D", "WebGL2"].flatMap(mode => {
        const baseline = report.results.find(result => result.build === "baseline" && result.requestedMode === mode && result.status === "passed");
        const current = report.results.find(result => result.build === "current" && result.requestedMode === mode && result.status === "passed");
        if (!baseline || !current) return [];
        const consistency = {
            canonicalDocument: current.documentSha256 === baseline.documentSha256,
            camera: current.imported.zoom === baseline.imported.zoom
                && current.imported.pan.x === baseline.imported.pan.x
                && current.imported.pan.y === baseline.imported.pan.y,
            canvasBounds: ["x", "y", "width", "height"].every(key => current.imported.canvas[key] === baseline.imported.canvas[key]),
            surface: current.environment.surface === baseline.environment.surface
                && current.environment.device === baseline.environment.device,
            metricBridge: current.metricBridge.kind === baseline.metricBridge.kind,
            browserTransport: current.browserTransport === baseline.browserTransport,
            traceConfiguration: current.traceDiagnosticRequested === baseline.traceDiagnosticRequested,
        };
        const mismatches = Object.keys(consistency).filter(key => !consistency[key]);
        return [{ mode, baseline: baseline.timing, current: current.timing, consistency,
            comparable: mismatches.length === 0, mismatches,
            comparisonNote: mismatches.length ? "Different document/view/surface/bridge/transport/trace configuration: raw observations retained, comparative deltas suppressed." : "Same canonical document, camera, canvas bounds, observed surface, metric bridge and transport/trace configuration; sequential-order caveats still apply.",
            medianDeltaMs: mismatches.length ? null : current.timing.medianMs - baseline.timing.medianMs,
            p95DeltaMs: mismatches.length ? null : current.timing.p95Ms - baseline.timing.p95Ms,
            fixtureSha256: report.workload.inputSha256 }];
    });
} catch (error) { report.error = error.stack || String(error); process.exitCode = 1; console.error(report.error); }
finally {
    try {
        if (browserOwner) {
            report.browserShutdown = await browserOwner.close("Text benchmark finished");
            if (report.browserShutdown.status !== "closed") process.exitCode = 1;
            console.log(JSON.stringify({ diagnostic: "native-text-pan-browser-shutdown-v1", browserTransport,
                ...report.browserShutdown, errors: report.browserShutdown.errors.map(error => error.slice(0, 500)) }));
        } else await browser?.close();
    }
    finally {
        if (server.listening) {
            server.closeIdleConnections?.();
            if (browserOwner) {
                // These are only this benchmark's HTTP connections. Owned
                // Chromium has stopped (or failed its kill/exit checks), and no
                // later mode may run; open sockets must not hide the outcome.
                server.closeAllConnections?.();
                try {
                    await boundedTextPanOperation(() => new Promise(done => server.close(done)),
                        performance.now() + TEXT_PAN_TRACE_LIMITS.teardownCommandMs, "close trace-enabled benchmark HTTP server");
                } catch (error) { report.serverCleanupError = error.stack || String(error); process.exitCode = 1; }
            } else await new Promise(done => server.close(done));
        }
        report.finished = new Date().toISOString();
        try { await writeFile(resolve(output, "report.json"), JSON.stringify(report, null, 2)); }
        catch (error) {
            // A storage failure must be explicit, and must not strand the
            // process after a fatal browser teardown merely to retry a write.
            report.reportPersistenceError = error.stack || String(error);
            process.exitCode = 1;
            console.error(`Final benchmark report could not be persisted: ${report.reportPersistenceError}`);
        }
        // No asynchronous work follows this persisted fatal record. Public
        // BrowserServer.kill()/ChildProcess.kill() have already been attempted;
        // an unconfirmed exit is reported as failure, never as a stopped trace.
        // Exit also prevents lingering transport handles from hanging this job.
        if (browserOwner && (report.traceDiagnosticStoppedBrowser || report.browserShutdown?.status !== "closed" || report.serverCleanupError || report.reportPersistenceError))
            process.exit(1);
    }
}
