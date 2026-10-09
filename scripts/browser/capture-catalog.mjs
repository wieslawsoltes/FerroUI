// Captures a tour of pages of the ControlCatalog site built with threads twice, once drawn by the
// render thread and once with the same module kept on the thread of the page (`?RenderThread=false`),
// and reports per page how many sampled pixels differ between the two.
//
//   scripts/build-browser.sh control-catalog-browser --threads
//   node scripts/browser/capture-catalog.mjs [<site directory>] [--mode WebGL2] [--out <directory>]
//
// The site directory defaults to target/browser-threads/control-catalog-browser. With --out the
// captures are written there as <page>.render-thread.png and <page>.one-thread.png.
//
// The pages are those of the tour in catalog-pages.mjs. Each is captured when no frame has been drawn
// for a while; a page that keeps drawing (an animation) is visited and not compared. The pixels are
// sampled every 4 pixels and compared with the tolerance the tests use (8 per channel; a page passes
// with at most 0.5 % of its samples different), leaving out the corner where the catalog draws its
// frame counter, which shows a different number in every run. The exit code is 1 when a page differs
// by more, when a run was not drawn by the thread it was meant for, or when a page logged an error.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, sleep, differing, colours } from "./harness.mjs";
import { TOUR, TOUR_SIZE, drive, settled, visit, waitUntilReady } from "./catalog-pages.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : fallback; };
const mode = option("--mode", "WebGL2");
const out = option("--out", undefined);
const site = path.resolve(args[0] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-threads", "control-catalog-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser --threads`);
    process.exit(2);
}
if (!fs.existsSync(path.join(site, "ferroui-threads.js"))) {
    console.error(`${site} was built without threads: there is no render thread to compare with`);
    process.exit(2);
}

// The debug overlay of the catalog (the frame counter) is drawn in the top left corner of the view.
const FRAME_COUNTER = { x: 0, y: 0, width: 520, height: 28 };
const START_TIMEOUT = 180_000;
const fileName = (name) => name.replace(/[^A-Za-z0-9]+/g, "-").toLowerCase();

/** Visits the tour in one page and returns what each page of it showed. */
async function tour(label, query) {
    // The files of the pages are fetched by the navigations that need them, in both runs alike.
    const page = await open(site, {
        query: `?RenderingMode=${mode}&PrefetchAssets=false${query}`, width: TOUR_SIZE.width, height: TOUR_SIZE.height, isolated: true
    });
    const shown = new Map();
    try {
        await waitUntilReady(page, START_TIMEOUT);
        drive(page);
        for (const entry of TOUR) {
            await visit(page, entry);
            // The pictures of a page arrive after it is shown, and are drawn when they do.
            await sleep(600);
            const quiet = entry.animated ? null : await settled(page, 700, 15_000);
            const capture = await page.screenshot(out ? path.join(out, `${fileName(entry.name)}.${label}.png`) : undefined);
            shown.set(entry.name, { capture, quiet: quiet !== null });
        }
        return { shown, rendering: await page.rendering(), errors: [...page.errors] };
    } finally { await page.close(); }
}

const onRenderThread = await tour("render-thread", "");
const onOneThread = await tour("one-thread", "&RenderThread=false");

let failed = 0;
const fail = (text) => { failed++; console.log(`FAIL  ${text}`); };
const a = onRenderThread.rendering; const b = onOneThread.rendering;
if (!(a.on_render_thread === "true" && a.other_thread === "true" && a.frame_thread === a.render_thread)) {
    fail(`the first run was not drawn by a render thread: ${JSON.stringify(a)}`);
}
if (!(b.on_render_thread === "false" && b.other_thread === "false")) {
    fail(`the run with RenderThread=false was not drawn by the thread of the page: ${JSON.stringify(b)}`);
}
console.log(`render thread: ${a.frames} frames, ${a.kind} (OpenGL ES ${a.gl}), calls of its ticks served by the main thread: ${a.proxied}, panics: ${a.panics}`);
console.log(`one thread:    ${b.frames} frames, ${b.kind} (OpenGL ES ${b.gl})`);

for (const entry of TOUR) {
    const first = onRenderThread.shown.get(entry.name); const second = onOneThread.shown.get(entry.name);
    if (!first.quiet || !second.quiet) {
        console.log(`      ${entry.name}: not compared, the page keeps drawing (${colours(first.capture)} and ${colours(second.capture)} colours)`);
        continue;
    }
    const { samples, different } = differing(first.capture, second.capture, 4, { skip: FRAME_COUNTER });
    const line = `${entry.name}: ${different} of ${samples} sampled pixels differ`;
    if (colours(second.capture) <= 20) { fail(`${entry.name}: the page on one thread shows ${colours(second.capture)} colours, it was not drawn`); } else if (different > samples / 200) { fail(line); } else { console.log(`ok    ${line}`); }
}
for (const [label, run] of [["the render thread", onRenderThread], ["one thread", onOneThread]]) {
    if (run.errors.length > 0) { fail(`the page on ${label} logged errors:\n      ${run.errors.join("\n      ")}`); }
}
if (out) { console.log(`captures written to ${out}`); }
console.log(failed === 0 ? `all ${TOUR.length} pages of the tour were visited in both modes` : `${failed} failures`);
process.exit(failed === 0 ? 0 : 1);
