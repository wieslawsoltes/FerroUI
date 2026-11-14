#!/usr/bin/env node
// What a page switch of the ControlCatalog site costs in headless Chromium: the entries of the
// drawer are clicked in turn with real pointer events, and for each switch the WebGL work of its
// frames is counted (gl-work.mjs) and the time to the first frame of the new page is measured.
//
//   node scripts/browser/navigation-cost.mjs <site directory | address> [options]
//
//   --rounds <n>     how often the route (Buttons, CheckBox, Home) is gone through (default 3). The
//                    first visit of a page is reported apart from the visits after it: it uploads
//                    the glyphs and links the programs the page needs, the later ones should not
//   --size <w>x<h>   size of the viewport (default 1400x900)
//   --angle <name>   what WebGL runs on: swiftshader (the default) or a backend of ANGLE with the
//                    GPU (metal). The counts are the same on both; the times are not
//   --query <text>   more of the query string of the page
//   --slowdown <n>   slow the processor of the page down n times (for the times only)
//   --json           print the result as JSON
//   --gate           exit with 1 when a switch is over the budget (`BUDGET`)
//
// The site is a folder built by scripts/build-browser.sh (a site built with threads is opened with
// `RenderThread=false`: the work is counted on the thread of the page), or the address of a
// deployed site, opened as given: pass the query that keeps its frames on the thread of the page
// (`https://.../?Threads=false`).
//
// The counts do not depend on the machine, so they are gated: here with --gate, in
// tests/navigation_cost.test.mjs for every pull request, and in the suite `browser` of
// scripts/perf/gate.py, which also compares the times of two sites. Why they are gated:
// docs/porting/desktop-performance.md, section 9.56.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { open, sleep } from "./harness.mjs";
import { drive, inDrawer, waitUntilReady } from "./catalog-pages.mjs";
import { GL_WORK_COUNTS, GL_WORK_PROBE, readGlWork } from "./gl-work.mjs";

/** The pages of the route, and the section of the drawer that holds the first two. */
export const ROUTE = ["Buttons", "CheckBox", "Home"];
const SECTION = "Basic Input";
const START_TIMEOUT = 180_000;
// How long the frames of a switch are counted after the view shows the page: its transition and
// the animation of the selection of the drawer are over by then.
const SETTLE = 1500;

// What a switch may cost, for each page of the route and visit: the masks (textures of one channel)
// its frames upload, and their pixels. A frame drawn under a clip that is one rectangle uploads the
// masks of the paths it rasterises on the processor and, on the first visit of a page, its glyphs:
// 34 masks and 0.17 million pixels for the first switch to Buttons, 18 and 0.11 million for one
// after it, 2 for CheckBox, one a frame for the animation of Home. A frame drawn under a clip of
// several rectangles uploads one for every draw (section 9.56): 191 masks and 3.3 million pixels,
// 120 and 2.5 million, 27 to 38, and 420 to 440 for Home. The budgets are about twice the first
// numbers and less than half of the second ones; a budget of pixels leaves room for one more frame
// of the whole content area (1.1 million pixels), and is left out where that is all there is.
export const BUDGET = {
    first: {
        Buttons: { masks: 80, maskPixels: 1_600_000 },
        CheckBox: { masks: 16 },
    },
    again: {
        Buttons: { masks: 48, maskPixels: 1_400_000 },
        CheckBox: { masks: 16 },
        Home: { masks: 220, maskPixels: 45_000_000 },
    },
};

const median = (values) => {
    const sorted = [...values].sort((a, b) => a - b);
    return sorted.length % 2 ? sorted[(sorted.length - 1) / 2] : (sorted[sorted.length / 2 - 1] + sorted[sorted.length / 2]) / 2;
};

