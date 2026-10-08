// Behaviour tests of steps B2.1 and B2.3 of the render worker on the render_worker_clear example,
// in headless Chrome: a canvas whose control is transferred to a thread of the module is drawn to
// there, without the compositor. The page is cross-origin isolated either by the headers of the
// server or by the service worker of the threaded mode.
//
//   scripts/browser/setup.sh --threads && source .tools/env.sh
//   scripts/build-browser.sh render_worker_clear --threads
//   node scripts/browser/tests/render_worker_clear.test.mjs [<site directory>]     default: target/browser-threads/render_worker_clear
//
// B2.1: one frame, a clear with WebGL through the GL interface of the port.
// B2.3: the software mode; a frame loop on the thread that goes on while the page is busy; the
// size of the canvas crossing to the thread; the wake-up of the page from the thread; and the
// thread of the page waiting for a frame, with and without a frame out of turn.
//
// The page writes the state of the example (`renderWorkerClearState`, a line of `name=value` pairs)
// into the element `result` and the frame counter the thread wakes it with into `frames`; the
// picture is read in a capture of the page. The lines the checks print before their verdict
// (`    measured: ...`) are the measurements the design asks for; they are recorded, and only what
// must hold is asserted. See docs/porting/browser-render-worker.md, section 6, "B2.1" and "B2.3".
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, near, sleep } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-threads", "render_worker_clear");

const RESULT = "document.getElementById('result')?.textContent ?? ''";
const STATE = "renderWorkerClear.renderWorkerClearState()";
const FRAMES = "renderWorkerClear.renderWorkerClearFrames()";
const WAKE_UPS = "Number(/wakeups=(\\d+)/.exec(document.getElementById('frames').textContent)[1])";
const REPORTED = "Number(/frames=(\\d+)/.exec(document.getElementById('frames').textContent)[1])";
const WORKERS = "navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)";
// The colour of a frame without a frame loop (COLOR in the main.rs of the example); with a loop the
// third channel runs through ANIMATED_BLUE .. ANIMATED_BLUE + ANIMATED_BLUE_STEPS - 1.
const COLOR = [32, 96, 192];
const ANIMATED_BLUE = [64, 159];
// The square every frame has (MARKER_COLOR, MARKER_OFFSET, MARKER_SIZE), in device pixels from
// the top left corner of the canvas, and the background of the page.
const MARKER_COLOR = [224, 160, 32];
const MARKER = { offset: 20, size: 30 };
const PAGE = [255, 255, 255];
// The longest the thread of the page waits for a frame in the experiment, in milliseconds.
const WAIT_TIMEOUT = 2000;

const measured = (text) => console.log(`    measured: ${text}`);
const fields = (line) => Object.fromEntries(line.trim().split(/\s+/).map((pair) => pair.split("=")).filter((pair) => pair.length === 2));
// A page whose main thread never returns would leave `evaluate` waiting for ever.
const within = (promise, ms, what) => Promise.race([
    promise,
    sleep(ms).then(() => { throw new Error(`the page did not answer within ${ms} ms: ${what}`); })
]);

// Waits for the thread to report its first frame and checks the report. `kind` is "webgl" or
// "software"; `animated` tells whether the page runs a frame loop.
async function expectFirstFrame(page, { kind = "webgl", animated = false, size = "200x120" } = {}) {
    await page.waitFor(`/state=(done|failed)/.test(${RESULT})`, 60000);
    const result = await page.evaluate(RESULT);
    const state = fields(result);
    assert(state.atomics === "true" && state.state === "done" && /^[1-9]\d*$/.test(state.thread) && state.other_thread === "true" && state.target === "1",
        `the page shows: ${result}`);
    assert(state.kind === kind, `the render target is ${state.kind}, not ${kind}: ${result}`);
    assert(kind === "webgl" ? /^[23]$/.test(state.gl) : state.gl === "0", `the version of OpenGL ES is ${state.gl}: ${result}`);
    assert(state.gl_error === "0", `OpenGL reported the error ${state.gl_error}: ${result}`);
    assert(state.size === size, `the first frame is ${state.size}, not ${size}: ${result}`);
    if (!animated) {
        assert(state.color === COLOR.join(",") && state.frames === "1", `the page shows: ${result}`);
    }
    assert(await page.evaluate("self.crossOriginIsolated") === true, "the page is not cross-origin isolated");
    return state;
}

