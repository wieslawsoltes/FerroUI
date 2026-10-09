// Captures a tour of pages of a ControlCatalog site whose module has both render backends twice, once
// drawn by Skia and once by the Vello backend (`?Renderer=Vello`), and reports per page how far the two
// pictures are apart.
//
//   scripts/build-browser.sh control-catalog-browser --features vello
//   node scripts/browser/compare-renderers.mjs [<site directory>] [--mode WebGL2] [--out <directory>] [--bound <share>] [--angle <backend>]
//
// The site directory defaults to target/browser-vello/control-catalog-browser. --mode is the rendering
// mode of both runs: WebGL2 compares Ganesh with the hybrid mode, Software2D Skia raster with the CPU
// mode. With --out the captures are written there as <page>.skia.png and <page>.vello.png.
//
// The pages are those of the tour in catalog-pages.mjs. Each is captured when no frame has been drawn
// for a while; a page that keeps drawing (an animation) is visited and not compared. Every pixel is
// compared, leaving out the corner where the catalog draws its frame counter. The two backends
// rasterize text differently (docs/porting/vello-backend.md, section 8), so a pixel counts as different
// when a channel is more than 32 of 255 apart, the tolerance of the comparison harness of the desktop,
// and a page passes with at most --bound of its pixels different (default 0.05). The share of pixels
// that differ at all is printed beside it. The exit code is 1 when a page differs by more, when a run
// was not drawn by the backend it was meant for, or when a page logged an error.
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
const bound = Number(option("--bound", "0.05"));
const angle = option("--angle", undefined);
const site = path.resolve(args[0] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser-vello", "control-catalog-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser --features vello`);
    process.exit(2);
}
const isolated = fs.existsSync(path.join(site, "ferroui-threads.js"));
if (out) { fs.mkdirSync(out, { recursive: true }); }

// The debug overlay of the catalog (the frame counter) is drawn in the top left corner of the view.
const FRAME_COUNTER = { x: 0, y: 0, width: 520, height: 28 };
const START_TIMEOUT = 180_000;
const fileName = (name) => name.replace(/[^A-Za-z0-9]+/g, "-").toLowerCase();

/** Visits the tour in one page and returns what each page of it showed. */
async function tour(label, query) {
    // The files of the pages are fetched by the navigations that need them, in both runs alike.
    const page = await open(site, {
        query: `?RenderingMode=${mode}&PrefetchAssets=false${query}`, width: TOUR_SIZE.width, height: TOUR_SIZE.height, isolated, angle
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

const skia = await tour("skia", "");
const vello = await tour("vello", "&Renderer=Vello");

let failed = 0;
const fail = (text) => { failed++; console.log(`FAIL  ${text}`); };
const a = skia.rendering; const b = vello.rendering;
if (a.renderer !== "Skia") { fail(`the first run was not drawn by Skia: ${JSON.stringify(a)}`); }
if (b.renderer !== "Vello") { fail(`the run with Renderer=Vello was not drawn by the Vello backend (was the module built with the feature vello?): ${JSON.stringify(b)}`); }
if (a.kind !== b.kind) { fail(`the two runs drew to canvases of different kinds: ${a.kind} and ${b.kind}`); }
console.log(`Skia:  ${a.frames} frames, ${a.kind} (OpenGL ES ${a.gl})`);
console.log(`Vello: ${b.frames} frames, ${b.kind} (OpenGL ES ${b.gl}), panics of the render thread: ${b.panics}`);

const percent = (part, whole) => `${(100 * part / whole).toFixed(2)} %`;
for (const entry of TOUR) {
    const first = skia.shown.get(entry.name); const second = vello.shown.get(entry.name);
    if (!first.quiet || !second.quiet) {
        console.log(`      ${entry.name}: not compared, the page keeps drawing (${colours(first.capture)} and ${colours(second.capture)} colours)`);
        continue;
    }
    const coarse = differing(second.capture, first.capture, 1, { tolerance: 32, skip: FRAME_COUNTER });
    const fine = differing(second.capture, first.capture, 1, { tolerance: 0, skip: FRAME_COUNTER });
    const line = `${entry.name}: ${percent(coarse.different, coarse.samples)} of the pixels differ by more than 32 of 255, ${percent(fine.different, fine.samples)} at all`;
    if (colours(second.capture) <= 20) { fail(`${entry.name}: the page drawn by the Vello backend shows ${colours(second.capture)} colours, it was not drawn`); } else if (coarse.different > coarse.samples * bound) { fail(line); } else { console.log(`ok    ${line}`); }
}
for (const [label, run] of [["Skia", skia], ["the Vello backend", vello]]) {
    if (run.errors.length > 0) { fail(`the page drawn by ${label} logged errors:\n      ${run.errors.join("\n      ")}`); }
}
if (out) { console.log(`captures written to ${out}`); }
console.log(failed === 0 ? `all ${TOUR.length} pages of the tour were visited with both backends` : `${failed} failures`);
process.exit(failed === 0 ? 0 : 1);
