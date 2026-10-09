// Drives the pages of the ControlCatalog site through the exports of its host
// (samples/ControlCatalog.Browser): what the view shows (`catalogState`) and where its frames are drawn
// (`catalogRendering`). Used by tests/control_catalog.test.mjs and by capture-catalog.mjs.
import { sleep } from "./harness.mjs";

export const STATE_TIMEOUT = 20_000;

// The drawer is 260 pixels wide; its elements are those to the left of that edge.
export const DRAWER_EDGE = 260;
export const inDrawer = (e) => e.hit && e.x + e.width <= DRAWER_EDGE;
export const inContent = (e) => e.hit && e.x >= DRAWER_EDGE;

/** The `name=value` pairs of a line an export of the host answers with. */
export const pairs = (line) => Object.fromEntries(line.split(";").map((pair) => {
    const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)];
}));

/** A short description of what a state shows, for failure messages. */
export function describe(state) {
    if (!state) { return "nothing"; }
    const texts = state.elements.filter((e) => e.text && e.hit).map((e) => `${e.text}@${Math.round(e.x)},${Math.round(e.y)}`);
    return `page ${JSON.stringify(state.page)}${state.navigating ? " (navigating)" : ""}, drawer ${state.drawerOpen ? "open" : "closed"}, focus ${JSON.stringify(state.focus)}, texts that can be clicked ${JSON.stringify(texts)}`;
}

/** Gives a page of the harness the functions that read and drive the view of the catalog. */
export function drive(page) {
    page.state = async () => JSON.parse(await page.evaluate("controlCatalog.catalogState()"));
    /** Where the frames of the view are rendered (the export exists in both builds). */
    page.rendering = async () => pairs(await page.evaluate("controlCatalog.catalogRendering()"));
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

/**
 * Waits until the application has started and its view can take input: the splash is closed, the view
 * has a canvas and a state, a frame was drawn and something of the view is hit. The splash is closed
 * by the thread of the page; the first frame comes from the thread that renders, which with a render
 * thread is another one and may be later, and a view is hit from what its last frame drew: input
 * that arrives before that frame hits nothing.
 */
export async function waitUntilReady(page, timeout) {
    await page.waitFor(`(() => {
        const canvas = document.querySelector("#out canvas");
        const splash = document.querySelector("#out .ferroui-splash");
        return canvas && canvas.width > 0 && (!splash || splash.classList.contains("splash-close"))
            && globalThis.controlCatalog && controlCatalog.catalogState() !== "null";
    })()`, timeout);
    await page.waitFor(`Number(/frames=(\\d+)/.exec(controlCatalog.catalogRendering())[1]) > 0
        && JSON.parse(controlCatalog.catalogState()).elements.some((e) => e.hit)`, timeout);
}

/** Clicks the entry `text` of the drawer and waits until the main view shows the page `header`. */
export async function navigate(page, text, header = text) {
    await page.clickElement(await page.find(text, inDrawer));
    // The navigation page ignores a navigation while it runs one (as upstream): wait until it has finished.
    await page.until(`the page "${header}" is shown`, (s) => s.page === header && !s.navigating
        // The title bar of the navigation page shows the header of the page.
        && s.elements.some((e) => e.type === "TextBlock" && e.text === header && e.hit && e.y < 48 && e.x >= DRAWER_EDGE));
}

/** Waits until no frame has been drawn for `quiet` ms and returns what the view reports then; null when it keeps drawing. */
export async function settled(page, quiet = 500, timeout = 8000) {
    let last = await page.rendering(); let since = Date.now();
    for (const end = Date.now() + timeout; Date.now() < end;) {
        await sleep(100);
        const now = await page.rendering();
        if (now.frames !== last.frames) { last = now; since = Date.now(); } else if (Date.now() - since >= quiet) { return now; }
    }
    return null;
}

// A tour of the catalog: pages of most sections, among them pages with pictures (Image, Container
// Queries), with text input (TextBox), with what is drawn on the thread that renders by code of the
// page (Composition: a custom visual handler and animations) and the OpenGL page, which in a browser
// shows its knobs and its information text without a GL control. The sections lower in the drawer come
// first: an open section pushes the entries below it down. `animated` pages keep drawing and are not
// compared between two runs.
export const TOUR = [
    { section: "Window & Platform", name: "Platform Information" },
    { section: "Status & Feedback", name: "Data Validation" },
    { section: "Media & Graphics", name: "OpenGL" },
    { section: "Media & Graphics", name: "Image" },
    { section: "Media & Graphics", name: "Composition", animated: true },
    { section: "Layout", name: "Container Queries" },
    { section: "Layout", name: "Border" },
    { section: "Navigation & Pages", name: "TabControl" },
    { section: "Collections & Data", name: "ListBox" },
    { section: "Text", name: "TextBox" },
    { section: "Text", name: "TextBlock" },
    { section: "Basic Input", name: "Slider" },
    { section: "Basic Input", name: "Buttons" }
];
// A view tall enough for the drawer to show every section with the entries of an open one.
export const TOUR_SIZE = { width: 1024, height: 1100 };

/** The entry `text` of the drawer, scrolled into the view when the drawer is longer than the view. */
async function drawerEntry(page, text, height) {
    const shown = (s) => s.elements.find((e) => e.text === text && inDrawer(e));
    for (let attempt = 0; attempt < 16; attempt++) {
        try { return shown(await page.until(`"${text}" is in the drawer`, (s) => !s.navigating && shown(s), 2500)); } catch { }
        // Down first, then back up past where it started.
        await page.wheel(Math.round(DRAWER_EDGE / 2), Math.round(height / 2), 0, attempt < 8 ? 200 : -400);
    }
    return page.find(text, inDrawer);
}

/** Opens the page `entry` of the tour: its section in the drawer, then its entry. */
export async function visit(page, entry, height = TOUR_SIZE.height) {
    let state = await page.state();
    if (!state.elements.some((e) => e.text === entry.name && inDrawer(e))) {
        await page.clickElement(await drawerEntry(page, entry.section, height));
        await page.until(`the section "${entry.section}" is shown`, (s) => s.page === entry.section && !s.navigating);
    }
    await page.clickElement(await drawerEntry(page, entry.name, height));
    state = await page.until(`the page "${entry.name}" is shown`, (s) => s.page === entry.name && !s.navigating);
    // The pointer leaves the drawer, so that no entry is drawn hovered.
    await page.mouseMove(Math.round((DRAWER_EDGE + TOUR_SIZE.width) / 2), height - 6);
    return state;
}
