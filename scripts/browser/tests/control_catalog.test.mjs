// Tests of the ControlCatalog site in headless Chrome.
//
//   scripts/build-browser.sh control-catalog-browser
//   node scripts/browser/tests/control_catalog.test.mjs [<site directory>] [--screenshot <file.png>]
//
// The site directory defaults to target/browser/control-catalog-browser. The checks:
//
// - no file of the site (the asset files included) carries a brand asset of the upstream project that
//   samples/ControlCatalog/PlaceholderAssets replaces;
// - once for WebGL2 (`?RenderingMode=WebGL2`) and once for the 2D canvas (`?RenderingMode=Software2D`):
//   the application starts (the splash is closed and the view has a canvas), the canvas has the
//   context of the mode, the main view is drawn (the canvas is not blank), the view is laid out again
//   when the page is made larger (the area the larger size adds is drawn) and smaller, and nothing is
//   logged as an error (console errors, uncaught exceptions, failed loads);
// - the module, the list of the asset files and the start-up asset files are downloaded once each,
//   through the preloads of the host page, and the page preloads no other asset file;
// - input, driven with real pointer and key events: the navigation drawer opens from its toggle
//   button, three pages are reached through the drawer and show their content, a click on a button
//   has its effect, and text typed into a text box becomes its text;
// - the native control demo (samples/ControlCatalog.Browser/embed_sample_browser.rs): the Native Embed
//   page shows its two native controls, elements of the page over the view (a button of the page that
//   counts the clicks it gets from real pointer events, and an iframe), and a check box of the page hides
//   one of them;
// - the asset files (samples/ControlCatalog.Browser/wwwroot/page-assets.js): the start-up downloads the
//   start-up files only, the files of the pages follow the first frame, every file is downloaded once,
//   and nothing requests a bundle (`.assets`); with that prefetch off (`?PrefetchAssets=false`), opening
//   the Container Queries page downloads exactly its files and its images have their bitmaps and are
//   drawn, and the CJK sample of the TextBox page finds its font (the typeface the font manager resolves
//   is WenQuanYi Micro Hei) and draws its glyphs.
//
// The view is read through the `catalogState` export of the host (samples/ControlCatalog.Browser),
// which reports the drawer, the current page, the focus and the visible text with its bounds; the
// tests find the controls they click from those bounds.
//
// The same checks run against a site built with threads, which is recognised by the file the build
// adds to such a site (ferroui-threads.js) and served cross-origin isolated:
//
//   scripts/build-browser.sh control-catalog-browser --threads
//   node scripts/browser/tests/control_catalog.test.mjs target/browser-threads/control-catalog-browser
//
// There the catalog is rendered by a render thread (docs/porting/browser-render-worker.md, "B2.7"),
// and more checks follow: which thread drew the frames (the `catalogRendering` export), the same
// module kept on one thread (`?RenderThread=false`), and a frame that panics on the render thread,
// which the page has to see as an error while the frames go on.
//
// A view is hit from what its last frame drew, and with a render thread the first frame comes after
// the view exists: every page is waited for until something of its view is hit before a check
// sends input (`hit` of the elements of `catalogState`). The same holds for every later frame: the
// state is what the application has laid out, and a click lands on what the view has drawn. A check
// takes the element it clicks from `page.find` or `page.element`, which wait until the view has drawn
// the state they read (`catalogRequestFrame` and `catalogFrameDrawn` of the host; see
// ../catalog-pages.mjs).
//
// The last check visits a tour of pages and prints how much of the memory of the module was in use
// (the `catalogMemory` export): the number a site built with threads, whose memory is fixed, is
// sized by.
//
// With --screenshot the picture of the WebGL2 run at the first size is written to the given file.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SPLASH_PROBE, open, run, assert, sleep, splashFrames } from "../harness.mjs";
import {
    DRAWER_EDGE, STATE_TIMEOUT, TOUR, TOUR_SIZE, describe, drive, inContent, inDrawer, navigate, pairs, visit,
    waitUntilReady
} from "../catalog-pages.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const argv = process.argv.slice(2);
let screenshotFile;
const positional = [];
for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--screenshot") { screenshotFile = path.resolve(argv[++i]); } else { positional.push(argv[i]); }
}
const site = path.resolve(positional[0] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser", "control-catalog-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser`);
    process.exit(2);
}

// A site built with threads has the script that isolates its page.
const threaded = fs.existsSync(path.join(site, "ferroui-threads.js"));
// The parameter that keeps a module built with threads on the thread of the page.
const ONE_THREAD = "RenderThread=false";
const measured = (text) => console.log(`      measured: ${text}`);

// The list of the asset files of the site (samples/ControlCatalog/build/page_files.rs).
const MANIFEST = "assets/ControlCatalog.json";
const manifest = JSON.parse(fs.readFileSync(path.join(site, MANIFEST), "utf8"));
/** The file of the site of the asset with the rooted path `asset`. */
const fileOf = (asset) => `${manifest.directory}${asset}`;

// Start-up includes compiling the WebAssembly module, which takes a while on a cold CI runner.
const START_TIMEOUT = 180_000;
const FRAME_TIMEOUT = 60_000;
const FIRST_SIZE = { width: 1024, height: 700 };
const LARGER_SIZE = { width: 1440, height: 900 };
const SMALLER_SIZE = { width: 800, height: 600 };
// Below 640 pixels the drawer of the main view is an overlay that starts closed.
const NARROW_SIZE = { width: 600, height: 700 };

/** Opens the site in a page of `size` and waits until the application has started and drawn. */
async function start({ mode = "WebGL2", size = FIRST_SIZE, query = "" } = {}) {
    const page = await open(site, { query: `?RenderingMode=${mode}${query}`, width: size.width, height: size.height, isolated: threaded, initScript: SPLASH_PROBE });
    const started = Date.now();
    // Whether a render thread draws the view: a module built with threads, unless the page keeps it
    // on its own thread.
    page.onRenderThread = threaded && !query.includes(ONE_THREAD);
    try {
        await waitUntilReady(page, START_TIMEOUT);
        // The splash closes on the first drawn frame, whichever thread drew it: not on an empty canvas.
        const splash = await splashFrames(page);
        assert(splash && splash.frames >= 1, `the splash was closed before the first frame of the view was drawn: ${JSON.stringify(splash)}`);
    } catch (error) { await page.close(); throw error; }
    page.startSeconds = (Date.now() - started) / 1000;
    drive(page);
    // The size of the canvas in device pixels and the device pixel ratio. A canvas whose control went
    // to a render thread is sized there: its size is the size of the last frame.
    page.canvasSize = async () => {
        const dpr = await page.evaluate("devicePixelRatio");
        if (page.onRenderThread) {
            const [width, height] = (await page.rendering()).size.split("x").map(Number);
            return { width, height, dpr };
        }
        return { ...await page.evaluate(`(c => ({ width: c.width, height: c.height }))(document.querySelector("#out canvas"))`), dpr };
    };
    return page;
}

/** The number of distinct colours in a region of a decoded picture. */
function distinctColours(picture, { x = 0, y = 0, width = picture.width - x, height = picture.height - y } = {}) {
    const colours = new Set();
    for (let row = y; row < y + height; row++) {
        for (let column = x; column < x + width; column++) { colours.add(picture.pixel(column, row).join(",")); }
    }
    return colours.size;
}

const checks = [];
/** Adds a check that runs `body` on a fresh page; the end of the log of the page is added to a failure. */
const check = (name, body, options) => checks.push([name, async () => {
    const page = await start(options);
    try { await body(page); } catch (error) {
        error.message += `\n${page.log.slice(-8).join("\n")}`;
        throw error;
    } finally { await page.close(); }
}]);

// --- branding --------------------------------------------------------------------------------------

/**
 * The site must not carry the brand assets of the upstream project that samples/ControlCatalog/PlaceholderAssets
 * replaces: neither the bytes of a replaced asset nor the path data of a replaced geometry resource may
 * appear in a file of the site.
 */
checks.push(["the site carries none of the replaced brand assets", async () => {
    const catalog = path.join(root, "samples", "ControlCatalog");
    const placeholders = path.join(catalog, "PlaceholderAssets");
    const published = fs.readdirSync(site, { recursive: true }).map((name) => path.join(site, name))
        .filter((file) => fs.statSync(file).isFile()).map((file) => fs.readFileSync(file));
    const shipped = (needle) => published.some((content) => content.indexOf(needle) >= 0);
    const found = [];
    let compared = 0;

    for (const name of fs.readdirSync(placeholders)) {
        const original = path.join(catalog, "Assets", name);
        if (!fs.existsSync(original) || !fs.statSync(original).isFile()) { continue; }
        // A run of bytes from the middle of the file: long enough to be unique to it.
        const bytes = fs.readFileSync(original);
        const length = Math.min(256, Math.floor(bytes.length / 2));
        const middle = Math.floor((bytes.length - length) / 2);
        compared++;
        if (shipped(bytes.subarray(middle, middle + length))) { found.push(`Assets/${name}`); }
    }

    const geometries = path.join(placeholders, "StreamGeometry");
    const documents = fs.readdirSync(catalog, { recursive: true }).filter((name) => name.endsWith(".xaml"))
        .map((name) => fs.readFileSync(path.join(catalog, name), "utf8"));
    for (const file of fs.readdirSync(geometries).filter((name) => name.endsWith(".txt"))) {
        const key = file.slice(0, -".txt".length);
        const opening = `<StreamGeometry x:Key="${key}">`;
        for (const document of documents) {
            for (let at = document.indexOf(opening); at >= 0; at = document.indexOf(opening, at + 1)) {
                const data = document.slice(at + opening.length, document.indexOf("</StreamGeometry>", at)).trim();
                compared++;
                if (shipped(Buffer.from(data))) { found.push(`the path data of the resource ${key}`); }
            }
        }
    }
    assert(compared > 0, "no replaced asset was found to compare");
    assert(found.length === 0, `the site carries ${found.join(", ")}`);
}]);

// --- rendering and resize, per rendering mode ---------------------------------------------------------

for (const [mode, expectedContext] of [["WebGL2", "webgl2"], ["Software2D", "2d"]]) {
    check(`${mode}: the application starts, draws with a ${expectedContext} context and is laid out again on resize`, async (page) => {
        console.log(`      started in ${page.startSeconds.toFixed(1)} s`);

        /** Waits until the canvas has the device-pixel size of `size` and the picture of `region` has enough colours. */
        async function waitForFrame(size, region, minimumColours) {
            let canvas; let colours = 0;
            for (const end = Date.now() + FRAME_TIMEOUT; Date.now() < end;) {
                canvas = await page.canvasSize();
                if (canvas.width === Math.round(size.width * canvas.dpr) && canvas.height === Math.round(size.height * canvas.dpr)) {
                    const picture = await page.screenshot(undefined, { x: 0, y: 0, ...size });
                    colours = distinctColours(picture, region);
                    if (colours >= minimumColours) { return { canvas, colours }; }
                }
                await sleep(250);
            }
            throw new Error(`at ${size.width}x${size.height} the canvas is ${canvas?.width}x${canvas?.height} with ${colours} colours (wanted ${minimumColours})`);
        }

        const first = await waitForFrame(FIRST_SIZE, undefined, 64);
        console.log(`      drawn at ${FIRST_SIZE.width}x${FIRST_SIZE.height}: canvas ${first.canvas.width}x${first.canvas.height}, ${first.colours} colours`);
        let kind;
        if (page.onRenderThread) {
            // The canvas of the page gave its control to the render thread and has no context to
            // ask for: the thread that draws tells what it drew the last frame with.
            const rendering = await page.rendering();
            kind = rendering.kind === "software" ? "2d" : rendering.kind === "webgl" ? (rendering.gl === "3" ? "webgl2" : "webgl") : "unknown";
            assert(rendering.other_thread === "true", `the frames were not drawn by another thread: ${JSON.stringify(rendering)}`);
        } else {
            // Asking a canvas for a context of another kind than the one it has gives null, so this
            // tells the kind without creating a context.
            kind = await page.evaluate(`(c => c.getContext("webgl2") ? "webgl2" : c.getContext("webgl") ? "webgl" : c.getContext("2d") ? "2d" : "unknown")(document.querySelector("#out canvas"))`);
        }
        assert(kind === expectedContext, `the canvas has a ${kind} context`);
        if (screenshotFile && mode === "WebGL2") {
            await page.screenshot(screenshotFile, { x: 0, y: 0, ...FIRST_SIZE });
            console.log(`      screenshot written to ${path.relative(root, screenshotFile)}`);
        }

        // The strip that only the larger size has, below the top and to the right of the first width.
        await page.resize(LARGER_SIZE.width, LARGER_SIZE.height);
        const added = { x: FIRST_SIZE.width + 8, y: 40, width: LARGER_SIZE.width - FIRST_SIZE.width - 16, height: LARGER_SIZE.height - 80 };
        const larger = await waitForFrame(LARGER_SIZE, added, 4);
        console.log(`      laid out at ${LARGER_SIZE.width}x${LARGER_SIZE.height}: ${larger.colours} colours in the added area`);
        await page.resize(SMALLER_SIZE.width, SMALLER_SIZE.height);
        await waitForFrame(SMALLER_SIZE, undefined, 64);

        // Let a few more frames run so that errors of the render loop show up.
        await sleep(1000);
        assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
    }, { mode });
}

// --- input ---------------------------------------------------------------------------------------------

check("the module, the list of the asset files and the start-up files are downloaded once, through the preloads of the page", async (page) => {
    // A preload that does not match the request of the script (another name, other credentials) is
    // not used, and the file is downloaded a second time.
    const requests = JSON.parse(await page.evaluate(`JSON.stringify(performance.getEntriesByType("resource")
        .map((e) => ({ file: decodeURIComponent(new URL(e.name).pathname.slice(1)), initiator: e.initiatorType })))`));
    for (const file of ["control_catalog_browser.wasm", MANIFEST, ...manifest.startup.map(fileOf)]) {
        const found = requests.filter((r) => r.file === file);
        assert(found.length === 1 && found[0].initiator === "link",
            `${file} was requested ${found.length} times (${found.map((r) => r.initiator).join(", ")}), expected once by its preload`);
    }
    const preloaded = requests.filter((r) => r.initiator === "link" && r.file.startsWith(`${manifest.directory}/`)).map((r) => r.file);
    assert(preloaded.length === manifest.startup.length, `the page preloads the asset files ${preloaded.join(", ")}, expected the start-up files`);
    assert(!page.log.some((line) => /preload/i.test(line)), `the browser reported a preload problem:\n${page.log.filter((line) => /preload/i.test(line)).join("\n")}`);
});

check("the navigation drawer opens from its toggle button", async (page) => {
    let state = await page.until("the drawer is closed", (s) => s.drawerOpen === false);
    assert(!state.elements.some((e) => e.name === "SearchBox" && e.hit), "the search box of the closed drawer can be reached");
    // The first frame of the view shows the drawer open beside the content: the main view closes it
    // for this width when it is loaded, after that frame. The button is clicked once the view has
    // drawn it where the closed drawer leaves it.
    const toggle = await page.element("the toggle button of the drawer is shown", (e) => e.name === "PART_BackButton" && e.hit);
    await page.clickElement(toggle);
    state = await page.until("the drawer is open and shows its entries", (s) => s.drawerOpen === true
        && s.elements.some((e) => e.name === "SearchBox" && inDrawer(e))
        && ["Home", "Basic Input", "Text", "Settings"].every((text) => s.elements.some((e) => e.text === text && inDrawer(e))));
    // The drawer covers the content: the home page behind it can no longer be clicked there.
    const covered = state.elements.find((e) => e.text === "Basic Input" && e.x < DRAWER_EDGE && !inDrawer(e));
    assert(!covered || !covered.hit, "the content behind the open drawer can still be clicked");
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
}, { size: NARROW_SIZE });

check("three pages are reached through the drawer and show their content", async (page) => {
    // Each section of the drawer opens the page of the section and shows the entries of its pages.
    // The section lower in the drawer comes first: an open section above it pushes its entries
    // out of the view at this height.
    await navigate(page, "Text");
    await navigate(page, "TextBox");
    await page.find("First Look", inContent);

    await navigate(page, "Basic Input");
    await navigate(page, "Buttons");
    await page.find("Click the first button to raise Click.", inContent);
    await page.find("Standard _button", inContent);

    await navigate(page, "CheckBox");
    await page.until("the check boxes of the page are shown", (s) => s.elements.filter((e) => e.type === "CheckBox" && inContent(e)).length >= 3);
    assert(!(await page.state()).elements.some((e) => e.text === "Click the first button to raise Click." && e.hit), "the previous page is still shown");
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
});

check("a click on a button raises its Click", async (page) => {
    await navigate(page, "Basic Input");
    await navigate(page, "Buttons");
    const button = await page.find("Standard _button", inContent);
    await page.clickElement(button);
    await page.find("Click raised 1 time.", inContent);
    await page.clickElement(button);
    await page.find("Click raised 2 times.", inContent);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
});

check("text typed into a text box becomes its text", async (page) => {
    await navigate(page, "Text");
    await navigate(page, "TextBox");
    // The card of the sample opens it on the navigation page.
    await page.clickElement(await page.find("First Look", inContent));
    // The sample is pushed with a transition; wait until it has ended before clicking into the page.
    await page.until("the text box of the sample is shown", (s) => !s.navigating && s.elements.some((e) => e.name === "FirstLookBox" && inContent(e)));
    const box = await page.element("the text box of the sample is shown", (e) => e.name === "FirstLookBox" && inContent(e));
    assert(box.text === "" || box.text === null, `the text box starts with "${box.text}"`);
    await page.clickElement(box);
    await page.until("the text box has the focus", (s) => s.focus?.type === "TextBox");
    await page.type("Ferro UI");
    let state = await page.until(`the text box holds "Ferro UI"`, (s) => s.focus?.text === "Ferro UI");
    assert(state.elements.find((e) => e.name === "FirstLookBox").text === "Ferro UI", "the text box of the sample does not hold the typed text");
    // The caption of the sample is bound to the text.
    await page.find("Text: Ferro UI", inContent);
    await page.press("Backspace"); await page.press("Backspace");
    state = await page.until(`Backspace removed two characters`, (s) => s.focus?.text === "Ferro ");
    await page.find("Text: Ferro ", inContent);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
});

check("the native controls of the Native Embed page are elements of the page over the view", async (page) => {
    await navigate(page, "Window & Platform");
    await page.clickElement(await page.find("Native Embed", inDrawer));
    await page.until("the Native Embed page is shown", (s) => s.page === "Native Embed" && !s.navigating);
    const controls = `JSON.stringify(Array.from(document.querySelector("#out .ferroui-native-host").children).map((e) => {
        const r = e.getBoundingClientRect();
        return { tag: e.tagName, display: e.style.display, src: e.src ?? null, text: e.innerText,
            buttons: e.querySelectorAll("button").length, x: r.x, y: r.y, width: r.width, height: r.height };
    }))`;
    await page.waitFor(`JSON.parse(${controls}).filter((c) => c.display === "block").length === 2`, STATE_TIMEOUT);
    // The native control host follows the bounds of its ancestors, not their render transforms (as
    // upstream): the controls were placed while the page slid in, at the start of the slide, right of
    // the view. A change of the bounds of the view places them again.
    await page.resize(LARGER_SIZE.width - 40, LARGER_SIZE.height);
    await page.waitFor(`JSON.parse(${controls}).every((c) => c.display === "block" && c.x + c.width <= ${LARGER_SIZE.width - 40} + 4)`, STATE_TIMEOUT);
    const [first, second] = JSON.parse(await page.evaluate(controls)).sort((a, b) => (a.tag === "DIV" ? -1 : 1) - (b.tag === "DIV" ? -1 : 1));
    // The first sample: the default native control of the platform with the button embed.js adds to it.
    assert(first.tag === "DIV" && first.buttons === 1 && first.text === "Hello world", `first native control ${JSON.stringify(first)}`);
    // The second: an iframe.
    assert(second.tag === "IFRAME" && second.src === "https://www.youtube.com/embed/kZCIporjJ70", `second native control ${JSON.stringify(second)}`);
    // Both lie over the content of the page, right of the drawer.
    assert(first.x >= DRAWER_EDGE && second.x >= DRAWER_EDGE && first.width > 1 && second.width > 1, `native controls at ${JSON.stringify([first, second])}`);

    // The button of the page gets real pointer events.
    const button = JSON.parse(await page.evaluate(`JSON.stringify((r => ({ x: r.x + r.width / 2, y: r.y + r.height / 2 }))(document.querySelector("#out .ferroui-native-host button").getBoundingClientRect()))`));
    await page.click(Math.round(button.x), Math.round(button.y));
    await page.waitFor(`document.querySelector("#out .ferroui-native-host button").innerText === "Click count 1"`, STATE_TIMEOUT);
    await page.click(Math.round(button.x), Math.round(button.y));
    await page.waitFor(`document.querySelector("#out .ferroui-native-host button").innerText === "Click count 2"`, STATE_TIMEOUT);

    // The check box above the first sample hides its native control.
    await page.drawn();
    const visible = (await page.state()).elements.filter((e) => e.text === "Visible" && inContent(e)).sort((a, b) => a.y - b.y || a.x - b.x)[0];
    assert(visible, "no check box of the page");
    await page.clickElement(visible);
    await page.waitFor(`(e => e && e.style.display === "none")(Array.from(document.querySelector("#out .ferroui-native-host").children).find((e) => e.tagName === "DIV"))`, STATE_TIMEOUT);
    assert(await page.evaluate(`Array.from(document.querySelector("#out .ferroui-native-host").children).find((e) => e.tagName === "IFRAME").style.display`) === "block",
        "the native control of the other sample was hidden too");

    // The iframe cannot reach its site from the test machine; that is not an error of the application.
    const errors = page.errors.filter((line) => !line.includes("youtube"));
    assert(errors.length === 0, `errors were logged:\n${errors.join("\n")}`);
}, { size: LARGER_SIZE });

// --- asset files ---------------------------------------------------------------------------------------

/** The asset files the page requested: from resource timing (asset paths), and from the log of page-assets.js (with the reason). */
async function fileRequests(page) {
    return JSON.parse(await page.evaluate(`JSON.stringify({
        resources: performance.getEntriesByType("resource").map((e) => ({ path: decodeURIComponent(new URL(e.name).pathname), start: e.startTime, end: e.responseEnd }))
            .filter((e) => e.path.startsWith("/${manifest.directory}/")).map((e) => ({ ...e, name: e.path.slice("/${manifest.directory}".length) })),
        bundles: performance.getEntriesByType("resource").map((e) => new URL(e.name).pathname).filter((p) => p.endsWith(".assets")),
        requests: controlCatalogAssets.requests,
        marks: Object.fromEntries(performance.getEntriesByType("mark").map((m) => [m.name, m.startTime])),
    })`));
}

check("the start-up downloads the start-up files only; the other files follow the first frame, each once", async (page) => {
    const others = Object.keys(manifest.files).filter((asset) => !manifest.startup.includes(asset));
    assert(others.length >= 20 && manifest.prefetch.length === others.length && new Set(manifest.prefetch).size === others.length,
        `the manifest lists ${others.length} files besides the start-up files, and prefetches ${manifest.prefetch.length}`);
    await page.waitFor(`performance.getEntriesByType("mark").some((m) => m.name === "assets prefetched")`, 120_000);
    const { resources, bundles, requests, marks } = await fileRequests(page);
    assert(bundles.length === 0, `asset bundles were requested: ${bundles.join(", ")}`);
    const firstFrame = marks["first frame"];
    assert(firstFrame > 0, "no first frame mark");
    const before = resources.filter((r) => r.start < firstFrame).map((r) => r.name).sort();
    assert(JSON.stringify(before) === JSON.stringify([...manifest.startup].sort()), `downloaded before the first frame: ${before.join(", ")}`);
    const counts = new Map();
    for (const r of resources) counts.set(r.name, (counts.get(r.name) ?? 0) + 1);
    const wrong = Object.keys(manifest.files).filter((asset) => counts.get(asset) !== 1).map((asset) => `${asset}: ${counts.get(asset) ?? 0}`);
    assert(wrong.length === 0 && counts.size === Object.keys(manifest.files).length, `files not downloaded once: ${wrong.join(", ")}`);
    const prefetched = requests.filter((r) => r.reason === "prefetch");
    assert(prefetched.length === others.length && prefetched.every((r) => r.start >= firstFrame),
        `prefetched ${prefetched.length} files, first frame at ${firstFrame}`);
    const startupBytes = manifest.startup.reduce((sum, asset) => sum + manifest.files[asset], 0);
    console.log(`      ${manifest.startup.length} start-up files, ${(startupBytes / 1e6).toFixed(2)} MB; first frame ${Math.round(firstFrame)} ms; ${others.length} files prefetched by ${Math.round(marks["assets prefetched"])} ms`);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
});

check("opening a page downloads its files and its images are drawn", async (page) => {
    await sleep(1500);
    let { resources } = await fileRequests(page);
    assert(resources.length === manifest.startup.length, `downloaded without prefetching: ${resources.map((r) => r.name).join(", ")}`);
    const expected = manifest.pages["Container Queries"];
    assert(expected?.length > 0, "the manifest lists no file of the Container Queries page");

    await navigate(page, "Layout");
    ({ resources } = await fileRequests(page));
    assert(resources.length === manifest.startup.length, `opening the section page downloaded ${resources.map((r) => r.name).join(", ")}`);
    await navigate(page, "Container Queries");
    const { requests, resources: after } = await fileRequests(page);
    const fetched = requests.filter((r) => r.reason === "page").map((r) => r.name).sort();
    const downloaded = after.map((r) => r.name).filter((name) => !manifest.startup.includes(name)).sort();
    assert(JSON.stringify(downloaded) === JSON.stringify(fetched), `the page downloaded ${downloaded.join(", ")}`);
    assert(JSON.stringify(fetched) === JSON.stringify([...expected].sort()), `the page fetched ${fetched.join(", ")}, expected ${expected.join(", ")}`);

    const state = await page.until("the images of the page have their bitmaps",
        (s) => s.elements.some((e) => e.type === "Image" && e.source && e.source.width > 0 && e.x >= DRAWER_EDGE && e.width > 40 && e.height > 40));
    const image = state.elements.find((e) => e.type === "Image" && e.source && e.x >= DRAWER_EDGE && e.width > 40 && e.height > 40);
    // A photograph has many colours; an empty image control has the one of the background.
    const region = { x: Math.ceil(image.x), y: Math.ceil(image.y), width: Math.floor(image.width) - 1, height: Math.min(Math.floor(image.height) - 1, FIRST_SIZE.height - Math.ceil(image.y) - 1) };
    let colours = 0;
    for (const end = Date.now() + FRAME_TIMEOUT; Date.now() < end && colours < 64; await sleep(250)) {
        colours = distinctColours(await page.screenshot(undefined, { x: 0, y: 0, ...FIRST_SIZE }), region);
    }
    assert(colours >= 64, `the image at ${JSON.stringify(region)} has ${colours} colours`);
    console.log(`      ${fetched.join(", ")} fetched on navigation; the image at ${region.x},${region.y} shows ${colours} colours`);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
}, { query: "&PrefetchAssets=false" });

check("the CJK sample of the TextBox page finds its font and draws its glyphs", async (page) => {
    await navigate(page, "Text");
    await navigate(page, "TextBox");
    const { requests } = await fileRequests(page);
    const fetched = requests.filter((r) => r.reason === "page").map((r) => r.name);
    assert(manifest.pages.TextBox.includes("/Assets/Fonts/WenQuanYiMicroHei-01.ttf") && manifest.pages.TextBox.every((name) => fetched.includes(name)),
        `the TextBox page fetched ${fetched.join(", ")}`);
    await page.clickElement(await page.find("Fonts and Complex Scripts", inContent));
    await page.until("the sample is shown", (s) => !s.navigating && s.elements.some((e) => e.text === "Complex scripts" || (e.type === "TextBox" && inContent(e))));
    const isCjk = (e) => e.type === "TextBox" && e.text?.startsWith("计算机科学") && e.x >= DRAWER_EDGE;
    let state = await page.state();
    // The sample is the last one of the page: scroll down to it.
    for (let i = 0; i < 40 && !state.elements.some((e) => isCjk(e) && e.y > 0 && e.y + 120 < LARGER_SIZE.height); i++) {
        await page.wheel(Math.round((DRAWER_EDGE + LARGER_SIZE.width) / 2), Math.round(LARGER_SIZE.height / 2), 0, 240);
        state = await page.state();
    }
    const box = state.elements.find(isCjk);
    assert(box, `no CJK text box in view; the view shows: ${describe(state)}`);
    assert(box.font === "WenQuanYi Micro Hei", `the CJK text box resolves the typeface ${JSON.stringify(box.font)}`);
    await sleep(500);
    // The glyphs: pixels that stand out from the background of the box (its most frequent colour).
    const picture = await page.screenshot(undefined, { x: 0, y: 0, ...LARGER_SIZE });
    const region = { x: Math.ceil(box.x) + 4, y: Math.max(0, Math.ceil(box.y) + 4), width: Math.min(Math.floor(box.width), 300) - 8, height: Math.min(80, LARGER_SIZE.height - Math.ceil(box.y) - 8) };
    const pixels = [];
    for (let y = region.y; y < region.y + region.height; y++) {
        for (let x = region.x; x < region.x + region.width; x++) { pixels.push(picture.pixel(x, y)); }
    }
    const counts = new Map();
    for (const pixel of pixels) { const key = pixel.slice(0, 3).join(","); counts.set(key, (counts.get(key) ?? 0) + 1); }
    const background = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0].split(",").map(Number);
    const luminance = ([r, g, b]) => 0.299 * r + 0.587 * g + 0.114 * b;
    const ink = pixels.filter((pixel) => Math.abs(luminance(pixel) - luminance(background)) > 80).length;
    assert(ink > 200, `the CJK text box at ${JSON.stringify(region)} has ${ink} pixels of glyphs`);
    console.log(`      the CJK text box uses ${box.font}; ${ink} pixels of glyphs`);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
}, { size: LARGER_SIZE, query: "&PrefetchAssets=false" });

// --- the render thread (stage B2.7): a site built with threads only --------------------------------

// Waits until the view reports more than `frames` frames.
async function framesAfter(page, frames, timeout = FRAME_TIMEOUT) {
    let rendering;
    for (const end = Date.now() + timeout; Date.now() < end; await sleep(50)) {
        rendering = await page.rendering();
        if (Number(rendering.frames) > Number(frames)) { return rendering; }
    }
    throw new Error(`no frame was drawn within ${timeout} ms: ${JSON.stringify(rendering)}`);
}

if (threaded) {
    check("the frames of the catalog are drawn by a render thread", async (page) => {
        let rendering = await page.rendering();
        assert(rendering.on_render_thread === "true", `the render loop of the page does not tick on a render thread: ${JSON.stringify(rendering)}`);
        assert(rendering.render_thread !== "0" && rendering.page_thread !== "0", `the threads have no ids: ${JSON.stringify(rendering)}`);
        assert(rendering.other_thread === "true" && rendering.frame_thread === rendering.render_thread && rendering.frame_thread !== rendering.page_thread,
            `the frames were not drawn by the render thread: ${JSON.stringify(rendering)}`);
        assert(rendering.kind === "webgl" && rendering.gl === "3", `the mode did not give a WebGL 2 target: ${JSON.stringify(rendering)}`);
        // The canvas of the page gave its control away: it hands out no context.
        const context = await page.evaluate(`(() => { try { return document.querySelector("#out canvas").getContext("2d") ? "2d" : "none"; } catch (error) { return error.name; } })()`);
        assert(context !== "2d", "the page still owns its canvas");
        // Input reaches the view and is drawn by that thread.
        await navigate(page, "Basic Input");
        rendering = await framesAfter(page, rendering.frames);
        assert(rendering.frame_thread === rendering.render_thread, `a frame after input was drawn by another thread: ${JSON.stringify(rendering)}`);
        measured(`${rendering.frames} frames in ${rendering.ticks} ticks of the render thread; calls a tick had the main thread serve: ${rendering.proxied} (last function ${rendering.last_proxied})`);
        assert(rendering.panics === "0", `the render thread panicked: ${JSON.stringify(rendering)}`);
        assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
    });

    check(`with ${ONE_THREAD} the module built with threads draws on the thread of the page`, async (page) => {
        const rendering = await page.rendering();
        assert(rendering.on_render_thread === "false" && rendering.render_thread === "0", `a render thread was started: ${JSON.stringify(rendering)}`);
        assert(Number(rendering.frames) > 0 && rendering.other_thread === "false" && rendering.ticks === "0",
            `the frames were not drawn by the thread of the page: ${JSON.stringify(rendering)}`);
        const kind = await page.evaluate(`(c => c.getContext("webgl2") ? "webgl2" : "other")(document.querySelector("#out canvas"))`);
        assert(kind === "webgl2", "the page did not keep its canvas");
        await navigate(page, "Basic Input");
        await navigate(page, "Buttons");
        await page.find("Standard _button", inContent);
        assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
    }, { query: `&${ONE_THREAD}` });

    // A frame that panics on the render thread is caught by the render loop there (as upstream's
    // loop catches the exception of a frame). Nothing of it would reach the page by itself: the
    // platform queues a call for the thread of the page, which logs the message as an error.
    check("a frame that panics on the render thread is an error of the page, and the frames go on", async (page) => {
        await navigate(page, "Basic Input");
        const before = await page.rendering();
        assert(page.errors.length === 0, `errors were logged before the panic:\n${page.errors.join("\n")}`);
        assert(await page.evaluate("controlCatalog.catalogPanicInFrame()") === true, "the job that panics was not posted");
        const reported = (line) => line.includes("the render thread panicked") && line.includes("catalogPanicInFrame");
        for (const end = Date.now() + STATE_TIMEOUT; Date.now() < end && !page.errors.some(reported); await sleep(100)) { }
        assert(page.errors.some(reported), `the page logged no error for the panic; its errors:\n${page.errors.join("\n")}\nits log ends:\n${page.log.slice(-6).join("\n")}`);
        const panics = await page.evaluate("JSON.stringify(controlCatalogPanics())");
        assert(JSON.parse(panics).length === 1 && JSON.parse(panics)[0].includes("catalogPanicInFrame"), `the script of the platform kept ${panics}`);
        assert((await page.rendering()).panics === "1", `the panic was not counted: ${JSON.stringify(await page.rendering())}`);
        // The loop goes on: the view answers input and draws.
        await navigate(page, "Buttons");
        const button = await page.find("Standard _button", inContent);
        await page.clickElement(button);
        await page.find("Click raised 1 time.", inContent);
        const after = await framesAfter(page, before.frames);
        assert(after.frame_thread === after.render_thread && after.panics === "1", `after the panic: ${JSON.stringify(after)}`);
        measured(`the page logged: ${page.errors.find(reported).slice(0, 200)}`);
    });
}

// --- memory ------------------------------------------------------------------------------------------

// The memory of a module built with threads is fixed (FERROUI_BROWSER_THREAD_MEMORY_MB of
// scripts/build-browser.sh), so what the catalog needs has to be known: the end of the dynamic memory
// of the module after a tour of pages, sampled ten times a second by the host page
// (`?MemoryReport=true`). Without threads the memory grows, and the number tells how far.
check("the memory of the module after a tour of pages", async (page) => {
    const memory = async () => pairs(await page.evaluate("controlCatalog.catalogMemory()"));
    const megabytes = (bytes) => (Number(bytes) / (1024 * 1024)).toFixed(1);
    const atStart = await memory();
    assert(Number(atStart.top) > 0 && Number(atStart.size) >= Number(atStart.top), `the memory of the module: ${JSON.stringify(atStart)}`);
    const visited = [];
    for (const entry of TOUR) {
        await visit(page, entry);
        // The page is drawn (and, with the prefetch on, its files have arrived or are on their way).
        await sleep(700);
        const now = await memory();
        visited.push(`${entry.name} ${megabytes(now.top)}`);
    }
    // The files of the pages that were not visited arrive too (the prefetch is on).
    await page.waitFor(`performance.getEntriesByType("mark").some((m) => m.name === "assets prefetched")`, 120_000);
    await sleep(1000);
    const atEnd = await memory();
    console.log(`      end of the dynamic memory after each page, MB: ${visited.join(", ")}`);
    measured(`memory in use at the start ${megabytes(atStart.top)} MB, at the end ${megabytes(atEnd.top)} MB, peak ${megabytes(atEnd.peak)} MB, of ${megabytes(atEnd.size)} MB (${threaded ? "fixed" : "grown to"})`);
    assert(Number(atEnd.peak) >= Number(atStart.top) && Number(atEnd.peak) <= Number(atEnd.size), `the memory of the module: ${JSON.stringify(atEnd)}`);
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
}, { query: "&MemoryReport=true", size: TOUR_SIZE });

await run(checks);
