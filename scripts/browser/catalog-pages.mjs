// Drives the pages of the ControlCatalog site through the exports of its host
// (samples/ControlCatalog.Browser): what the view shows (`catalogState`), whether it has drawn that
// (`catalogRequestFrame`, `catalogFrameDrawn`) and where its frames are drawn (`catalogRendering`).
// Used by tests/control_catalog.test.mjs and by capture-catalog.mjs.
//
// The state is what the application has laid out; input hits what the last frame drew. The two differ
// from a change of the application until the frame with that change, and the thread that renders draws
// when the browser gives it an animation frame. A render thread waits for the animation frames of its
// worker, which headless Chrome with the software rasteriser withholds for hundreds of milliseconds
// after a frame that was expensive to present (the first frames of a page, a transition), while the
// thread of the page, which no longer renders, answers at once. So nothing here clicks where the state
// says an element is before the view has drawn that state: `page.find` and `page.element` wait for a
// frame asked for after the state was read and return the element only if it is still where it was.
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

/**
 * Waits until the view has drawn everything the application has changed up to this call: the host is
 * asked for a frame (a commit of the compositor of the view) and answers once the thread that renders
 * has drawn it.
 */
export async function drawn(page, timeout = STATE_TIMEOUT) {
    if (!await page.evaluate("controlCatalog.catalogRequestFrame()")) { throw new Error("the view has no compositor to ask for a frame"); }
    for (const end = Date.now() + timeout; Date.now() < end; await sleep(20)) {
        if (await page.evaluate("controlCatalog.catalogFrameDrawn()")) { return; }
    }
    throw new Error(`the view did not draw the frame it was asked for within ${timeout} ms: ${await page.evaluate("controlCatalog.catalogRendering()")}`);
}

const sameBounds = (a, b) => a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/** Gives a page of the harness the functions that read and drive the view of the catalog. */
export function drive(page) {
    page.state = async () => JSON.parse(await page.evaluate("controlCatalog.catalogState()"));
    /** Waits until the view has drawn everything the application has changed so far (`drawn`). */
    page.drawn = (timeout) => drawn(page, timeout);
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
    /**
     * The first visible element that matches, once the view has drawn it where the state says it is:
     * waits for the element, then for a frame asked for after that, and returns the element if the
     * state still has it at the same bounds (it starts over when it moved or left meanwhile).
     */
    page.element = async (description, match, timeout = STATE_TIMEOUT) => {
        let state;
        for (const end = Date.now() + timeout; Date.now() < end;) {
            state = await page.until(description, (s) => s.elements.some(match), Math.max(1, end - Date.now()));
            const before = state.elements.find(match);
            await page.drawn();
            state = await page.state();
            const after = state.elements.find(match);
            if (after && sameBounds(before, after)) { return after; }
        }
        throw new Error(`timed out waiting until ${description} and stays where it is; the view shows: ${describe(state)}`);
    };
    /** The visible element whose text is `text` (and that matches `filter`), once the view has drawn it (`page.element`). */
    page.find = (text, filter = () => true) => page.element(`"${text}" is shown`, (e) => e.text === text && filter(e));
    /** Clicks the middle of an element of the state with real pointer events. */
    page.clickElement = async (element) => {
        await page.click(Math.round(element.x + element.width / 2), Math.round(element.y + element.height / 2));
    };
    return page;
}

/**
 * Waits until the application has started and its view can take input: the splash is closed, the view
 * has a canvas and a state, a frame was drawn, something of the view is hit and the view has drawn
 * what it has laid out by then. The splash is closed when the first frame of the view has been drawn,
 * whichever thread drew it (a module built before that closed it at the first animation frame of the
 * thread of the page, which with a render thread is earlier: the count of frames is still awaited),
 * and a view is hit from what its last frame drew: input that arrives before that frame hits nothing.
 * The first frame is also not the last one of the start: the main view adapts its drawer to the width
 * of the view when it is loaded, after the first frame, and the frame with that layout follows.
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
    await drawn(page, timeout);
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
    for (let attempt = 0; attempt < 16; attempt++) {
        try {
            await page.until("no navigation is running", (s) => !s.navigating, 2500);
            return await page.element(`"${text}" is in the drawer`, (e) => e.text === text && inDrawer(e), 2500);
        } catch { }
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
