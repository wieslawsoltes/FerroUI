// Behaviour tests of the browser platform on the themed_view example, in headless Chrome.
//
//   scripts/build-browser.sh themed_view
//   node scripts/browser/tests/themed_view.test.mjs [<site directory>]     default: target/browser/themed_view
//
// The page is driven with real mouse, wheel and key events; the state of the controls is read
// through the `themedViewState` export of the example, the page through the DOM.
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, sleep } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const site = process.argv[2] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser", "themed_view");

// Where the controls are in a 460 x 520 view (see docs/porting/images/themed_view_browser.png).
const BUTTON = [46, 66]; const CHECK_BOX = [26, 107]; const TEXT_BOX = [230, 149]; const LIST = [200, 300]; const EMPTY = [300, 450];

async function start(query = "") {
    const page = await open(site, { query, width: 460, height: 520 });
    await page.waitForView();
    page.state = async () => Object.fromEntries((await page.evaluate("themedView.themedViewState()")).split(";").map((pair) => {
        const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)];
    }));
    // Records, after the handlers of the view ran, whether each key kept its default action.
    await page.evaluate(`(globalThis.keys = [], document.addEventListener("keydown", (e) => { keys.push({ key: e.key, prevented: e.defaultPrevented }); if (globalThis.keepPage) { e.preventDefault(); } }), true)`);
    page.lastKey = async () => await page.evaluate("JSON.stringify(keys[keys.length - 1])").then(JSON.parse);
    return page;
}

const checks = [];
const check = (name, body, query = "") => checks.push([name, async () => {
    const page = await start(query);
    try { await body(page); } catch (error) { error.message += "\n" + page.log.slice(-5).join("\n"); throw error; } finally { await page.close(); }
}]);

check("the pointer hovers and clicks a button", async (page) => {
    await page.mouseMove(...BUTTON); await sleep(200);
    assert((await page.state()).button_over === "true", "the button is not under the pointer");
    await page.click(...BUTTON);
    await page.click(...BUTTON);
    const state = await page.state();
    assert(state.clicks === "2", `expected 2 clicks, state: ${JSON.stringify(state)}`);
    await page.mouseMove(...EMPTY); await sleep(200);
    assert((await page.state()).button_over === "false", "the pointer left the button but it is still hovered");
});

check("a click toggles the check box", async (page) => {
    assert((await page.state()).checked === "Some(true)", "the check box starts checked");
    await page.click(...CHECK_BOX);
    assert((await page.state()).checked === "Some(false)", "the check box did not toggle");
});

check("the cursor of the page follows the control under the pointer", async (page) => {
    const cursor = () => page.evaluate(`document.getElementById("out").style.cursor`);
    await page.mouseMove(...TEXT_BOX); await sleep(300);
    assert(await cursor() === "text", `over the text box the cursor is "${await cursor()}"`);
    await page.mouseMove(...EMPTY); await sleep(300);
    assert(await cursor() === "", `away from the text box the cursor is "${await cursor()}"`);
});

check("typing in a text box edits its text and does not act on the page", async (page) => {
    await page.click(...TEXT_BOX);
    assert((await page.state()).focus === "text_box", "the text box did not take the focus");
    await page.press("End");
    await page.type("abc");
    let state = await page.state();
    assert(state.text === "Text boxabc", `text is "${state.text}"`);
    assert((await page.lastKey()).prevented, "a typed character kept its default action");
    await page.press("Backspace");
    state = await page.state();
    assert(state.text === "Text boxab", `after Backspace the text is "${state.text}"`);
    assert((await page.lastKey()).prevented, "Backspace kept its default action (history navigation)");
    await page.press("Home"); await page.type("X");
    assert((await page.state()).text === "XText boxab", `after Home the text is "${(await page.state()).text}"`);
    const hidden = await page.evaluate(`document.querySelector(".ferroui-input-element").value`);
    assert(!hidden.includes("X") || hidden === "XText boxab", `the hidden input element collected typed text: "${hidden}"`);
});

