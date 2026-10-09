// How much of the memory of the module the ControlCatalog site uses over a tour of its pages, in headless Chrome.
//
//   node scripts/browser/catalog-memory.mjs <site directory> [options]
//
//   --query <text>      more of the query string of the page, e.g. "&RenderThread=false"
//   --isolated          serve the site cross-origin isolated (a site built with threads)
//   --angle <backend>   what WebGL runs on: swiftshader (the default) or a backend of ANGLE with the GPU (metal)
//   --all               visit every page of the catalog, not the tour of thirteen (below)
//   --passes <n>        with --all: go through the catalog n times in the same session (default 1); the
//                       memory after each pass tells whether a page that is visited again costs again
//   --verbose           with --all: a line on the standard error for every page as it is visited
//   --json              print the result as JSON
//
// The tour is the one of catalog-pages.mjs (thirteen pages), at its size, in WebGL2, with the prefetch of
// the files of the pages on, as the last check of tests/control_catalog.test.mjs makes it. With --all it
// is the whole catalog (`visitAll` of catalog-pages.mjs): every page of the list the application is built
// from, each opened from the drawer, its content scrolled down and back (a virtualized list for thirty
// wheel events) and its demos opened that need no file and do not leave the page (`PAGE_ACTIONS`); the
// pages that could not be visited are listed with the reason. This is the session the fixed memory of a
// module built with threads is sized by (FERROUI_BROWSER_THREAD_MEMORY_MB of scripts/build-browser.sh).
// The numbers are
// those of the `catalogMemory` export of the host: the end of the dynamic memory of the module (everything
// the module uses lies below it: its data, the stacks and what the allocator took from the system), its
// largest value (the page asks ten times a second with `?MemoryReport=true`) and the size of the memory
// (fixed in a module built with threads, grown to in one built without). What the browser holds outside
// the memory of the module (the WebGL contexts, the canvases, the compiled code) is not in them.
import fs from "node:fs";
import path from "node:path";
import { open, sleep } from "./harness.mjs";
import { ALL_SIZE, TOUR, TOUR_SIZE, catalogPages, drive, pairs, visit, visitAll, waitUntilReady } from "./catalog-pages.mjs";

const args = process.argv.slice(2);
const valued = new Set(["--query", "--angle", "--passes"]);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
if (!site || !fs.existsSync(path.join(site, "index.html"))) {
    console.error("usage: node scripts/browser/catalog-memory.mjs <site directory> [--query q] [--isolated] [--angle a] [--all [--passes n] [--verbose]] [--json]");
    process.exit(2);
}
const query = option("--query", "");
const all = args.includes("--all");
const passes = Number(option("--passes", "1"));
const megabytes = (bytes) => Number((Number(bytes) / (1024 * 1024)).toFixed(1));

const page = await open(site, {
    query: `?RenderingMode=WebGL2&MemoryReport=true${query}`, ...(all ? ALL_SIZE : TOUR_SIZE), isolated: args.includes("--isolated"), angle: option("--angle")
});
try {
    await waitUntilReady(page, 180_000);
    drive(page);
    const memory = async () => pairs(await page.evaluate("controlCatalog.catalogMemory()"));
    const atStart = await memory();
    const visited = []; let notVisited = []; const afterPass = [];
    if (all) {
        for (let pass = 0; pass < passes; pass++) {
            const tour = await visitAll(page, {
                // A later pass leaves out what the first could not visit.
                ...(pass > 0 ? { pages: catalogPages().filter((entry) => !notVisited.some((missing) => missing.name === entry.name)) } : {}),
                again: pass > 0,
                after: async () => megabytes((await memory()).top),
                log: args.includes("--verbose") ? (line) => console.error(`pass ${pass + 1}${line}`) : undefined
            });
            // The pages of the first pass; of a later one only what the memory was after it.
            if (pass === 0) {
                visited.push(...tour.visited.map((entry) => ({ page: entry.name, topMb: entry.result, ...(entry.missed.length ? { missed: entry.missed } : {}) })));
                notVisited = tour.notVisited;
            } else {
                notVisited.push(...tour.notVisited.map((entry) => ({ ...entry, reason: `in pass ${pass + 1}: ${entry.reason}` })));
            }
            afterPass.push(megabytes((await memory()).top));
        }
    } else {
        for (const entry of TOUR) {
            await visit(page, entry);
            // The page is drawn, and its files have arrived or are on their way.
            await sleep(700);
            visited.push({ page: entry.name, topMb: megabytes((await memory()).top) });
        }
    }
    // The files of the pages that were not visited arrive too (the prefetch is on).
    await page.waitFor(`performance.getEntriesByType("mark").some((m) => m.name === "assets prefetched")`, 120_000);
    await sleep(1000);
    const atEnd = await memory();
    const rendering = await page.rendering();
    const result = {
        renderThread: rendering.on_render_thread === "true" && rendering.other_thread === "true",
        startMb: megabytes(atStart.top), endMb: megabytes(atEnd.top), peakMb: megabytes(atEnd.peak), sizeMb: megabytes(atEnd.size),
        ...(passes > 1 ? { afterPassMb: afterPass } : {}), pages: visited, notVisited, errors: page.errors
    };
    if (args.includes("--json")) { console.log(JSON.stringify(result)); } else {
        console.log(`${site}${query ? ` (${query})` : ""}, drawn by ${result.renderThread ? "a render thread" : "the thread of the page"}: memory of the module in use at the start ${result.startMb} MB, ` +
            `at the end ${result.endMb} MB, peak ${result.peakMb} MB, of ${result.sizeMb} MB`);
        console.log(`  after each of ${visited.length} pages, MB: ${visited.map((v) => `${v.page} ${v.topMb}`).join(", ")}`);
        if (passes > 1) { console.log(`  after each of ${passes} passes, MB: ${afterPass.join(", ")}`); }
        const missed = visited.filter((v) => v.missed);
        if (missed.length > 0) { console.log(`  demos that were not shown: ${missed.map((v) => `${v.page}: ${v.missed.join(", ")}`).join("; ")}`); }
        if (notVisited.length > 0) { console.log(`  not visited:\n${notVisited.map((v) => `    ${v.section} / ${v.name}: ${v.reason}`).join("\n")}`); }
        if (page.errors.length > 0) { console.log(`errors:\n${page.errors.join("\n")}`); }
    }
} finally {
    await page.close();
}