const isFrameColor = (animated) => (pixel) => animated
    ? near(pixel.slice(0, 2), COLOR.slice(0, 2)) && pixel[2] >= ANIMATED_BLUE[0] - 6 && pixel[2] <= ANIMATED_BLUE[1] + 6
    : near(pixel, COLOR);

// Reads a capture of the page until it shows a frame of the right size: the colour of the frame
// from edge to edge of the canvas, the page beside it, and (`marker`) the square where it belongs
// in device pixels and nowhere else. A frame that the browser stretches from another size has the
// square at another place and of another size. `width` and `height` are the size of the canvas.
async function expectFrameInCanvas(page, { width = 200, height = 120, animated = false, marker = true, viewport } = {}) {
    assert(await page.evaluate("document.querySelectorAll('#view canvas').length") === 1, "the view does not hold one canvas");
    const box = JSON.parse(await page.evaluate("JSON.stringify(document.querySelector('#view canvas').getBoundingClientRect())"));
    assert(box.width === width && box.height === height, `the canvas is ${box.width} x ${box.height}, not ${width} x ${height}`);
    // The control of the canvas is with the worker: the page cannot draw to it or size it.
    assert(await page.evaluate("(() => { try { document.querySelector('#view canvas').getContext('2d'); return false; } catch { return true; } })()") === true,
        "the page can still take a context of the canvas: its control was not transferred");
    const at = ([x, y]) => [Math.round(box.x + x), Math.round(box.y + y)];
    const inside = [
        [box.width / 2, box.height / 2],
        [3, 3], [box.width - 4, 3], [3, box.height - 4], [box.width - 4, box.height - 4]
    ].map(at);
    const outside = [[box.width + 20, box.height / 2]].map(at);
    // Just inside the square at two of its corners, and just outside it at the same two.
    const far = MARKER.offset + MARKER.size;
    const inMarker = [[MARKER.offset + 3, MARKER.offset + 3], [far - 4, far - 4]].map(at);
    const aroundMarker = [[MARKER.offset - 5, MARKER.offset - 5], [far + 6, far + 6]].map(at);
    const frameColor = isFrameColor(animated);
    // The frame of the worker reaches the page a little after the thread reported it.
    const end = Date.now() + 15000;
    let seen;
    for (;;) {
        const shot = await page.screenshot();
        if (viewport) {
            assert(shot.width === viewport[0] && shot.height === viewport[1],
                `the capture is ${shot.width} x ${shot.height}, not ${viewport[0]} x ${viewport[1]}`);
        }
        const read = (points) => points.map(([x, y]) => shot.pixel(x, y));
        seen = { inside: read(inside), inMarker: read(inMarker), aroundMarker: read(aroundMarker) };
        const filled = seen.inside.every(frameColor);
        const square = !marker || (seen.inMarker.every((pixel) => near(pixel, MARKER_COLOR)) && seen.aroundMarker.every(frameColor));
        if (filled && square) {
            const beside = read(outside);
            assert(beside.every((pixel) => near(pixel, PAGE)), `the page beside the canvas is ${beside.join(" | ")}`);
            return seen.inside[0];
        }
        if (Date.now() > end) { break; }
        await sleep(200);
    }
    throw new Error(`the canvas does not show a frame of ${width} x ${height}: the capture has ${seen.inside.join(" | ")} at ${inside.join(" | ")}`
        + (marker ? `, ${seen.inMarker.join(" | ")} inside the square and ${seen.aroundMarker.join(" | ")} around it` : ""));
}

// Whether the frames of the render target have the square: WebGL 1 cannot copy between
// framebuffers, and its frames are the colour alone.
const hasMarker = (state) => state.marker === "true";