check("shortcuts of the browser are not swallowed", async (page) => {
    await page.click(...TEXT_BOX);
    // The recorder has noted whether the view prevented the key; it then cancels the shortcut itself,
    // so that a reload or a new tab does not replace the page under test.
    await page.evaluate("(globalThis.keepPage = true)");
    for (const [key, modifier] of [["r", "ctrl"], ["r", "meta"], ["l", "ctrl"], ["l", "meta"], ["t", "ctrl"], ["F5", null], ["F12", null]]) {
        await page.press(key, modifier ? [modifier] : []);
        const last = await page.lastKey();
        assert(last.key.toLowerCase() === key.toLowerCase(), `the page did not see ${modifier ?? ""}+${key}`);
        assert(!last.prevented, `${modifier ? modifier + "+" : ""}${key} was prevented`);
    }
    assert((await page.state()).text === "Text box", `a shortcut typed into the text box: "${(await page.state()).text}"`);
    await page.evaluate("(globalThis.keepPage = false, true)");
    // A dead key (an accent waiting for its letter) belongs to the input method.
    await page.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Dead", code: "BracketLeft", windowsVirtualKeyCode: 219 });
    await page.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Dead", code: "BracketLeft", windowsVirtualKeyCode: 219 });
    assert((await page.lastKey()).key === "Dead" && !(await page.lastKey()).prevented, "a dead key was prevented");
});

check("arrows, Space and Page Down do not scroll the page while the view has the focus", async (page) => {
    // Make the page scrollable below the view.
    await page.evaluate(`(document.body.style.overflow = "auto", document.body.insertAdjacentHTML("beforeend", '<div style="height:3000px"></div>'), true)`);
    // A control of the view takes the focus (and with it the element of the view the focus of the page).
    await page.click(...CHECK_BOX);
    for (const key of ["ArrowDown", "ArrowUp", "Space", "PageDown", "End", "Home"]) {
        await page.press(key);
        assert((await page.lastKey()).prevented, `${key} kept its default action`);
        assert(await page.evaluate("scrollY") === 0, `${key} scrolled the page`);
    }
    // The same keys scroll the page again once the focus is outside the view.
    await page.evaluate(`(document.body.insertAdjacentHTML("afterbegin", '<button id="outside" style="position:fixed;left:400px;top:480px">x</button>'), document.getElementById("outside").focus(), true)`);
    await page.press("PageDown");
    assert(!(await page.lastKey()).prevented, "Page Down was prevented outside the view");
});

check("Tab moves the focus inside the view, and leaves it only when the view does not handle it", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("beforeend", '<button id="after">after</button>'), true)`);
    await page.click(...TEXT_BOX);
    const seen = [];
    for (let i = 0; i < 8; i++) {
        await page.press("Tab");
        const last = await page.lastKey();
        const inside = await page.evaluate(`document.getElementById("out").contains(document.activeElement)`);
        const focus = (await page.state()).focus;
        seen.push(`${focus}${last.prevented ? "" : "(passed on)"}`);
        // A Tab the view handled stays in the view; one it did not handle is the browser's.
        assert(last.prevented === inside, `Tab ${i + 1}: prevented=${last.prevented} but the focus of the page is ${inside ? "inside" : "outside"} the view (${seen.join(" > ")})`);
        if (!inside) { break; }
    }
    assert(new Set(seen).size > 1, `Tab did not move the focus: ${seen.join(" > ")}`);
    console.log(`      focus order: ${seen.join(" > ")}`);
});

check("arrow keys move a focused slider", async (page) => {
    await page.click(...TEXT_BOX);
    for (let i = 0; i < 6 && (await page.state()).focus !== "slider"; i++) { await page.press("Tab"); }
    assert((await page.state()).focus === "slider", `could not reach the slider with Tab: ${(await page.state()).focus}`);
    const before = Number((await page.state()).slider);
    await page.press("ArrowRight");
    const after = Number((await page.state()).slider);
    assert(after > before, `the slider did not move: ${before} -> ${after}`);
});

check("the wheel scrolls the list and not the page", async (page) => {
    await page.evaluate(`(document.body.style.overflow = "auto", document.body.insertAdjacentHTML("beforeend", '<div style="height:3000px"></div>'), true)`);
    const before = await page.screenshot();
    await page.wheel(...LIST, 0, 100);
    await sleep(600);
    const after = await page.screenshot();
    let changed = 0;
    for (let y = 265; y < 355; y += 3) { for (let x = 20; x < 430; x += 5) { if (before.pixel(x, y).join() !== after.pixel(x, y).join()) { changed++; } } }
    assert(changed > 50, `the list did not scroll (${changed} sampled pixels changed)`);
    assert(await page.evaluate("scrollY") === 0, "the wheel scrolled the page");
});

check("a click on a list item selects it", async (page) => {
    assert((await page.state()).selected === "1", "the second item starts selected");
    await page.click(60, 279);
    assert((await page.state()).selected === "0", `selected is ${(await page.state()).selected}`);
});

check("the focus leaving the view is reported", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("afterbegin", '<button id="outside" style="position:fixed;left:380px;top:470px;z-index:10">x</button>'), true)`);
    await page.click(...TEXT_BOX);
    assert((await page.state()).focus === "text_box", "the text box did not take the focus");
    await page.type("a");
    await page.click(400, 482);
    assert(await page.evaluate(`document.activeElement.id`) === "outside", "the outside button did not take the focus of the page");
    await page.type("zzz");
    assert((await page.state()).text === "Text boxa", `keys typed outside the view reached the text box: "${(await page.state()).text}"`);
});

