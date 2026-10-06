// Tests of the ControlCatalog site in headless Chrome.
//
//   scripts/build-browser.sh control-catalog-browser
//   node scripts/browser/tests/control_catalog.test.mjs [<site directory>] [--screenshot <file.png>]
//
// The site directory defaults to target/browser/control-catalog-browser. The checks:
//
// - no file of the site carries a brand asset of the upstream project that
//   samples/ControlCatalog/PlaceholderAssets replaces;
// - once for WebGL2 (`?RenderingMode=WebGL2`) and once for the 2D canvas (`?RenderingMode=Software2D`):
//   the application starts (the splash is closed and the view has a canvas), the canvas has the
//   context of the mode, the main view is drawn (the canvas is not blank), the view is laid out again
//   when the page is made larger (the area the larger size adds is drawn) and smaller, and nothing is
//   logged as an error (console errors, uncaught exceptions, failed loads);
// - input, driven with real pointer and key events: the navigation drawer opens from its toggle
//   button, three pages are reached through the drawer and show their content, a click on a button
//   has its effect, and text typed into a text box becomes its text;
// - the native control demo (samples/ControlCatalog.Browser/embed_sample_browser.rs): the Native Embed
//   page shows its two native controls, elements of the page over the view (a button of the page that
//   counts the clicks it gets from real pointer events, and an iframe), and a check box of the page hides
//   one of them.
//
// The view is read through the `catalogState` export of the host (samples/ControlCatalog.Browser),
// which reports the drawer, the current page, the focus and the visible text with its bounds; the
// tests find the controls they click from those bounds.
//
// With --screenshot the picture of the WebGL2 run at the first size is written to the given file.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, sleep } from "../harness.mjs";

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

// Start-up includes compiling the WebAssembly module, which takes a while on a cold CI runner.
const START_TIMEOUT = 180_000;
const FRAME_TIMEOUT = 60_000;
const STATE_TIMEOUT = 20_000;
const FIRST_SIZE = { width: 1024, height: 700 };
const LARGER_SIZE = { width: 1440, height: 900 };
const SMALLER_SIZE = { width: 800, height: 600 };
// Below 640 pixels the drawer of the main view is an overlay that starts closed.
const NARROW_SIZE = { width: 600, height: 700 };

/** Opens the site in a page of `size` and waits until the application has started and drawn. */
async function start({ mode = "WebGL2", size = FIRST_SIZE } = {}) {
    const page = await open(site, { query: `?RenderingMode=${mode}`, width: size.width, height: size.height });
    const started = Date.now();
    try {
        await page.waitFor(`(() => {
            const canvas = document.querySelector("#out canvas");
            const splash = document.querySelector("#out .ferroui-splash");
            return canvas && canvas.width > 0 && (!splash || splash.classList.contains("splash-close"))
                && globalThis.controlCatalog && controlCatalog.catalogState() !== "null";
        })()`, START_TIMEOUT);
    } catch (error) { await page.close(); throw error; }
    page.startSeconds = (Date.now() - started) / 1000;
    page.state = async () => JSON.parse(await page.evaluate("controlCatalog.catalogState()"));
    /** Waits until `predicate(state)` holds and returns that state. */
    page.until = async (description, predicate, timeout = STATE_TIMEOUT) => {
        let state;
        for (const end = Date.now() + timeout; Date.now() < end;) {
            state = await page.state();
            if (predicate(state)) { return state; }
            await sleep(100);
        }
        throw new Error(`timed out waiting until ${description}; the view shows: ${describe(state)}`);
    };
    /** The visible element whose text is `text` (and that matches `filter`), waiting for it to appear. */
    page.find = async (text, filter = () => true) => {
        const state = await page.until(`"${text}" is shown`, (s) => s.elements.some((e) => e.text === text && filter(e)));
        return state.elements.find((e) => e.text === text && filter(e));
    };
    /** Clicks the middle of an element of the state with real pointer events. */
    page.clickElement = async (element) => {
        await page.click(Math.round(element.x + element.width / 2), Math.round(element.y + element.height / 2));
    };
    return page;
}

/** A short description of what a state shows, for failure messages. */
function describe(state) {
    if (!state) { return "nothing"; }
    const texts = state.elements.filter((e) => e.text && e.hit).map((e) => `${e.text}@${Math.round(e.x)},${Math.round(e.y)}`);
    return `page ${JSON.stringify(state.page)}${state.navigating ? " (navigating)" : ""}, drawer ${state.drawerOpen ? "open" : "closed"}, focus ${JSON.stringify(state.focus)}, texts that can be clicked ${JSON.stringify(texts)}`;
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
                canvas = await page.evaluate(`(c => ({ width: c.width, height: c.height, dpr: devicePixelRatio }))(document.querySelector("#out canvas"))`);
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
        // Asking a canvas for a context of another kind than the one it has gives null, so this tells
        // the kind without creating a context.
        const kind = await page.evaluate(`(c => c.getContext("webgl2") ? "webgl2" : c.getContext("webgl") ? "webgl" : c.getContext("2d") ? "2d" : "unknown")(document.querySelector("#out canvas"))`);
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

// The drawer is 260 pixels wide; its elements are those to the left of that edge.
const DRAWER_EDGE = 260;
const inDrawer = (e) => e.hit && e.x + e.width <= DRAWER_EDGE;
const inContent = (e) => e.hit && e.x >= DRAWER_EDGE;

/** Clicks the entry `text` of the drawer and waits until the main view shows the page `header`. */
async function navigate(page, text, header = text) {
    await page.clickElement(await page.find(text, inDrawer));
    // The navigation page ignores a navigation while it runs one (as upstream): wait until it has finished.
    await page.until(`the page "${header}" is shown`, (s) => s.page === header && !s.navigating
        // The title bar of the navigation page shows the header of the page.
        && s.elements.some((e) => e.type === "TextBlock" && e.text === header && e.hit && e.y < 48 && e.x >= DRAWER_EDGE));
}

check("the navigation drawer opens from its toggle button", async (page) => {
    let state = await page.until("the drawer is closed", (s) => s.drawerOpen === false);
    assert(!state.elements.some((e) => e.name === "SearchBox" && e.hit), "the search box of the closed drawer can be reached");
    const toggle = state.elements.find((e) => e.name === "PART_BackButton" && e.hit);
    assert(toggle, `no toggle button of the drawer; the view shows: ${describe(state)}`);
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
    await navigate(page, "Basic Input");
    await navigate(page, "Buttons");
    await page.find("Click the first button to raise Click.", inContent);
    await page.find("Standard _button", inContent);

    await navigate(page, "CheckBox");
    await page.until("the check boxes of the page are shown", (s) => s.elements.filter((e) => e.type === "CheckBox" && inContent(e)).length >= 3);
    assert(!(await page.state()).elements.some((e) => e.text === "Click the first button to raise Click." && e.hit), "the previous page is still shown");

    await navigate(page, "Text");
    await navigate(page, "TextBox");
    await page.find("First Look", inContent);
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
    const box = (await page.until("the text box of the sample is shown",
        (s) => !s.navigating && s.elements.some((e) => e.name === "FirstLookBox" && inContent(e)))).elements.find((e) => e.name === "FirstLookBox");
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

await run(checks);