// The frame loop goes on while the script of the page keeps its thread busy.
async function expectFramesWhileThePageIsBusy(page) {
    await page.waitFor(`${FRAMES} > 10`, 15000);
    const [before, after] = JSON.parse(await within(page.evaluate(`(() => {
        const before = ${FRAMES};
        const end = performance.now() + 200;
        while (performance.now() < end) { }
        return JSON.stringify([before, ${FRAMES}]);
    })()`), 20000, "200 ms of work on the thread of the page"));
    measured(`frames drawn while the thread of the page was busy for 200 ms: ${after - before}`);
    assert(after - before >= 3, `the thread drew ${after - before} frames while the page was busy for 200 ms (${before} before, ${after} after)`);
}

// The thread wakes the dispatcher of the page after its frames, and the page is told the counter.
async function expectWakeUps(page) {
    const first = await page.evaluate(WAKE_UPS);
    await page.waitFor(`${WAKE_UPS} >= ${first + 5}`, 15000);
    const [reported, frames, wakeUps] = JSON.parse(await page.evaluate(`JSON.stringify([${REPORTED}, ${FRAMES}, ${WAKE_UPS}])`));
    measured(`wake-ups of the page: ${wakeUps} for ${frames} frames`);
    assert(reported > 0 && reported <= frames, `the page was told ${reported} frames, the thread drew ${frames}`);
    assert(wakeUps <= frames, `the page was woken ${wakeUps} times for ${frames} frames`);
}

// The experiment: the thread of the page waits for a frame. What is asserted is that the page
// does not hang; how long the wait takes, and whether it ends without a frame out of turn, is
// recorded.
async function measureWaits(page) {
    const wait = async (outOfTurn) => {
        const started = Date.now();
        const line = await within(page.evaluate(`renderWorkerClear.renderWorkerClearWaitForFrame(${outOfTurn}, ${WAIT_TIMEOUT})`),
            WAIT_TIMEOUT + 20000, `a wait for a frame, out of turn: ${outOfTurn}`);
        const outcome = fields(line);
        assert(/^(true|false)$/.test(outcome.ended) && Number.isFinite(Number(outcome.duration_ms)), `the wait answered: ${line}`);
        // The wait is bounded by its timeout, give or take the spin of the runtime.
        assert(Number(outcome.duration_ms) <= WAIT_TIMEOUT + 1000, `the wait took ${outcome.duration_ms} ms with a timeout of ${WAIT_TIMEOUT} ms`);
        assert(Date.now() - started <= WAIT_TIMEOUT + 15000, `the page took ${Date.now() - started} ms to answer`);
        return outcome;
    };
    const withFrame = [];
    for (let i = 0; i < 5; i++) {
        withFrame.push(await wait(true));
        // Back to the browser between the waits, so that each starts from a page at rest.
        await sleep(100);
    }
    measured(`wait for a frame with a frame out of turn: ended=${withFrame.map((outcome) => outcome.ended).join(",")} duration_ms=${withFrame.map((outcome) => outcome.duration_ms).join(",")}`);
    const without = [];
    for (let i = 0; i < 3; i++) {
        without.push(await wait(false));
        await sleep(100);
    }
    measured(`wait for a frame without a frame out of turn (timeout ${WAIT_TIMEOUT} ms): ended=${without.map((outcome) => outcome.ended).join(",")} duration_ms=${without.map((outcome) => outcome.duration_ms).join(",")}`);
    // The page goes on afterwards: the thread draws and the page answers.
    const frames = await within(page.evaluate(FRAMES), 20000, "the frame counter after the waits");
    await page.waitFor(`${FRAMES} > ${frames + 5}`, 15000);
}

// The window is resized: the canvas element follows it, the observer of the element writes the
// new size where the thread reads it, and the thread sizes the canvas it draws to.
async function expectResize(page, viewport, canvas, marker) {
    await page.resize(viewport[0], viewport[1]);
    await page.waitFor(`${STATE}.includes(' size=${canvas[0]}x${canvas[1]} ')`, 15000);
    await expectFrameInCanvas(page, { width: canvas[0], height: canvas[1], animated: true, marker, viewport });
}