check("a text box that loses the focus to the page does not take it back", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("afterbegin", '<button id="outside" style="position:fixed;left:380px;top:470px;z-index:10">x</button>'), true)`);
    await page.click(...TEXT_BOX);
    assert((await page.state()).focus === "text_box", "the text box did not take the focus");
    assert(await page.evaluate(`document.activeElement.classList.contains("ferroui-input-element")`), "the hidden input element does not have the focus of the page while a text box is edited");
    await page.click(400, 482);
    await sleep(400);
    assert(await page.evaluate(`document.activeElement.id`) === "outside", `the focus of the page is on "${await page.evaluate("document.activeElement.id || document.activeElement.className")}" after a click outside the view`);
    assert((await page.state()).focus === "other", `the view still has a focused control: ${(await page.state()).focus}`);
    assert(await page.evaluate(`document.querySelector(".ferroui-input-element").style.display`) === "none", "the hidden input element is still shown");
});

check("Tab out of a text box leaves the view without stopping on the hidden input element", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("beforeend", '<button id="after">after</button>'), true)`);
    assert(await page.evaluate(`document.querySelector(".ferroui-input-element").tabIndex`) === -1, "the hidden input element is a tab stop");
    await page.click(...TEXT_BOX);
    let left = false;
    for (let i = 0; i < 12 && !left; i++) {
        await page.press("Tab");
        const active = await page.evaluate(`document.activeElement.id || document.activeElement.className`);
        const inside = await page.evaluate(`document.getElementById("out").contains(document.activeElement)`);
        // Inside the view the focus of the page is on the view or, while a text box is edited, on its input element.
        if (!inside) { left = true; assert(active === "after", `Tab left the view to "${active}"`); }
    }
    if (left) {
        await sleep(300);
        assert(await page.evaluate(`document.activeElement.id`) === "after", "the view took the focus back after Tab left it");
        assert((await page.state()).focus === "other", `the view still has a focused control: ${(await page.state()).focus}`);
    } else {
        console.log("      the view handles every Tab (its focus cycles): Tab never reaches the browser here");
    }
});