// In the page: the time of the pointer event of a click, of the first animation frame in which the
// view shows the page asked for, of the first one in which its navigation has ended and of the
// first frame the view drew at or after that, and of the first animation frame in which the
// pixels of the content area are other ones than before the click (`painted`). The pixels are read
// through a canvas of the page, which waits for the frame to be rasterised: the other times are
// those of the thread of the page alone.
const TIMES = `(() => {
    const n = globalThis.__ferroNavigation = { start: 0, target: null, shown: null, ended: null, drawn: null, before: 0, painted: null };
    const probe = document.createElement("canvas"); probe.width = 112; probe.height = 80;
    const context = probe.getContext("2d", { willReadFrequently: true });
    const source = document.querySelector("#out canvas");
    // The content area, right of the drawer and below the title bars, scaled down.
    n.pixels = () => {
        const k = source.width / source.clientWidth;
        context.drawImage(source, 280 * k, 100 * k, source.width - 280 * k, source.height - 100 * k, 0, 0, 112, 80);
        const data = context.getImageData(0, 0, 112, 80).data; let hash = 0;
        for (let i = 0; i < data.length; i += 4) { hash = (Math.imul(hash, 31) + data[i] + (data[i + 1] << 8) + (data[i + 2] << 16)) | 0; }
        return hash;
    };
    const frames = () => Number(/frames=(\\d+)/.exec(controlCatalog.catalogRendering())[1]);
    addEventListener("pointerdown", () => {
        n.start = performance.now(); n.shown = null; n.ended = null; n.drawn = null; n.painted = null;
        for (const name of Object.keys(__ferroGlWork)) { __ferroGlWork[name] = 0; }
    }, true);
    let last = frames();
    const loop = () => {
        const now = performance.now();
        const count = frames();
        if (n.target !== null && n.drawn === null) {
            if (n.ended === null) {
                const state = JSON.parse(controlCatalog.catalogState());
                if (n.shown === null && state.page === n.target) { n.shown = now - n.start; }
                if (state.page === n.target && !state.navigating) { n.ended = now - n.start; }
            }
            if (n.ended !== null && count !== last) { n.drawn = now - n.start; }
        }
        if (n.target !== null && n.painted === null && n.start > 0 && n.pixels() !== n.before) {
            n.painted = performance.now() - n.start;
        }
        last = count;
        requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
})()`;

/**
 * Goes through the route `rounds` times and returns `{ rendering, navigations, first, again }`:
 * every switch (`{ page, visit, frames, shownMs, endedMs, paintedMs, ...counts }`), and for each page of the route the
 * switch that showed it first (`first`; not for the page the site starts on) and the median of the
 * switches that showed it again (`again`).
 */
export async function measure(site, { rounds = 3, width = 1400, height = 900, angle, query = "", slowdown = 1 } = {}) {
    const address = /^https?:/.test(site) ? site : null;
    const directory = address ? fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-navigation-")) : site;
    const threaded = !address && fs.existsSync(path.join(site, "ferroui-threads.js"));
    const page = drive(await open(directory, {
        query: `?RenderingMode=WebGL2${threaded ? "&RenderThread=false" : ""}${query}`, width, height, angle,
        isolated: threaded, initScript: GL_WORK_PROBE, navigate: !address,
    }));
    try {
        if (address) { await page.send("Page.navigate", { url: address }); }
        await waitUntilReady(page, START_TIMEOUT);
        const rendering = await page.rendering();
        await page.evaluate(TIMES);
        if (slowdown > 1) { await page.send("Emulation.setCPUThrottlingRate", { rate: slowdown }); }

        const go = async (text) => {
            const entry = await page.find(text, inDrawer);
            const before = Number((await page.rendering()).frames);
            await page.evaluate(`(__ferroNavigation.target = ${JSON.stringify(text)}, __ferroNavigation.start = 0, __ferroNavigation.before = __ferroNavigation.pixels(), 0)`);
            await page.clickElement(entry);
            await page.until(`the page "${text}" is shown`, (s) => s.page === text && !s.navigating, 60_000);
            await sleep(SETTLE);
            const times = JSON.parse(await page.evaluate("JSON.stringify(__ferroNavigation)"));
            const work = await readGlWork(page);
            if (work.draws === 0) {
                throw new Error(`no draw call was counted for the switch to "${text}": the frames of the site are not `
                    + `drawn with WebGL on the thread of the page (${JSON.stringify(rendering)})`);
            }
            return { page: text, frames: Number((await page.rendering()).frames) - before, shownMs: Math.round(times.shown ?? 0), endedMs: Math.round(times.drawn ?? times.ended ?? 0), paintedMs: Math.round(times.painted ?? 0), ...work };
        };

        // The entries of the route are in a section of the drawer that starts closed: open it (its
        // entry shows the page of the section) and start from the home page.
        const seen = new Set(["Home", SECTION]);
        if (!(await page.state()).elements.some((e) => e.text === ROUTE[0] && inDrawer(e))) { await go(SECTION); }
        if ((await page.state()).page !== "Home") { await go("Home"); }

        const navigations = [];
        for (let round = 0; round < rounds; round++) {
            for (const text of ROUTE) {
                navigations.push({ ...(await go(text)), visit: seen.has(text) ? "again" : "first" });
                seen.add(text);
            }
        }
        if (page.errors.length) { throw new Error(`the page reported errors: ${page.errors.slice(0, 5).join(" | ").slice(0, 600)}`); }

        const first = {}; const again = {};
        for (const text of ROUTE) {
            const of = (visit) => navigations.filter((n) => n.page === text && n.visit === visit);
            if (of("first").length) { first[text] = of("first")[0]; }
            if (of("again").length) {
                again[text] = Object.fromEntries(["frames", "shownMs", "endedMs", "paintedMs", ...GL_WORK_COUNTS].map((name) => [name, median(of("again").map((n) => n[name]))]));
            }
        }
        return { rendering, navigations, first, again };
    } finally {
        await page.close();
        if (address) { fs.rmSync(directory, { recursive: true, force: true }); }
    }
}

