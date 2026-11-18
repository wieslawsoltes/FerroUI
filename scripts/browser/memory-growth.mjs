// Counts the growths of the WebAssembly memory of a site up to its first frame and after it
// (#760, docs/porting/browser-platform.md, "Memory at the start").
//
//   node scripts/browser/memory-growth.mjs <site directory> [--query q] [--runs n] [--settle ms] [--json]
//
// `MEMORY_GROWTH_PROBE` is an init script (`open(.., { initScript: MEMORY_GROWTH_PROBE })`): it
// wraps `WebAssembly.Memory.prototype.grow`, which Emscripten's `emscripten_resize_heap` calls
// when the heap has to grow, and keeps in `globalThis.__ferroMemoryGrowth`:
//
// - `growths`, `pages`, `ms`: the calls, the pages of 64 KiB they added and the time spent in them
//   (the call itself; Emscripten renews its views of the memory after it, which is not counted);
// - `first`: the size of the memory in bytes when it first grew, `last`: its size after the last
//   growth (the size of the memory of the module now, when it grew at all);
// - `atView`: the counts when the first frame of the view was drawn (the splash closed).
//
// A module built with threads has a memory that does not grow (`FERROUI_BROWSER_THREAD_MEMORY_MB`
// of scripts/build-browser.sh): its counts are zero, and a growth there is a failure.

import fs from "node:fs";
import path from "node:path";
import { open, sleep } from "./harness.mjs";

export const MEMORY_GROWTH_PROBE = `(() => {
    const growth = globalThis.__ferroMemoryGrowth = { growths: 0, pages: 0, ms: 0, first: null, last: null, atView: null };
    const prototype = globalThis.WebAssembly?.Memory?.prototype;
    if (!prototype || !prototype.grow) { return; }
    const grow = prototype.grow;
    prototype.grow = function (pages) {
        if (growth.first === null) { growth.first = this.buffer.byteLength; }
        const started = performance.now();
        const result = grow.call(this, pages);
        growth.ms += performance.now() - started;
        growth.growths++;
        growth.pages += Number(pages) || 0;
        growth.last = this.buffer.byteLength;
        return result;
    };
    new MutationObserver(() => {
        if (growth.atView || !document.querySelector(".ferroui-splash.splash-close")) { return; }
        growth.atView = { growths: growth.growths, pages: growth.pages, ms: growth.ms, last: growth.last };
    }).observe(document, { subtree: true, attributes: true, attributeFilter: ["class"] });
})();`;

/** The counts of the page since its load. */
export async function readMemoryGrowth(page) {
    return JSON.parse(await page.evaluate("JSON.stringify(globalThis.__ferroMemoryGrowth ?? null)"));
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname);
if (isMain) {
    const args = process.argv.slice(2);
    const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
    const valued = new Set(["--query", "--runs", "--settle"]);
    const site = args.find((a, i) => !a.startsWith("--") && !valued.has(args[i - 1]));
    if (!site || !fs.existsSync(path.join(site, "index.html"))) {
        console.error("usage: node scripts/browser/memory-growth.mjs <site directory> [--query q] [--runs n] [--settle ms] [--json]");
        process.exit(2);
    }
    const runs = Number(option("--runs", "3"));
    const settle = Number(option("--settle", "3000"));
    const results = [];
    for (let run = 0; run < runs; run++) {
        const page = await open(site, { query: option("--query", ""), initScript: MEMORY_GROWTH_PROBE });
        try {
            await page.waitForView();
            await sleep(settle);
            results.push(await readMemoryGrowth(page));
        } finally {
            await page.close();
        }
    }
    if (args.includes("--json")) {
        console.log(JSON.stringify(results, null, 2));
    } else {
        const mb = (bytes) => (bytes === null || bytes === undefined ? "-" : (bytes / 1048576).toFixed(1));
        for (const [index, result] of results.entries()) {
            const view = result.atView ?? { growths: 0, pages: 0, ms: 0, last: null };
            console.log(`run ${index + 1}: to the first frame ${view.growths} growths (${view.pages} pages, ${view.ms.toFixed(2)} ms), ` +
                `${view.last ? `memory ${mb(view.last)} MB` : "the memory it was instantiated with"}; after ${settle} ms ${result.growths} growths ` +
                `(${result.pages} pages, ${result.ms.toFixed(2)} ms), ` +
                (result.growths ? `first grown from ${mb(result.first)} MB, now ${mb(result.last)} MB` : "never grown"));
        }
    }
}