check("a Tab the application does not handle leaves the view in one step, from a text box too", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("beforeend", '<button id="after">after</button>'), true)`);
    // The controls of this example cycle the focus, so the application is made to decline Tab.
    await page.evaluate(`(async () => {
        const module = await import("./ferroui.js");
        const original = themedView.InputHelper_OnKeyDown;
        themedView.InputHelper_OnKeyDown = (id, code, key, modifiers) => key === "Tab" ? false : original(id, code, key, modifiers);
        module.FerroExports.attach(themedView);
        return true;
    })()`);
    await page.click(...TEXT_BOX);
    assert(await page.evaluate(`document.activeElement.classList.contains("ferroui-input-element")`), "the hidden input element does not have the focus");
    await page.press("Tab");
    assert(!(await page.lastKey()).prevented, "an unhandled Tab was prevented");
    await sleep(300);
    assert(await page.evaluate(`document.activeElement.id`) === "after", `one Tab moved the focus of the page to "${await page.evaluate("document.activeElement.id || document.activeElement.className")}"`);
    assert((await page.state()).focus === "other", `the view still has a focused control: ${(await page.state()).focus}`);
    assert(await page.evaluate(`document.querySelector(".ferroui-input-element").style.display`) === "none", "the hidden input element is still shown");
    // And back with Shift+Tab: the element of the view is the tab stop, not the hidden input element.
    await page.press("Tab", ["shift"]);
    assert(await page.evaluate(`document.activeElement.id`) === "out", `Shift+Tab went to "${await page.evaluate("document.activeElement.id || document.activeElement.className")}"`);
});

check("the view keeps its focused control when the window is deactivated, and drops it when another element takes the focus", async (page) => {
    await page.evaluate(`(document.body.insertAdjacentHTML("beforeend", '<button id="second">second</button>'), true)`);
    await page.click(...BUTTON);
    assert((await page.state()).focus === "button", `the button did not take the focus: ${(await page.state()).focus}`);
    // A window or tab that is deactivated: the focus goes nowhere in the document, and the document is not focused.
    await page.evaluate(`(() => { const real = document.hasFocus; document.hasFocus = () => false;
        document.getElementById("out").dispatchEvent(new FocusEvent("focusout", { bubbles: true, relatedTarget: null }));
        document.hasFocus = real; return true; })()`);
    assert((await page.state()).focus === "button", `deactivating the window cleared the focus of the view: ${(await page.state()).focus}`);
    // Another element of the same document takes the focus.
    await page.evaluate(`(document.getElementById("second").focus(), true)`);
    await sleep(200);
    assert((await page.state()).focus === "other", `the view kept its focused control although another element has the focus: ${(await page.state()).focus}`);
});

check("the barrel button of a pen is reported as such, the right button of a mouse as a mouse button", async (page) => {
    const modifiers = await page.evaluate(`(async () => {
        const module = await import("./ferroui.js");
        const seen = [];
        const original = themedView.InputHelper_OnPointerDown;
        themedView.InputHelper_OnPointerDown = (...args) => { seen.push(args[args.length - 1]); return original(...args); };
        module.FerroExports.attach(themedView);
        const host = document.getElementById("out");
        for (const pointerType of ["pen", "mouse"]) {
            host.dispatchEvent(new PointerEvent("pointerdown", { pointerType, pointerId: 7, button: 2, buttons: 2, clientX: 300, clientY: 450, bubbles: true }));
            host.dispatchEvent(new PointerEvent("pointerup", { pointerType, pointerId: 7, button: 2, buttons: 0, clientX: 300, clientY: 450, bubbles: true }));
        }
        return seen;
    })()`);
    const PEN_BARREL_BUTTON = 2048; const RIGHT_MOUSE_BUTTON = 32;
    assert(modifiers.length === 2, `expected two pointer-down reports, got ${JSON.stringify(modifiers)}`);
    assert((modifiers[0] & PEN_BARREL_BUTTON) !== 0 && (modifiers[0] & RIGHT_MOUSE_BUTTON) === 0, `pen: modifiers ${modifiers[0]}`);
    assert((modifiers[1] & RIGHT_MOUSE_BUTTON) !== 0 && (modifiers[1] & PEN_BARREL_BUTTON) === 0, `mouse: modifiers ${modifiers[1]}`);
});

check("input works with the software render target too", async (page) => {
    assert(await page.evaluate(`!!document.querySelector("canvas").getContext("2d")`), "the canvas is not a 2D canvas");
    await page.click(...BUTTON);
    await page.click(...TEXT_BOX); await page.press("End"); await page.type("q");
    const state = await page.state();
    assert(state.clicks === "1" && state.text === "Text boxq", JSON.stringify(state));
}, "?RenderingMode=Software2D");

await run(checks);