/** The switches of a result that are over the budget, as lines of text; none when it is kept. */
export function overBudget(result, budget = BUDGET) {
    const over = [];
    for (const visit of ["first", "again"]) {
        for (const [text, numbers] of Object.entries(result[visit])) {
            for (const [name, limit] of Object.entries(budget[visit][text] ?? {})) {
                if (numbers[name] > limit) {
                    over.push(`${text}, ${visit === "first" ? "first visit" : "shown again"}: ${name} ${numbers[name]}, the budget is ${limit}`);
                }
            }
        }
    }
    return over;
}

/** The lines of a table of a result: a row for each page and visit. */
export function table(result) {
    const columns = ["frames", "draws", "programs", "textures", "texturePixels", "masks", "maskPixels", "framebufferBinds", "clears", "shownMs", "endedMs", "paintedMs"];
    const rows = [["switch to", ...columns]];
    for (const [visit, label] of [["first", "first visit"], ["again", "shown again"]]) {
        for (const [text, numbers] of Object.entries(result[visit])) { rows.push([`${text}, ${label}`, ...columns.map((name) => String(numbers[name]))]); }
    }
    const widths = rows[0].map((_, column) => Math.max(...rows.map((row) => row[column].length)));
    return rows.map((row) => row.map((cell, column) => (column === 0 ? cell.padEnd(widths[column]) : cell.padStart(widths[column]))).join("  "));
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
    const argv = process.argv.slice(2);
    const options = { rounds: 3 }; let site; let json = false; let gate = false;
    for (let i = 0; i < argv.length; i++) {
        if (argv[i] === "--rounds") { options.rounds = Number(argv[++i]); } else if (argv[i] === "--angle") { options.angle = argv[++i]; } else if (argv[i] === "--query") { options.query = `&${argv[++i].replace(/^[?&]/, "")}`; } else if (argv[i] === "--size") { [options.width, options.height] = argv[++i].split("x").map(Number); } else if (argv[i] === "--slowdown") { options.slowdown = Number(argv[++i]); } else if (argv[i] === "--json") { json = true; } else if (argv[i] === "--gate") { gate = true; } else { site = argv[i]; }
    }
    if (!site || !(options.rounds >= 1)) {
        console.error("usage: node scripts/browser/navigation-cost.mjs <site directory | address> [--rounds n] [--size WxH] [--angle name] [--query text] [--slowdown n] [--json] [--gate]");
        process.exit(2);
    }
    if (!/^https?:/.test(site)) {
        site = path.resolve(site);
        if (!fs.existsSync(path.join(site, "index.html"))) {
            console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser`);
            process.exit(2);
        }
    }
    const result = await measure(site, options);
    const over = overBudget(result);
    if (json) { console.log(JSON.stringify({ site, ...result, budget: BUDGET, over })); } else {
        console.log(`${site}: ${JSON.stringify(result.rendering)}`);
        for (const line of table(result)) { console.log(line); }
        console.log(over.length ? `over the budget:\n  ${over.join("\n  ")}` : "every switch is within the budget");
    }
    process.exit(gate && over.length ? 1 : 0);
}
