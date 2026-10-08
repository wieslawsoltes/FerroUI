// Behaviour tests of step B2.1 of the render worker on the render_worker_clear example, in headless
// Chrome: a canvas whose control is transferred to a thread of the module is cleared there with
// WebGL through the GL interface of the port, without the compositor. The page is cross-origin
// isolated either by the headers of the server or by the service worker of the threaded mode.
//
//   scripts/browser/setup.sh --threads && source .tools/env.sh
//   scripts/build-browser.sh render_worker_clear --threads
//   node scripts/browser/tests/render_worker_clear.test.mjs [<site directory>]     default: target/browser-threads/render_worker_clear
//
// The page writes the state of the example (`renderWorkerClearState`, a line of `name=value` pairs)
// into the element `result`; the colour is read in a capture of the page. See
// docs/porting/browser-render-worker.md, section 6 and "B2.1".
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, near, sleep } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-threads", "render_worker_clear");

const RESULT = "document.getElementById('result')?.textContent ?? ''";
const WORKERS = "navigator.serviceWorker.getRegistrations().then((registrations) => registrations.length)";
// The colour the example clears to (COLOR in its main.rs), and the background of the page.
const COLOR = [32, 96, 192];
const PAGE = [255, 255, 255];

// Waits for the thread to report its frame and checks the report.
async function expectClearOnAnotherThread(page) {
    await page.waitFor(`/state=(done|failed)/.test(${RESULT})`, 60000);
    const result = await page.evaluate(RESULT);
    assert(/^atomics=true state=done thread=[1-9]\d* other_thread=true target=1 kind=webgl gl=[23] gl_error=0 size=200x120 color=32,96,192$/.test(result),
        `the page shows: ${result}`);
    assert(await page.evaluate("self.crossOriginIsolated") === true, "the page is not cross-origin isolated");
}

// Reads the colour in a capture of the page: the canvas shows it from edge to edge, and the page
// next to the canvas does not.
async function expectColorInCanvas(page) {
    assert(await page.evaluate("document.querySelectorAll('#view canvas').length") === 1, "the view does not hold one canvas");
    const box = JSON.parse(await page.evaluate("JSON.stringify(document.querySelector('#view canvas').getBoundingClientRect())"));
    assert(box.width === 200 && box.height === 120, `the canvas is ${box.width} x ${box.height}, not 200 x 120`);
    // The control of the canvas is with the worker: the page cannot draw to it or size it.
    assert(await page.evaluate("(() => { try { document.querySelector('#view canvas').getContext('2d'); return false; } catch { return true; } })()") === true,
        "the page can still take a context of the canvas: its control was not transferred");
    const inside = [
        [box.x + box.width / 2, box.y + box.height / 2],
        [box.x + 3, box.y + 3], [box.x + box.width - 4, box.y + 3],
        [box.x + 3, box.y + box.height - 4], [box.x + box.width - 4, box.y + box.height - 4]
    ].map(([x, y]) => [Math.round(x), Math.round(y)]);
    const outside = [[Math.round(box.x + box.width + 20), Math.round(box.y + box.height / 2)]];
    // The frame of the worker reaches the page a little after the thread reported it.
    const end = Date.now() + 15000;
    let seen;
    for (;;) {
        const shot = await page.screenshot();
        seen = inside.map(([x, y]) => shot.pixel(x, y));
        if (seen.every((pixel) => near(pixel, COLOR))) {
            const beside = outside.map(([x, y]) => shot.pixel(x, y));
            assert(beside.every((pixel) => near(pixel, PAGE)), `the page beside the canvas is ${beside.join(" | ")}`);
            return;
        }
        if (Date.now() > end) { break; }
        await sleep(200);
    }
    throw new Error(`the canvas does not show ${COLOR.join(",")}: the capture has ${seen.join(" | ")} at ${inside.join(" | ")}`);
}

await run([
    ["a thread clears the transferred canvas in a page isolated by the headers of the server", async () => {
        const page = await open(site, { isolated: true, width: 320, height: 200 });
        try {
            await expectClearOnAnotherThread(page);
            await expectColorInCanvas(page);
            assert(await page.evaluate(WORKERS) === 0, "a service worker was registered although the server sends the headers");
            assert(page.navigations.length === 1, `the page was loaded ${page.navigations.length} times: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }],
    ["without the headers the service worker isolates the page and the thread clears the canvas", async () => {
        const page = await open(site, { width: 320, height: 200 });
        try {
            await expectClearOnAnotherThread(page);
            await expectColorInCanvas(page);
            assert(await page.evaluate(WORKERS) === 1, "the service worker of the threaded mode is not registered");
            assert(await page.evaluate("navigator.serviceWorker.controller?.scriptURL.endsWith('/ferroui-sw.js?coi=1')") === true,
                "the page is not controlled by ferroui-sw.js?coi=1");
            assert(page.navigations.length === 2, `the page was loaded ${page.navigations.length} times, not twice: ${page.navigations.join(", ")}`);
            assert(page.errors.length === 0, `errors in the page:\n${page.errors.join("\n")}`);
        } finally { await page.close(); }
    }]
]);
