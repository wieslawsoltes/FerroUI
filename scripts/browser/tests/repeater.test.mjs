// Rendered ItemsRepeater regressions in the ControlCatalog site, driven by real pointer/wheel input.
//
//   scripts/build-browser.sh control-catalog-browser
//   node scripts/browser/tests/repeater.test.mjs [<site directory>] [--screenshot-dir <directory>]
//       [--mode WebGL2|Software2D] [--query <parameters>]
//
// Runs both rendering modes by default. A threaded site is recognised by ferroui-threads.js and is
// served cross-origin isolated, like control_catalog.test.mjs. No direct control mutation or test-only
// host export is used: the page's buttons, visible text/bounds, realization counters and drawn-frame
// acknowledgements are enough to check its four layouts, recycling and notifying collection changes.
// Screenshots include the laid-out cards, a scrolled viewport and every collection mutation. They are
// evidence from the supplied build; passing this test does not establish parity with every upstream
// Repeater scenario.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { assert, open, run } from "../harness.mjs";
import { DRAWER_EDGE, drive, inContent, visit, waitUntilReady } from "../catalog-pages.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const args = process.argv.slice(2);
let siteArgument;
let screenshotDirectory;
let extraQuery = "";
let modes = ["WebGL2", "Software2D"];
for (let i = 0; i < args.length; i++) {
    const argument = args[i];
    if (["--screenshot-dir", "--mode", "--query"].includes(argument)) {
        const value = args[++i];
        if (!value || value.startsWith("--")) { throw new Error(`missing value for ${argument}`); }
        if (argument === "--screenshot-dir") { screenshotDirectory = path.resolve(value); }
        if (argument === "--query") { extraQuery = value.replace(/^[?&]/, ""); }
        if (argument === "--mode") {
            assert(["WebGL2", "Software2D"].includes(value), `unknown rendering mode ${value}`);
            modes = [value];
        }
    } else {
        assert(!argument.startsWith("--") && siteArgument === undefined, `unexpected argument ${argument}`);
        siteArgument = argument;
    }
}
const site = path.resolve(siteArgument ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser", "control-catalog-browser"));
assert(fs.existsSync(path.join(site, "index.html")), `no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser`);
const threaded = fs.existsSync(path.join(site, "ferroui-threads.js"));
const SIZE = { width: 1280, height: 1000 };
const START_TIMEOUT = 180_000;
const FRAME_TIMEOUT = 60_000;
const ITEM_COUNT = 10_000;
// ItemsRepeaterPage.xaml: the status stretches across row 2, then a 12-DIP row gap and the
// row-3 Border inset (1 border + 12 padding) surround the scroller. The bare ContentPage fills
// the fixed-height browser view, with no bottom command bar or extra content padding.
const PAGE_ROW_SPACING = 12;
const SCROLLER_BORDER = 1;
const SCROLLER_PADDING = 12;
const layouts = [
    { button: "StackLayoutButton", name: "stack", columns: false },
    { button: "UniformGridLayoutButton", name: "uniform-grid", columns: true },
    { button: "FlowLayoutButton", name: "flow", columns: true },
    { button: "LinedFlowLayoutButton", name: "lined-flow", columns: true }
];
const report = {
    site, query: extraQuery, threaded, started: new Date().toISOString(),
    viewportDerivation: {
        source: "ItemsRepeaterPage.xaml and the measured RealizationStatus bounds; the host does not export ScrollViewer bounds",
        rowSpacing: PAGE_ROW_SPACING, border: SCROLLER_BORDER, padding: SCROLLER_PADDING, pageBottom: SIZE.height
    },
    runs: []
};

/** The page's counters, updated after collection changes and layout. */
function counters(state) {
    const text = state.elements.find((element) => element.name === "RealizationStatus")?.text;
    const match = /^(\d+) items · (\d+) realized · (\d+) prepared · (\d+) cleared$/.exec(text ?? "");
    return match ? Object.fromEntries(["items", "realized", "prepared", "cleared"].map((name, i) => [name, Number(match[i + 1])])) : null;
}

function itemViewport(state) {
    // catalogState exports TextBlock/TextBox/Button/Image, not the named RepeaterScroller. Derive
    // this fixed sample's scroller from exported layout geometry and its declared XAML insets.
    const status = state.elements.find((element) => element.name === "RealizationStatus");
    const inset = SCROLLER_BORDER + SCROLLER_PADDING;
    return status && status.width > 2 * inset && status.height > 0 ? {
        left: status.x + inset,
        top: status.y + status.height + PAGE_ROW_SPACING + inset,
        right: status.x + status.width - inset,
        bottom: SIZE.height - inset
    } : null;
}

/** Item labels whose centres are inside the scroller, ordered as they appear in the layout. */
function visibleItems(state) {
    const viewport = itemViewport(state);
    if (!viewport) { return []; }
    return state.elements.filter((element) => element.type === "TextBlock" && inContent(element)
        && /^(?:Item \d{5}|Inserted item \d+)$/.test(element.text ?? "")
        // catalogState's hit flag can include buffered rows clipped by this ancestor. A hit alone
        // must not make an offscreen label overlap the toolbar in our visibility or pixel checks.
        && element.x + element.width / 2 >= viewport.left && element.x + element.width / 2 < viewport.right
        && element.y + element.height / 2 >= viewport.top && element.y + element.height / 2 < viewport.bottom)
        .sort((a, b) => a.y - b.y || a.x - b.x);
}
const hasItem = (state, text) => visibleItems(state).some((item) => item.text === text);

function assertVirtualized(state) {
    const counts = counters(state);
    assert(counts !== null, "the page did not report its realization counters");
    const visible = visibleItems(state);
    assert(visible.length > 0, "no item label is inside the drawn viewport");
    // The sample's cards are at least 128 DIPs wide and 64 high. VerticalCacheLength=2 extends the
    // realization window to at most three viewport heights; allow boundary/anchor rows on both ends.
    // Use the whole window as an upper bound, including its toolbar and drawer, so this does not rely
    // on a particular theme's padding or text metrics.
    const limit = (Math.ceil(SIZE.height * 3 / 64) + 4) * Math.ceil(SIZE.width / 128);
    assert(counts.realized >= visible.length && counts.realized <= limit,
        `expected a bounded realization window (${visible.length}..${limit} items), got ${JSON.stringify(counts)}`);
    assert(counts.realized < counts.items, `the entire source was realized: ${JSON.stringify(counts)}`);
    return counts;
}

async function shown(page, description, predicate) {
    await page.until(description, predicate);
    await page.drawn(FRAME_TIMEOUT);
    return page.until(description, predicate);
}

async function click(page, name) {
    await page.clickElement(await page.element(`button ${name} is drawn`, (element) => element.name === name && inContent(element)));
}

/** Reads a screenshot only after a frame requested for the observed state has been drawn. */
async function capture(page, mode, name, state, modeReport) {
    await page.drawn(FRAME_TIMEOUT);
    const file = screenshotDirectory ? path.join(screenshotDirectory, `${mode.toLowerCase()}-${name}.png`) : undefined;
    const picture = await page.screenshot(file);
    const viewport = itemViewport(state);
    assert(viewport !== null, `${mode}/${name}: the sample's scroller viewport could not be derived`);
    const first = visibleItems(state)[0];
    const probe = visibleItems(state).find((item) => item.x >= viewport.left && item.y >= viewport.top
        && item.x + item.width <= viewport.right && item.y + item.height <= viewport.bottom);
    assert(probe !== undefined, `${mode}/${name}: no item's complete text bounds are inside the scroller`);
    const colours = new Set();
    for (let y = Math.max(0, Math.floor(probe.y)); y < Math.min(picture.height, Math.ceil(probe.y + probe.height)); y++) {
        for (let x = Math.max(0, Math.floor(probe.x)); x < Math.min(picture.width, Math.ceil(probe.x + probe.width)); x++) {
            colours.add(picture.pixel(x, y).join(","));
        }
    }
    assert(colours.size > 4, `${mode}/${name}: the visible item's text bounds contain only ${colours.size} colours; its glyphs were not drawn`);
    // The initial stack's text bounds span the same horizontal viewport as every other layout.
    // Inspect labels even when their centres fail hit testing: a partially clipped last column can
    // still appear in catalogState, while filtering by `hit` alone would silently omit that bug.
    const edges = modeReport.itemTextBounds;
    const overflowing = state.elements.find((element) => element.type === "TextBlock"
        && /^(?:Item \d{5}|Inserted item \d+)$/.test(element.text ?? "")
        && element.x >= DRAWER_EDGE && element.y >= first.y - 1
        && element.y + element.height <= SIZE.height - 13
        && (element.x < edges.left - 1 || element.x + element.width > edges.right + 1));
    assert(!overflowing, `${mode}/${name}: item text exceeds the horizontal viewport ${JSON.stringify(edges)}: ${JSON.stringify(overflowing)}`);
    const indexes = visibleItems(state).filter((item) => /^Item \d{5}$/.test(item.text))
        .map((item) => Number(item.text.slice(5)));
    for (let i = 1; i < indexes.length; i++) {
        assert(indexes[i] === indexes[i - 1] + 1,
            `${mode}/${name}: the drawn rows omit or repeat an item between ${indexes[i - 1]} and ${indexes[i]}`);
    }
    const counts = assertVirtualized(state);
    modeReport.stages.push({
        name, ...counts, viewport,
        glyphProbe: { text: probe.text, bounds: { x: probe.x, y: probe.y, width: probe.width, height: probe.height }, colours: colours.size },
        visible: visibleItems(state).map(({ text, x, y, width, height }) => ({ text, x, y, width, height })),
        ...(file ? { screenshot: file } : {})
    });
    console.log(`      ${mode}/${name}: ${counts.items} items, ${counts.realized} realized, ${counts.prepared} prepared, ${counts.cleared} cleared`);
    return picture;
}

/** Counts changed pixels only in the cards, excluding buttons, their focus and the status counters. */
function changedCards(before, after, firstItem) {
    let changed = 0;
    const top = Math.max(0, Math.floor(firstItem.y - 16));
    for (let y = top; y < Math.min(before.height, after.height) - 24; y++) {
        for (let x = DRAWER_EDGE + 48; x < Math.min(before.width, after.width) - 48; x++) {
            if (before.pixel(x, y).some((channel, i) => Math.abs(channel - after.pixel(x, y)[i]) > 8)) { changed++; }
        }
    }
    return changed;
}

async function exercise(mode) {
    const query = new URLSearchParams(extraQuery);
    query.set("RenderingMode", mode);
    const page = await open(site, { query: `?${query}`, ...SIZE, isolated: threaded });
    const modeReport = { mode, stages: [] };
    report.runs.push(modeReport);
    try {
        await waitUntilReady(page, START_TIMEOUT);
        drive(page);
        await visit(page, { section: "Collections & Data", name: "ItemsRepeater" }, SIZE.height);
        let state = await shown(page, "the initial 10,000 items are drawn", (s) => counters(s)?.items === ITEM_COUNT && hasItem(s, "Item 00000"));
        const initialItem = visibleItems(state)[0];
        modeReport.itemTextBounds = { left: initialItem.x, right: initialItem.x + initialItem.width };
        const rendering = await page.rendering();
        assert(rendering.kind === (mode === "WebGL2" ? "webgl" : "software") && rendering.gl === (mode === "WebGL2" ? "3" : "0"),
            `the requested renderer was not used: ${JSON.stringify(rendering)}`);
        modeReport.rendering = rendering;
        modeReport.browser = await page.browser();

        for (const layout of layouts) {
            const before = counters(state);
            await click(page, layout.button);
            state = await shown(page, `${layout.name} lays out the first two items`, (s) => {
                const items = visibleItems(s);
                const first = items.find((item) => item.text === "Item 00000");
                const second = items.find((item) => item.text === "Item 00001");
                const counts = counters(s);
                // The initial stack button uses a different layout instance too. Wait for ownership
                // to change as well as geometry: the three wrapping layouts can share the first row.
                return counts && counts.prepared > before.prepared && counts.cleared > before.cleared
                    && first && second && (layout.columns
                        ? Math.abs(first.y - second.y) < 1 && second.x > first.x + 1
                        : Math.abs(first.x - second.x) < 1 && Math.abs(second.y - first.y - 72) < 1);
            });
            await capture(page, mode, layout.name, state, modeReport);
        }

        await click(page, "StackLayoutButton");
        state = await shown(page, "stack layout returns to the first item", (s) => hasItem(s, "Item 00000")
            && visibleItems(s).every((item) => Math.abs(item.x - visibleItems(s)[0].x) < 1));
        const atTop = await capture(page, mode, "stack-before-scroll", state, modeReport);
        const beforeScroll = counters(state);
        const wheelAt = { x: SIZE.width - 120, y: SIZE.height - 140 };
        await page.wheel(wheelAt.x, wheelAt.y, 0, 7200);
        state = await shown(page, "wheel input scrolls and recycles the initial items", (s) => {
            const counts = counters(s);
            return counts && counts.prepared > beforeScroll.prepared && counts.cleared > beforeScroll.cleared
                && !hasItem(s, "Item 00000") && visibleItems(s).some((item) => Number(item.text.slice(5)) >= 50);
        });
        const scrolled = await capture(page, mode, "scrolled", state, modeReport);
        assert(changedCards(atTop, scrolled, visibleItems(state)[0]) > 0, "wheel input changed the item state but no pixels of its viewport");
        await page.wheel(wheelAt.x, wheelAt.y, 0, -1_000_000);
        state = await shown(page, "scrolling back restores the first item", (s) => hasItem(s, "Item 00000"));
        const beforeInsert = await capture(page, mode, "before-insert", state, modeReport);
        const firstBeforeInsert = visibleItems(state)[0];
        await click(page, "InsertItemButton");
        state = await shown(page, "insert-first adds the new item and changes the count", (s) => counters(s)?.items === ITEM_COUNT + 1 && hasItem(s, "Inserted item 10000"));
        assert(visibleItems(state)[0].text === "Inserted item 10000", "the inserted item was not drawn first");
        const afterInsert = await capture(page, mode, "inserted", state, modeReport);
        assert(changedCards(beforeInsert, afterInsert, firstBeforeInsert) > 0, "insert changed the source and text state but no pixels of its viewport");

        await click(page, "MoveItemButton");
        state = await shown(page, "move-first-to-end restores the original first item", (s) => counters(s)?.items === ITEM_COUNT + 1
            && visibleItems(s)[0]?.text === "Item 00000" && !hasItem(s, "Inserted item 10000"));
        await capture(page, mode, "moved-from-start", state, modeReport);
        await page.wheel(wheelAt.x, wheelAt.y, 0, 1_000_000);
        state = await shown(page, "the moved item is drawn after the former last item", (s) => hasItem(s, "Item 09999") && hasItem(s, "Inserted item 10000"));
        assert(visibleItems(state).at(-1).text === "Inserted item 10000", "move did not put the inserted item at the end");
        await capture(page, mode, "moved-to-end", state, modeReport);

        await click(page, "RemoveItemButton");
        await shown(page, "remove-first changes the collection count", (s) => counters(s)?.items === ITEM_COUNT);
        await page.wheel(wheelAt.x, wheelAt.y, 0, -1_000_000);
        state = await shown(page, "remove-first removes Item 00000", (s) => visibleItems(s)[0]?.text === "Item 00001" && !hasItem(s, "Item 00000"));
        await capture(page, mode, "removed", state, modeReport);

        await click(page, "ResetItemsButton");
        state = await shown(page, "reset restores the original source", (s) => counters(s)?.items === ITEM_COUNT && visibleItems(s)[0]?.text === "Item 00000");
        await capture(page, mode, "reset", state, modeReport);
        assert(page.errors.length === 0, `browser errors were logged:\n${page.errors.join("\n")}`);
        assert((await page.rendering()).panics === "0", "the render thread reported a panic");
        modeReport.passed = true;
    } catch (error) {
        modeReport.passed = false;
        modeReport.error = String(error.message ?? error);
        if (screenshotDirectory) { await page.screenshot(path.join(screenshotDirectory, `${mode.toLowerCase()}-failure.png`)); }
        error.message += `\n${page.log.slice(-8).join("\n")}`;
        throw error;
    } finally {
        await page.close();
        if (screenshotDirectory) {
            fs.mkdirSync(screenshotDirectory, { recursive: true });
            fs.writeFileSync(path.join(screenshotDirectory, "repeater-report.json"), `${JSON.stringify(report, null, 2)}\n`);
        }
    }
}

await run(modes.map((mode) => [`${mode}: Repeater layouts, rendering, recycling and collection mutations`, () => exercise(mode)]));
