// Behaviour tests of the browser platform on the themed_view example, in headless Chrome.
//
//   scripts/build-browser.sh themed_view
//   node scripts/browser/tests/themed_view.test.mjs [<site directory>]     default: target/browser/themed_view
//
// The page is driven with real mouse, wheel, key and drag events; the state of the controls is read
// through the `themedViewState` export of the example, what the services of the platform answered
// through `themedViewServices`, and the page through the DOM. The native control host of the view is
// changed through `themedViewNativeHost`, and its native control read in the DOM.
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
    page.services = async () => Object.fromEntries((await page.evaluate("themedView.themedViewServices()")).split(";").map((pair) => {
        const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)];
    }));
    // Waits until a value the services report is the expected one.
    page.waitForService = async (name, expected, timeout = 10000) => {
        const end = Date.now() + timeout;
        let services;
        while (Date.now() < end) {
            services = await page.services();
            if (services[name] === expected) { return services; }
            await sleep(100);
        }
        throw new Error(`${name} is ${services?.[name]}, expected ${expected}; services: ${JSON.stringify(services)}`);
    };
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

// Rust panics unwind with the exceptions of the module (WebAssembly exception handling since Rust
// 1.93): the `catch_unwind` of the dispatcher must catch them in the page as it does on the desktop.
check("panics on the dispatcher are caught and passed on as on the desktop, and the view keeps working", async (page) => {
    const panic = async (kind) => Object.fromEntries((await page.evaluate(`themedView.themedViewPanic("${kind}")`)).split(";").map((pair) => {
        const i = pair.indexOf("="); return [pair.slice(0, i), pair.slice(i + 1)];
    }));
    const uncaught = () => page.errors.filter((line) => line.startsWith("[exception]"));

    // `invoke_local`: the panic of the callback reaches the caller.
    assert((await panic("invoke")).invoke === "an invoked callback panics", "the panic of an invoked callback did not reach its caller");

    // A posted job: the dispatcher's handler sees the panic and handles it; the next job runs.
    await panic("post");
    await page.waitFor(`themedView.themedViewPanic("").includes("jobs_after=1")`, 10000);
    let state = await panic("");
    assert(state.handled === "a posted job panics (handled)", `the unhandled-exception handler saw: ${state.handled}`);
    assert(uncaught().length === 0, `a handled panic reached the page:\n${uncaught().join("\n")}`);

    // Left unhandled, the panic leaves the job loop for the page, as wasm-bindgen's `PanicError`.
    await panic("unhandled");
    const end = Date.now() + 10000;
    while (uncaught().length === 0 && Date.now() < end) { await sleep(100); }
    state = await panic("");
    assert(state.handled === "a posted job panics (handled)|a posted job panics (unhandled)", `the unhandled-exception handler saw: ${state.handled}`);
    assert(uncaught().length === 1 && uncaught()[0].includes("PanicError: a posted job panics (unhandled)"),
        `expected one uncaught PanicError on the page, got:\n${uncaught().join("\n")}`);
    // The jobs queued behind it wait for the next signal of the dispatcher (as before Rust 1.93).
    await sleep(1000);
    console.log(`      jobs run after the unhandled panic: ${(await panic("")).jobs_after - 1}`);

    // The view still takes input.
    await page.click(...BUTTON);
    assert((await page.state()).clicks === "1", "the button did not take a click after the panics");
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
    // Chrome reserves ctrl+t (with ctrl+n and ctrl+w): some builds never deliver it to the page, so the view
    // cannot swallow it there. When it is delivered, it must keep its default action like the others.
    const reserved = ["ctrl+t"];
    for (const [key, modifier] of [["r", "ctrl"], ["r", "meta"], ["l", "ctrl"], ["l", "meta"], ["t", "ctrl"], ["F5", null], ["F12", null]]) {
        const name = `${modifier ? modifier + "+" : ""}${key}`;
        const before = await page.evaluate("keys.length");
        await page.press(key, modifier ? [modifier] : []);
        const last = await page.lastKey();
        const seen = await page.evaluate("keys.length") > before && last.key.toLowerCase() === key.toLowerCase();
        assert(seen || reserved.includes(name), `the page did not see ${name}`);
        assert(!seen || !last.prevented, `${name} was prevented`);
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

// --- services (stage B4) -------------------------------------------------------------------------

// The asynchronous Clipboard API needs the permissions and a focused document.
async function withClipboard(page) {
    await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
    await page.setPermission("clipboard-read", "granted");
    await page.setPermission("clipboard-write", "granted");
}

check("text is copied from a text box to the clipboard and pasted into it", async (page) => {
    await withClipboard(page);
    await page.click(...TEXT_BOX);
    await page.press("a", ["ctrl"]);
    await page.press("c", ["ctrl"]);
    assert((await page.lastKey()).prevented, "ctrl+c kept the copy of the browser");
    await page.waitFor(`navigator.clipboard.readText().then((text) => text === "Text box")`, 10000);

    await page.evaluate(`navigator.clipboard.writeText(" pasted").then(() => true)`);
    // A click after the end of the text ends the selection there (away from the first click, which
    // would make it a double click that selects a word).
    await page.click(TEXT_BOX[0] + 160, TEXT_BOX[1]);
    assert((await page.state()).caret === "8", `the click did not put the caret at the end: ${JSON.stringify(await page.state())}`);
    await page.press("v", ["ctrl"]);
    await page.waitFor(`themedView.themedViewState().includes("text=Text box pasted;")`, 10000);

    // Cut: the selection leaves the text box for the clipboard.
    await page.press("a", ["ctrl"]);
    await page.press("x", ["ctrl"]);
    await page.waitFor(`themedView.themedViewState().includes("text=;")`, 10000);
    await page.waitFor(`navigator.clipboard.readText().then((text) => text === "Text box pasted")`, 10000);
});

check("the clipboard of the top-level reads and writes the clipboard of the page", async (page) => {
    await withClipboard(page);
    await page.click(...TEXT_BOX);
    await page.evaluate(`navigator.clipboard.writeText("from the page").then(() => true)`);
    await page.evaluate("themedView.themedViewReadClipboard()");
    await page.waitForService("clipboard", `Some("ok:from the page")`);

    await page.evaluate(`themedView.themedViewWriteClipboard("from the view")`);
    await page.waitForService("clipboard", `Some("written")`);
    assert(await page.evaluate("navigator.clipboard.readText()") === "from the view", "the page does not see the text of the view");

    // Several formats of one item: the HTML of the page is a format of the item next to the text.
    await page.evaluate(`navigator.clipboard.write([new ClipboardItem({
        "text/plain": new Blob(["plain"], { type: "text/plain" }),
        "text/html": new Blob(["<b>html</b>"], { type: "text/html" }) })]).then(() => true)`);
    await page.evaluate("themedView.themedViewReadClipboard()");
    await page.waitForService("clipboard", `Some("ok:plain")`);
});

check("a read the page does not allow is reported as access denied", async (page) => {
    await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
    await page.setPermission("clipboard-read", "denied");
    await page.click(...TEXT_BOX);
    await page.evaluate("themedView.themedViewReadClipboard()");
    await page.waitForService("clipboard", `Some("error:AccessDenied")`);
});

check("without the asynchronous clipboard API a paste goes through the paste event", async (page) => {
    await withClipboard(page);
    await page.evaluate(`navigator.clipboard.writeText("fallback").then(() => true)`);
    // A browser without read and readText (Firefox before 127).
    await page.evaluate(`(Object.defineProperty(navigator.clipboard, "read", { value: undefined }),
        Object.defineProperty(navigator.clipboard, "readText", { value: undefined }), true)`);
    await page.click(...TEXT_BOX);
    await page.press("End");
    await page.press("v", ["ctrl"], { commands: ["paste"] });
    const key = await page.lastKey();
    assert(!key.prevented, "the key of the paste was prevented, so the browser could not paste");
    await page.waitFor(`themedView.themedViewState().includes("text=Text boxfallback;")`, 10000);

    // Without a paste event the read ends when the key is released.
    await page.evaluate("themedView.themedViewReadClipboard()");
    await page.press("Shift");
    await page.waitForService("clipboard", `Some("error:Other")`);
});

check("text dropped on a drop target is delivered with its effect", async (page) => {
    const [x, y] = (await page.services()).drop_target.split(",").map(Number);
    const data = { items: [{ mimeType: "text/plain", data: "dropped text" }], dragOperationsMask: 1 };
    await page.send("Input.dispatchDragEvent", { type: "dragEnter", x, y, data });
    await page.send("Input.dispatchDragEvent", { type: "dragOver", x, y, data });
    await page.send("Input.dispatchDragEvent", { type: "drop", x, y, data });
    const services = await page.waitForService("dropped", `Some("dropped text")`);
    assert(Number(services.drag_overs) >= 1, `the drop target saw no drag over: ${JSON.stringify(services)}`);

    // Outside the drop target nothing accepts the data.
    const [ex, ey] = BUTTON;
    await page.send("Input.dispatchDragEvent", { type: "dragEnter", x: ex, y: ey, data: { ...data, items: [{ mimeType: "text/plain", data: "elsewhere" }] } });
    await page.send("Input.dispatchDragEvent", { type: "drop", x: ex, y: ey, data: { ...data, items: [{ mimeType: "text/plain", data: "elsewhere" }] } });
    await sleep(300);
    assert((await page.services()).dropped === `Some("dropped text")`, "a drop outside the target was delivered to it");
});

check("the screen of the view and the safe area are reported", async (page) => {
    const screen = await page.evaluate("JSON.stringify([screen.width, screen.height, screen.availLeft, screen.availTop, screen.availWidth, screen.availHeight, devicePixelRatio])").then(JSON.parse);
    let services = await page.services();
    assert(services.screens === "1", `screens: ${services.screens}`);
    assert(services.bounds === `${screen[2]},${screen[3]},${screen[0]},${screen[1]}`, `bounds ${services.bounds}, screen ${screen}`);
    assert(services.working_area === `${screen[2]},${screen[3]},${screen[4]},${screen[5]}`, `working area ${services.working_area}, screen ${screen}`);
    assert(services.scaling === String(screen[6]), `scaling ${services.scaling}`);
    assert(services.primary === "true", `primary ${services.primary}`);
    assert(services.orientation === "Landscape" || services.orientation === "Portrait", `orientation ${services.orientation}`);
    assert(services.safe_area === "0,0,0,0", `safe area ${services.safe_area}`);
    assert(services.system_bar_visible === "Some(false)", `full screen ${services.system_bar_visible}`);

    // A device with insets (the variables are what the page sets from env(safe-area-inset-*)); a resize reports the change.
    const changes = Number(services.safe_area_changes);
    await page.evaluate(`(["--ferro-sal:1px", "--ferro-sat:2px", "--ferro-sar:3px", "--ferro-sab:4px"].forEach((pair) => {
        const [name, value] = pair.split(":"); document.documentElement.style.setProperty(name, value); }), true)`);
    await page.send("Emulation.setDeviceMetricsOverride", { width: 440, height: 500, deviceScaleFactor: 1, mobile: false });
    await page.waitFor(`themedView.themedViewServices().includes("safe_area=1,2,3,4")`, 10000);
    await page.waitFor(`Number(themedView.themedViewServices().match(/safe_area_changes=(\\d+)/)[1]) > ${changes}`, 10000);
});

check("the details of all screens are requested through the permission of the page", async (page) => {
    await page.setPermission("window-management", "granted");
    await page.evaluate("themedView.themedViewRequestScreenDetails()");
    const services = await page.waitForService("screen_details", "Some(true)");
    assert(Number(services.screens) >= 1, `screens: ${services.screens}`);
    assert(services.primary === "true", `primary ${services.primary}`);
});

check("the launcher opens absolute URIs in a new browsing context", async (page) => {
    await page.evaluate(`(globalThis.opened = [], window.open = (uri, target) => (opened.push([uri, target]), {}), true)`);
    await page.evaluate(`themedView.themedViewLaunch("https://example.com/a?b=c")`);
    await page.waitForService("launched", "Some(true)");
    assert(await page.evaluate("JSON.stringify(opened)") === JSON.stringify([["https://example.com/a?b=c", "_blank"]]), await page.evaluate("JSON.stringify(opened)"));

    // A relative URI is not opened; a blocked pop-up is reported.
    await page.evaluate(`themedView.themedViewLaunch("docs/index.html")`);
    await page.waitForService("launched", "Some(false)");
    await page.evaluate(`(window.open = () => null, opened.length = 0, true)`);
    await page.evaluate(`themedView.themedViewLaunch("https://example.com/b")`);
    await page.waitForService("launched", "Some(false)");
});

check("the back navigation of the browser is a back request of the view", async (page) => {
    await page.evaluate("themedView.themedViewInstallBackHandler()");
    const length = await page.evaluate("history.length");
    await page.evaluate("(history.back(), true)");
    await page.waitForService("back_requests", "1");
    // The handler returns to the entry it pushed; that navigation is not another back request.
    await sleep(1000);
    assert((await page.services()).back_requests === "1", `one back navigation was reported ${(await page.services()).back_requests} times`);
    assert(await page.evaluate("location.pathname") === "/index.html", "the page went back");
    assert(await page.evaluate("history.length") === length, "the history grew");

    // The next back navigation is a request again.
    await page.evaluate("(history.back(), true)");
    await page.waitForService("back_requests", "2");
    await sleep(1000);
    assert((await page.services()).back_requests === "2", `the second back navigation was reported as ${(await page.services()).back_requests}`);
});

// The native control of the native control host: the elements in the element of the view that lies over
// its canvas, with the inline style the host gives them.
const NATIVE_CONTROLS = `JSON.stringify(Array.from(document.querySelector("#out .ferroui-native-host").children).map((e) => ({
    tag: e.tagName, mark: e.dataset.mark ?? null, position: e.style.position, display: e.style.display,
    left: e.style.left, top: e.style.top, width: e.style.width, height: e.style.height })))`;

check("a native control is attached with its host, follows its bounds and visibility, and is detached with it", async (page) => {
    const controls = async () => JSON.parse(await page.evaluate(NATIVE_CONTROLS));
    const waitForControls = async (description, predicate) => {
        const end = Date.now() + 10000;
        let current;
        while (Date.now() < end) {
            current = await controls();
            if (predicate(current)) { return current; }
            await sleep(100);
        }
        throw new Error(`timed out waiting until ${description}; native controls: ${JSON.stringify(current)}, services: ${JSON.stringify(await page.services())}`);
    };
    const host = async () => (await page.services()).native_host?.split(",").map(Number);
    assert((await controls()).length === 0, "the view starts with a native control");
    assert((await page.services()).native_handle === "false", "the host has a native control before it is in the view");

    // Attached: the default native control of the platform, an element of the page at the bounds of the host.
    await page.evaluate(`(themedView.themedViewNativeHost("size", 120, 40), themedView.themedViewNativeHost("add", 0, 0), true)`);
    let [control] = await waitForControls("the native control is shown", (c) => c.length === 1 && c[0].display === "block");
    await page.waitForService("native_handle", "true");
    let [x, y, width, height] = await host();
    assert(width === 120 && height === 40, `the host is ${width} x ${height}`);
    assert(control.tag === "DIV" && control.position === "absolute", `native control ${JSON.stringify(control)}`);
    assert(control.left === `${x}px` && control.top === `${y}px` && control.width === "120px" && control.height === "40px",
        `the native control is at ${JSON.stringify(control)}, the host at ${x},${y}`);
    // It lies over the canvas: the page hit-tests it at its centre.
    assert(await page.evaluate(`document.elementFromPoint(${x + 60}, ${y + 20}) === document.querySelector("#out .ferroui-native-host").firstElementChild`),
        "the native control is not the element at its centre");

    // Resized and moved with the host.
    await page.evaluate(`(themedView.themedViewNativeHost("size", 200, 60), true)`);
    [control] = await waitForControls("the native control has the new size", (c) => c[0]?.width === "200px" && c[0]?.height === "60px");
    [x, y] = await host();
    assert(control.left === `${x}px` && control.top === `${y}px`, `the resized native control is at ${JSON.stringify(control)}, the host at ${x},${y}`);
    // The host is centred in the panel: a left margin moves it by half the margin.
    await page.evaluate(`(themedView.themedViewNativeHost("margin", 30, 0), true)`);
    [control] = await waitForControls("the native control moved with the host", (c) => c[0]?.left === `${x + 15}px`);
    [x, y, width, height] = await host();
    assert(control.left === `${x}px` && control.top === `${y}px` && width === 200 && height === 60,
        `the native control is at ${JSON.stringify(control)}, the host at ${x},${y} (${width} x ${height})`);

    // Hidden with the host, keeping its size, and shown again.
    await page.evaluate(`(themedView.themedViewNativeHost("hide", 0, 0), true)`);
    [control] = await waitForControls("the native control is hidden", (c) => c[0]?.display === "none");
    assert(control.width === "200px" && control.height === "60px", `the hidden native control is ${control.width} x ${control.height}`);
    await page.evaluate(`(themedView.themedViewNativeHost("show", 0, 0), true)`);
    await waitForControls("the native control is shown again", (c) => c[0]?.display === "block" && c[0]?.left === `${x}px`);

    // Detached from the page when the host leaves the view, then destroyed.
    await page.evaluate(`(themedView.themedViewNativeHost("remove", 0, 0), true)`);
    await waitForControls("the native control is detached", (c) => c.length === 0);
    await page.waitForService("native_handle", "false");

    // A host that comes back creates a new native control.
    await page.evaluate(`(themedView.themedViewNativeHost("add", 0, 0), true)`);
    await waitForControls("a new native control is shown", (c) => c.length === 1 && c[0].display === "block" && c[0].width === "200px");
    await page.waitForService("native_handle", "true");
    assert(page.errors.length === 0, `errors were logged:\n${page.errors.join("\n")}`);
});

check("a native control whose host is put back before it is destroyed keeps its element", async (page) => {
    await page.evaluate(`(themedView.themedViewNativeHost("size", 100, 30), themedView.themedViewNativeHost("add", 0, 0), true)`);
    await page.waitFor(`JSON.parse(${NATIVE_CONTROLS}).some((c) => c.display === "block")`, 10000);
    await page.evaluate(`(document.querySelector("#out .ferroui-native-host").firstElementChild.dataset.mark = "first", true)`);
    // Taken out and put back in one task: the destruction it queued finds the host in a view again.
    await page.evaluate(`(themedView.themedViewNativeHost("remove", 0, 0), themedView.themedViewNativeHost("add", 0, 0), true)`);
    await sleep(500);
    const controls = JSON.parse(await page.evaluate(NATIVE_CONTROLS));
    assert(controls.length === 1 && controls[0].mark === "first" && controls[0].display === "block", `native controls: ${JSON.stringify(controls)}`);
    assert((await page.services()).native_handle === "true", "the native control was destroyed");
});

await run(checks);