// The checks of a page with a frame loop, for one rendering mode.
async function expectFrameLoop(query, kind) {
    const page = await open(site, { isolated: true, width: 320, height: 200, query });
    try {
        const state = await expectFirstFrame(page, { kind, animated: true });
        const marker = hasMarker(state);
        measured(`render target: ${kind}${kind === "webgl" ? `, OpenGL ES ${state.gl}` : ""}, the frames ${marker ? "have" : "do not have"} the square`);
        const first = await expectFrameInCanvas(page, { animated: true, marker });
        // The colour changes with the frames.
        const end = Date.now() + 15000;
        let later = first;
        while (later[2] === first[2] && Date.now() < end) {
            await sleep(120);
            later = await expectFrameInCanvas(page, { animated: true, marker });
        }
        assert(later[2] !== first[2], `the colour of the canvas stays ${first.join(",")}`);
        await expectFramesWhileThePageIsBusy(page);
        await expectWakeUps(page);
        await measureWaits(page);
        await expectResize(page, [480, 300], [300, 180], marker);
        await expectResize(page, [240, 150], [150, 90], marker);
        const last = fields(await page.evaluate(STATE));
        assert(last.gl_error === "0", `OpenGL reported the error ${last.gl_error}`);
        // A build with the assertions of Emscripten warns once, as an error of the console, when
        // the thread of the page blocks: that is the experiment, and it is recorded, not failed.
        const warned = page.errors.filter((line) => line.includes("Blocking on the main thread"));
        if (warned.length > 0) { measured(`the runtime warned: ${warned[0]}`); }
        const errors = page.errors.filter((line) => !warned.includes(line));
        assert(errors.length === 0, `errors in the page:\n${errors.join("\n")}`);
    } finally { await page.close(); }
}

await run([
    ["a thread clears the transferred canvas in a page isolated by the headers of the server", async () => {
        const page = await open(site, { isolated: true, width: 320, height: 200 });
        try {
            const state = await expectFirstFrame(page);
            await expectFrameInCanvas(page, { marker: hasMarker(state) });
            // The one frame woke the page once, and the page was told of it.
            await page.waitFor(`${WAKE_UPS} === 1 && ${REPORTED} === 1`, 15000);
            assert(await page.evaluate(WORKERS) === 0, "a service worker was registered although the server sends the headers");
            assert(page.navigations.length === 1, `the page was loaded ${page.navigations.length} times: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["without the headers the service worker isolates the page and the thread clears the canvas", async () => {
        const page = await open(site, { width: 320, height: 200 });
        try {
            const state = await expectFirstFrame(page);
            await expectFrameInCanvas(page, { marker: hasMarker(state) });
            assert(await page.evaluate(WORKERS) === 1, "the service worker of the threaded mode is not registered");
            assert(await page.evaluate("navigator.serviceWorker.controller?.scriptURL.endsWith('/ferroui-sw.js?coi=1')") === true,
                "the page is not controlled by ferroui-sw.js?coi=1");
            assert(page.navigations.length === 2, `the page was loaded ${page.navigations.length} times, not twice: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["the software mode fills a retained framebuffer on the thread and puts it to the canvas", async () => {
        const page = await open(site, { isolated: true, width: 320, height: 200, query: "?RenderingMode=Software2D" });
        try {
            const state = await expectFirstFrame(page, { kind: "software" });
            assert(hasMarker(state), "the software frame has no square");
            await expectFrameInCanvas(page);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["WebGL frames: the loop runs while the page is busy, wakes the page, is waited for and follows a resize",
        () => expectFrameLoop("?Frames=true", "webgl")],
    ["software frames: the loop runs while the page is busy, wakes the page, is waited for and follows a resize",
        () => expectFrameLoop("?Frames=true&RenderingMode=Software2D", "software")]
]);
