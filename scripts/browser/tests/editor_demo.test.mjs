// Real-browser acceptance checks of the linked Rust editor application.
// Build: scripts/build-browser.sh editor-demo-browser
// Run: node scripts/browser/tests/editor_demo.test.mjs [site] [--screenshot file.png]
// The only application exports called are startup and read-only diagnostics.
// Editing and commands enter through the browser event bridge.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert, headlessClipboardKey as clipboardKey } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const args = process.argv.slice(2);
const screenshotIndex = args.indexOf("--screenshot");
const screenshot = screenshotIndex < 0 ? null : args.splice(screenshotIndex, 2)[1];
const site = path.resolve(args[0] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser/editor-demo-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    throw new Error(`No browser site at ${site}; run scripts/build-browser.sh editor-demo-browser`);
}
const stateExpression = "JSON.parse(globalThis.editorDemo.editorState())";
async function waitForClipboardMarker(page, identifier) {
    const encoded = Buffer.from(identifier, "ascii").toString("hex");
    const type = JSON.stringify(`web application/frn-fmt.text.${encoded}`);
    await page.waitFor(`navigator.clipboard.read().then(async items => {
        const item = items.find(item => item.types.includes(${type}));
        return item && await (await item.getType(${type})).text() === "1";
    })`, 15_000);
}
const state = (page) => page.evaluate(stateExpression);
async function settle(page) {
    assert(await page.evaluate("globalThis.editorDemo.editorRequestFrame()"), "editor has no compositor");
    await page.waitFor("globalThis.editorDemo.editorFrameDrawn()", 60_000);
}
async function waitState(page, predicate) {
    await page.waitFor(`(${predicate})(${stateExpression})`, 30_000);
    await settle(page);
    return state(page);
}
async function clickElement(page, predicate) {
    await settle(page);
    await page.waitFor(`${stateExpression}.elements.some(e => e.hit && (${predicate})(e))`, 30_000);
    const element = await page.evaluate(`${stateExpression}.elements.find(e => e.hit && (${predicate})(e))`);
    await page.click(element.x + element.width / 2, element.y + element.height / 2);
    await settle(page);
}
const toolbar = (page, name) => clickElement(page, `e => e.name === ${JSON.stringify(name)}`);
const focusEditor = (page) => clickElement(page, "e => e.type.endsWith('TextArea')");
async function checkDottedSeparator(page, initial, picture) {
    const area = initial.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    assert(area, "editor text area has no hit target");
    const top = Math.round(area.y + 32);
    const height = 90;
    let separatorX;
    // At the harness's 1× scale, a gutter column alternates between the
    // background and the separator every other pixel. Check the actual frame.
    for (let x = Math.round(area.x + 12); x < Math.round(area.x + 65); x++) {
        const band = Array.from({ length: height }, (_, row) => picture.pixel(x, top + row).join(","));
        const changes = band.filter((color, row) => row >= 1 && color !== band[row - 1]).length;
        const repeats = band.filter((color, row) => row >= 2 && color === band[row - 2]).length;
        if (changes >= height * 0.8 && repeats >= height * 0.85) {
            separatorX = x;
            break;
        }
    }
    assert(separatorX !== undefined, "line-number separator has no visible dots and gaps");
    const digitLeft = Math.max(Math.round(area.x + 6), separatorX - 24);
    let digitRows = 0;
    for (let y = top; y < top + height; y++) {
        const background = picture.pixel(Math.round(area.x + 2), y).join(",");
        for (let x = digitLeft; x <= separatorX - 5; x++) {
            if (picture.pixel(x, y).join(",") !== background) {
                digitRows++;
                break;
            }
        }
    }
    assert(digitRows >= 6, "line numbers produced no visible glyphs in the gutter");
    await page.click(area.x + area.width / 3, top);
    await page.press("a", ["ctrl"]);
    await waitState(page, "s => s.selectedText === s.text");
    await page.click(separatorX, top + 3);
    await settle(page);
    assert((await state(page)).selectedText === initial.text, "clicking the separator changed the selection");
}

async function checkDragSelection(page, initial, unselectedFrame) {
    const area = initial.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    const folding = initial.elements.find(e => e.type.endsWith("FoldingMargin"));
    assert(area && folding, "editor and folding margin must be visible");
    const top = Math.ceil(area.y);
    const left = Math.ceil(folding.x + folding.width);
    const x = left + 46;
    const y = top + 8;
    const isHighlight = ([r, g, b]) =>
        r > 155 && g > 180 && b > 235 && b - r > 26 && g - r > 12;
    await page.press("Home", ["ctrl"]);
    const gutterX = Math.round(folding.x + folding.width / 2);
    const cursorAt = (x, y) => `getComputedStyle(document.elementFromPoint(${x}, ${y})).cursor`;
    await page.mouseMove(gutterX, y);
    await page.waitFor(`${cursorAt(gutterX, y)} !== "text"`, 5_000);
    await page.mouseMove(x, y);
    await page.waitFor(`${cursorAt(x, y)} === "text"`, 5_000);
    await page.mouseDown(x, y);
    try {
        for (const [moves, minimumLength, minimumHighlight] of [
            [[24, 43], 20, 100],
            [[64, 80], 60, 300],
        ]) {
            for (const dy of moves) {
                await page.send("Input.dispatchMouseEvent", {
                    type: "mouseMoved", x, y: y + dy, buttons: 1, pointerType: "mouse",
                });
            }
            await waitState(page, `s => s.selectedText.length > ${minimumLength} && s.selectedText.includes(${JSON.stringify("\n")})`);
            assert(await page.evaluate(`${cursorAt(x, y + moves.at(-1))} === "text"`), "drag selection lost the I-beam cursor");
            const selectedFrame = await page.screenshot();
            let newHighlight = 0;
            for (let row = y; row < y + moves.at(-1) + 2; row += 2) {
                for (let col = left + 1; col < left + 300; col += 2) {
                    if (isHighlight(selectedFrame.pixel(col, row)) && !isHighlight(unselectedFrame.pixel(col, row))) {
                        newHighlight++;
                    }
                }
            }
            assert(newHighlight > minimumHighlight, `drag selection covered only ${newHighlight} sampled pixels`);
        }
        for (const [dragY, expectedCursor] of [[top - 6, false], [y + 80, true]]) {
            await page.send("Input.dispatchMouseEvent", {
                type: "mouseMoved", x, y: dragY, buttons: 1, pointerType: "mouse",
            });
            await page.waitFor(`(${cursorAt(x, dragY)} === "text") === ${expectedCursor}`, 5_000);
        }
    } finally {
        await page.mouseUp(x, y + 80);
    }
}

async function checkPointerHover(page, initial) {
    const area = initial.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    const folding = initial.elements.find(e => e.type.endsWith("FoldingMargin"));
    assert(area && folding, "editor and folding margin must be visible");
    const outside = { x: area.x + area.width / 3, y: area.y - 8 };
    const x = Math.ceil(folding.x + folding.width) + 46;
    const y = Math.ceil(area.y) + 8;
    await page.mouseMove(outside.x, outside.y);
    const before = await state(page);
    await page.mouseMove(x, y);
    await page.mouseMove(x + 1, y + 1);
    await waitState(page, `s => s.hoverStarts === ${before.hoverStarts + 1}`);
    assert((await state(page)).hoverStops === before.hoverStops, "small motion stopped hover");
    await page.mouseMove(x + 12, y + 12);
    await waitState(page, `s => s.hoverStops === ${before.hoverStops + 1}`);
    await waitState(page, `s => s.hoverStarts === ${before.hoverStarts + 2}`);
    await page.mouseMove(outside.x, outside.y);
    await waitState(page, `s => s.hoverStops === ${before.hoverStops + 2}`);
}

async function clickFirstFoldingMarker(page) {
    await settle(page);
    const gutter = (await state(page)).elements.find(e => e.type.endsWith("FoldingMargin"));
    assert(gutter, "the editor has no visible folding margin");
    const picture = await page.screenshot();
    const x = Math.floor(gutter.x + gutter.width / 2);
    const left = Math.floor(gutter.x + 1);
    const bottom = Math.min(picture.height - 1, Math.floor(gutter.y + gutter.height - 5));
    for (let y = Math.ceil(gutter.y); y < bottom; y++) {
        const background = picture.pixel(left, y).join(",");
        // A box outline spans the marker, while the vertical guide occupies one column.
        if ([-3, -2, -1, 0, 1, 2, 3].every(dx => picture.pixel(x + dx, y).join(",") !== background)) {
            await page.click(x, y + 3);
            return;
        }
    }
    throw new Error("the folding margin painted no marker box");
}
async function beginComposition(page, text) {
    const before = (await state(page)).text;
    // The browser platform receives composition events from its hidden native
    // input element. CDP's insertText commits a composition but does not start
    // one: begin it explicitly to exercise the IME client and preedit rendering.
    const start = await page.send("Input.imeSetComposition", {
        text, selectionStart: text.length, selectionEnd: text.length,
    });
    assert(!start.error, `could not start IME composition: ${JSON.stringify(start)}`);
    await settle(page);
    assert((await state(page)).text === before, "IME preedit changed the document before commit");
}
async function finishComposition(page, text) {
    const commit = await page.send("Input.insertText", { text });
    assert(!commit.error, `could not commit IME composition: ${JSON.stringify(commit)}`);
}
async function pressEnterDuringComposition(page) {
    await page.evaluate(`(() => {
        globalThis.editorEnterDuringComposition = false;
        document.addEventListener("keydown", event => {
            globalThis.editorEnterDuringComposition = event.key === "Enter" && event.isComposing && event.isTrusted;
        }, { capture: true, once: true });
    })()`);
    await page.press("Enter");
    await settle(page);
    assert(await page.evaluate("globalThis.editorEnterDuringComposition"),
        "the browser did not deliver a trusted Enter key during composition");
}
async function commitComposition(page, text) {
    await beginComposition(page, text);
    await finishComposition(page, text);
}
async function replaceDocument(page, text, command = "ctrl") {
    await focusEditor(page);
    await page.press("a", [command]);
    if (text.length) await commitComposition(page, text);
    else await page.press("Backspace");
    await waitState(page, `s => s.text === ${JSON.stringify(text)}`);
}

async function checkStationaryLinkCursor(page) {
    await replaceDocument(page, "https://example.org");
    const current = await state(page);
    const area = current.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    const folding = current.elements.find(e => e.type.endsWith("FoldingMargin"));
    assert(area && folding, "editor and folding margin must be visible");
    const x = Math.ceil(folding.x + folding.width) + 12;
    const y = Math.ceil(area.y) + 8;
    const cursor = `getComputedStyle(document.elementFromPoint(${x}, ${y})).cursor`;
    await page.mouseMove(x, y);
    await page.waitFor(`${cursor} === "text"`, 5_000);
    const control = { key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17 };
    await page.send("Input.dispatchKeyEvent", { type: "rawKeyDown", modifiers: 2, ...control });
    try {
        await page.waitFor(`${cursor} === "pointer"`, 5_000);
    } finally {
        await page.send("Input.dispatchKeyEvent", { type: "keyUp", modifiers: 0, ...control });
    }
    await page.waitFor(`${cursor} === "text"`, 5_000);
}

async function checkPasteEventFallback(page) {
    const text = "event-backed paste 😀";
    await replaceDocument(page, "");
    await page.evaluate(`(async () => {
        await navigator.clipboard.writeText(${JSON.stringify(text)});
        Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
        globalThis.editorPasteWasTrusted = false;
        document.addEventListener("paste", event => { globalThis.editorPasteWasTrusted = event.isTrusted; },
            { once: true, capture: true });
    })()`);
    try {
        await page.press("v", ["ctrl"], clipboardKey);
        await waitState(page, `s => s.text === ${JSON.stringify(text)} && s.editorError === null`);
        assert(await page.evaluate("globalThis.editorPasteWasTrusted"), "browser did not deliver a trusted paste event");
    } finally {
        await page.evaluate("delete navigator.clipboard");
    }
    await page.press("z", ["ctrl"]);
    await waitState(page, "s => s.text === ''");
}

async function checkEditorWatermark(page) {
    const empty = await state(page);
    const gutter = empty.elements.find(e => e.type.endsWith("FoldingMargin"));
    const area = empty.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    assert(gutter && area && empty.text === "", "watermark needs an empty editor");
    const left = Math.ceil(gutter.x + gutter.width) + 5;
    const top = Math.ceil(area.y);
    const hint = await page.screenshot();
    await page.press("Enter");
    await waitState(page, `s => s.text === ${JSON.stringify("\n")}`);
    const hidden = await page.screenshot();
    await page.press("z", ["ctrl"]);
    await waitState(page, "s => s.text === '' && s.caret === 0");
    const restored = await page.screenshot();
    let glyphPixels = 0;
    let restoredPixels = 0;
    for (let y = top; y < top + 26; y++) {
        for (let x = left; x < left + 120; x++) {
            if (hint.pixel(x, y)[0] + 20 < hidden.pixel(x, y)[0]) {
                glyphPixels++;
                if (restored.pixel(x, y)[0] === hint.pixel(x, y)[0]) restoredPixels++;
            }
        }
    }
    assert(glyphPixels > 60, `watermark painted only ${glyphPixels} detectable pixels`);
    assert(restoredPixels === glyphPixels, "undo did not restore the painted watermark");
}

async function checkWrappedScrollOnPointerExit(page) {
    await toolbar(page, "RustLanguage");
    const wrapping = (await state(page)).wordWrap;
    if (!wrapping) await toolbar(page, "Wrap");
    await waitState(page, "s => s.wordWrap");
    const line = "ab אבג 😀 e\u0301 " + "0123456789".repeat(13) + " end\n";
    await replaceDocument(page, line.repeat(32));
    await page.press("Home", ["ctrl"]);
    await waitState(page, "s => s.caret === 0 && s.scrollY === 0");
    await page.press("PageDown");
    await settle(page);
    await page.press("PageDown");
    const scrolled = await waitState(page, "s => s.scrollY > 100");
    const area = scrolled.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    assert(area, "editor text area has no hit target");
    const x = Math.ceil(area.x + area.width / 3);
    await page.mouseMove(x, Math.ceil(area.y + area.height / 2));
    await settle(page);
    const before = await state(page);
    await page.mouseMove(x, Math.ceil(area.y) - 2);
    await settle(page);
    const after = await state(page);
    assert(after.caret === before.caret, "pointer exit changed the caret");
    assert(after.scrollY === before.scrollY, `pointer exit scrolled the wrapped view from ${before.scrollY} to ${after.scrollY}`);
}

async function checkCompletionPointerInput(page) {
    await replaceDocument(page, `pri\n${"// completion scroll fixture\n".repeat(100)}`);
    await page.press("Home", ["ctrl"]);
    await page.press("End");
    const editor = await waitState(page, "s => s.caret === 3 && s.scrollY === 0");
    const area = editor.elements.find(e => e.hit && e.type.endsWith("TextArea"));
    const gutter = editor.elements.find(e => e.type.endsWith("FoldingMargin"));
    assert(area && gutter, "completion fixture has no editor viewport or gutter");
    const option = "s.elements.some(e => e.text === 'println!')";
    const openCompletion = async () => {
        await page.press("Space", ["ctrl"]);
        await waitState(page, `s => ${option}`);
    };
    await openCompletion();
    await page.click(gutter.x + gutter.width + 10, area.y + 7);
    await settle(page);
    const afterClick = await state(page);
    assert(afterClick.caret > 0 && afterClick.caret < 3,
        `completion swallowed the editor click at caret ${afterClick.caret}`);
    await waitState(page, `s => ${option} && s.focus.endsWith('TextArea')`);

    const x = area.x + area.width * 0.65;
    const y = area.y + area.height * 0.55;
    await page.wheel(x, y, 0, 4);
    await waitState(page, `s => s.scrollY > 0 && ${option}`);
    await page.wheel(x, y, 0, 600);
    await waitState(page, `s => s.scrollY > 50 && !${option}`);

    await page.press("Home", ["ctrl"]);
    await page.press("End");
    await waitState(page, "s => s.caret === 3 && s.scrollY === 0");
    await openCompletion();
    await toolbar(page, "Find");
    await waitState(page, `s => s.searchOpen && s.focus.endsWith('TextBox') && !${option}`);
    await page.press("Escape");
    await waitState(page, "s => !s.searchOpen");

    await openCompletion();
    await clickElement(page, "e => e.text === 'println!'");
    await waitState(page, `s => s.text.startsWith('println!("Hello, FerroUI!");\\n') && !${option}`);
}

async function checkEditorKeyGestures(page) {
    const text = `zero one two\n${"// page scroll fixture\n".repeat(100)}`;
    await replaceDocument(page, text);
    await page.press("Home", ["ctrl"]);
    await page.press("End");
    await page.press("ArrowLeft", ["shift"]);
    await waitState(page, "s => s.selectedText === 'o' && s.caret === 11 && s.scrollY === 0");
    await page.press("Escape");
    const afterEscape = await state(page);
    assert(afterEscape.selectedText === "o" && afterEscape.caret === 11,
        "Escape changed the editor selection while the search panel was closed");

    await page.evaluate(`(() => {
        globalThis.editorHostGestures = [];
        document.addEventListener('keydown', function observeHostGesture(event) {
            if ((event.altKey && event.key === 'ArrowLeft') ||
                (event.ctrlKey && (event.key === 'Enter' || event.key === 'Escape'))) {
                globalThis.editorHostGestures.push([event.key, event.defaultPrevented]);
                event.preventDefault();
                if (globalThis.editorHostGestures.length === 3) {
                    document.removeEventListener('keydown', observeHostGesture);
                }
            }
        });
    })()`);
    await page.press("ArrowLeft", ["alt"]);
    await page.press("Enter", ["ctrl"]);
    await page.press("Escape", ["ctrl"]);
    const hostGestures = await page.evaluate("globalThis.editorHostGestures");
    assert(JSON.stringify(hostGestures) === JSON.stringify([["ArrowLeft", false], ["Enter", false], ["Escape", false]]),
        `the editor consumed a host shortcut: ${JSON.stringify(hostGestures)}`);

    // Deliver the chord to the page without invoking Chromium's tab shortcut.
    await page.press("PageDown", ["ctrl"], { virtualKeyCodes: false });
    const scrolled = await waitState(page, "s => s.scrollY > 100");
    assert(scrolled.text === text && scrolled.selectedText === "o" && scrolled.caret === 11,
        "a modified key changed the document or caret");
}

async function checkFindPreviousRange(page) {
    await replaceDocument(page, "😀 one / one / one");
    await toolbar(page, "Find");
    await waitState(page, "s => s.searchOpen && s.focus.endsWith('TextBox')");
    await page.type("one");
    await waitState(page, "s => s.searchPattern === 'one'");
    await focusEditor(page);
    await page.press("Home", ["ctrl"]);
    for (let step = 0; step < 6; step++) await page.press("ArrowRight");
    await page.press("End", ["ctrl", "shift"]);
    await waitState(page, "s => s.selection.anchor === 7 && s.selection.caret === 18");
    await page.press("F3", ["shift"]);
    await waitState(page, "s => s.selection.anchor === 3 && s.selection.caret === 6 && s.selectedText === 'one'");
    await page.press("Escape");
    await waitState(page, "s => !s.searchOpen");
}

async function checkRectangleNavigation(page, command, box) {
    const line = "xxxx\n";
    const text = line.repeat(80) + "xxxx";
    await replaceDocument(page, text, command);
    await page.press("Home", [command]);
    await page.press("ArrowRight");
    await page.press("ArrowRight", [box, "shift"]);
    await waitState(page, "s => s.selectedText === 'x' && s.caret === 2");

    await page.press("PageDown", ["shift"]);
    const moved = await waitState(page, "s => s.caret > 10");
    const rows = Math.floor(moved.caret / line.length) + 1;
    assert(moved.caret % line.length === 2 && moved.selectedText === Array(rows).fill("x").join("\n"),
        "Shift+PageDown lost the rectangular selection");
    await page.press("PageUp", ["shift"]);
    await waitState(page, "s => s.selectedText === 'x' && s.caret === 2");

    await page.press("End", [command, "shift"]);
    const end = await waitState(page, "s => s.caret === s.text.length");
    assert(end.selectedText === Array(81).fill("xxx").join("\n"),
        "extending to the end of the document lost the rectangular selection");
    await page.press("Home", [command, "shift"]);
    await waitState(page, "s => s.selectedText === 'x' && s.selection.anchor === 1 && s.caret === 0");

    const columnText = "aaa\nbbb\nccc\nlast";
    await replaceDocument(page, columnText, command);
    await page.press("Home", [command]);
    await page.press("ArrowRight");
    await page.press("ArrowDown", [box, "shift"]);
    await page.press("ArrowDown", [box, "shift"]);
    await waitState(page, "s => s.caret === 9 && s.selection.anchor === 9");
    await page.press("d", ["ctrl"], { virtualKeyCodes: false });
    await waitState(page, `s => s.text === ${JSON.stringify("aaa\nbbb\nlast")} && s.caret === 8`);
    await page.press("z", [command]);
    await waitState(page, `s => s.text === ${JSON.stringify(columnText)} && s.caret === 9`);
}

async function setBrowserLanguage(page, tag) {
    await page.evaluate(`(() => {
        if (${tag === null}) delete navigator.language;
        else Object.defineProperty(navigator, "language", { value: ${JSON.stringify(tag)}, configurable: true });
        dispatchEvent(new Event("languagechange"));
    })()`);
}

async function checkStandaloneDeletion(page) {
    const source = "aa 😀\u0301 café";
    await replaceDocument(page, source);
    await page.press("End");
    for (let i = 0; i < 4; i++) await page.press("ArrowLeft", ["shift"]);
    await waitState(page, "s => s.selectedText === 'café'");

    async function browserDelete(command, inputType, expected, caret) {
        // Chromium performs an actual proxy edit and emits a trusted input
        // event here without keydown or beforeinput, as some soft input does.
        const result = await page.evaluate(`(() => {
            const proxy = document.activeElement;
            if (!proxy.classList.contains('ferroui-input-element')) return null;
            const events = [];
            const observe = event => events.push([event.type, event.inputType ?? null, event.isTrusted]);
            for (const type of ['keydown', 'beforeinput', 'input']) proxy.addEventListener(type, observe);
            const accepted = document.execCommand(${JSON.stringify(command)});
            for (const type of ['keydown', 'beforeinput', 'input']) proxy.removeEventListener(type, observe);
            return { accepted, events, value: proxy.value, selection: [proxy.selectionStart, proxy.selectionEnd] };
        })()`);
        assert(result?.accepted && JSON.stringify(result.events) === JSON.stringify([["input", inputType, true]]),
            `Chromium did not deliver the expected standalone deletion: ${JSON.stringify(result)}`);
        assert(result.value === expected && result.selection[0] === caret && result.selection[1] === caret,
            `the browser proxy did not mirror the deletion: ${JSON.stringify(result)}`);
        await waitState(page, `s => s.text === ${JSON.stringify(expected)} && s.caret === ${caret}`);
    }

    const withoutSelection = "aa 😀\u0301 ";
    await browserDelete("delete", "deleteContentBackward", withoutSelection, withoutSelection.length);
    await page.press("z", ["ctrl"]);
    await waitState(page, `s => s.text === ${JSON.stringify(source)}`);
    await page.press("Home");
    for (let i = 0; i < 3; i++) await page.press("ArrowRight");
    await waitState(page, "s => s.caret === 3 && s.selectedText === ''");
    await browserDelete("forwardDelete", "deleteContentForward", "aa  café", 3);
    await page.press("z", ["ctrl"]);
    await waitState(page, `s => s.text === ${JSON.stringify(source)}`);
    await page.press("Delete");
    await waitState(page, "s => s.text === 'aa  café' && s.caret === 3");
}

async function checkBrowserCorrection(page) {
    const prefix = "// header\r\n";
    const line = "prefix teh 🙂 suffix";
    const original = prefix + line;
    await replaceDocument(page, original);
    await waitState(page, `s => s.caret === ${original.length}`);

    for (const [replacement, hasBeforeInput] of [["the", true], ["their", false]]) {
        const proxy = await page.evaluate(`(() => {
            const input = document.activeElement;
            if (!input.classList.contains('ferroui-input-element') || input.value !== ${JSON.stringify(line)}) return null;
            const event = { inputType: 'insertReplacementText', data: ${JSON.stringify(replacement)}, bubbles: true };
            // Replay the DOM sequence outside any native IME session. The
            // editor caret begins at line end, well beyond the corrected word.
            if (${hasBeforeInput}) input.dispatchEvent(new InputEvent('beforeinput', { ...event, cancelable: true }));
            input.setRangeText(event.data, 7, 10, 'end');
            input.dispatchEvent(new InputEvent('input', event));
            return { value: input.value, position: input.selectionEnd };
        })()`);
        const corrected = `prefix ${replacement} 🙂 suffix`;
        const localCaret = 7 + replacement.length;
        assert(proxy?.value === corrected && proxy.position === localCaret,
            `the browser proxy did not mirror the correction: ${JSON.stringify(proxy)}`);
        await waitState(page, `s => s.text === ${JSON.stringify(prefix + corrected)} && s.caret === ${prefix.length + localCaret}`);
        await page.press("z", ["ctrl"]);
        await waitState(page, `s => s.text === ${JSON.stringify(original)} && s.caret === ${original.length}`);
    }
}

const checks = [];
for (const mode of ["Software2D", "WebGL2"]) {
    checks.push([`${mode}: linked editor, real Oniguruma, input, popups, undo, snippets, folding and resize`, async () => {
        const page = await open(site, { query: `?RenderingMode=${mode}`, width: 1180, height: 800, network: true });
        try {
            await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
            await page.waitForView();
            const initial = await waitState(page, "s => s && s.frames > 0 && s.tokenCount > 10 && s.cachedLines > 0");
            assert(initial.grammar === "source.rust", `unexpected initial grammar: ${initial.grammar}`);
            assert(initial.font === "Noto Mono", `code font did not resolve: ${initial.font}`);
            const fallbackFonts = ["WenQuanYi Micro Hei", "WenQuanYi Micro Hei", "WenQuanYi Micro Hei", "Noto Color Emoji"];
            assert(initial.sampleFallbackFonts?.length === fallbackFonts.length
                && fallbackFonts.every((family, i) => initial.sampleFallbackFonts[i] === family),
            `Japanese text and emoji did not resolve: ${JSON.stringify(initial.sampleFallbackFonts)}`);
            assert(initial.grammarError === null && initial.editorError === null, `initial editor error: ${JSON.stringify(initial)}`);
            const picture = await page.screenshot(mode === "Software2D" ? screenshot : undefined);
            const colors = new Set();
            for (let y = 0; y < picture.height; y += 4) {
                for (let x = 0; x < picture.width; x += 4) colors.add(picture.pixel(x, y).join(","));
            }
            assert(colors.size > 30, `editor canvas is blank or unpainted: ${colors.size} colors`);
            await checkDottedSeparator(page, initial, picture);
            await checkDragSelection(page, initial, picture);
            await checkPointerHover(page, initial);

            await focusEditor(page);
            await page.press("Home", ["ctrl"]);
            await waitState(page, "s => s.caret === 0 && s.selectedText === ''");
            await page.press("PageDown", ["shift"]);
            await waitState(page, "s => s.selection.anchor === 0 && s.selectedText.includes('\\n')");
            await page.press("Home", ["ctrl"]);
            await waitState(page, "s => s.caret === 0 && s.selectedText === ''");

            await checkEditorKeyGestures(page);
            await checkFindPreviousRange(page);
            await checkRectangleNavigation(page, "ctrl", "alt");
            await checkStandaloneDeletion(page);
            await checkBrowserCorrection(page);
            await checkStationaryLinkCursor(page);
            await replaceDocument(page, initial.text);

            // An emoji picker can commit text without a keypress or an IME session.
            await focusEditor(page);
            await page.press("a", ["ctrl"]);
            await waitState(page, "s => s.selectedText === s.text");
            const emoji = "🌐";
            const standalone = await page.send("Input.insertText", { text: emoji });
            assert(!standalone.error, `could not insert standalone text: ${JSON.stringify(standalone)}`);
            await waitState(page, `s => s.text === ${JSON.stringify(emoji)} && s.caret === ${emoji.length}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(initial.text)}`);

            // UTF-16 supplementary and combining sequences travel through the
            // real browser IME composition/commit path and the native undo stack.
            const unicode = "// 😀 e\u0301";
            await replaceDocument(page, unicode);
            assert((await state(page)).caret === unicode.length, "caret did not count UTF-16 units");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(initial.text)}`);
            await page.press("y", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(unicode)}`);
            // Redo maps the restored replacement selection through the edit.
            // Move to the end of the restored text before appending another line.
            await page.press("End");
            await waitState(page, `s => s.selection.anchor === ${unicode.length} && s.selection.caret === ${unicode.length}`);
            await page.press("Enter", ["shift"]);
            await page.type("let browser_smoke = 42;");
            const clipboardText = `${unicode}\nlet browser_smoke = 42;`;
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardText)}`);

            // The browser's native input mirrors the active line. A selection
            // on the second line uses local UTF-16 offsets in that input.
            await page.press("Home");
            for (let index = 0; index < 3; index++) {
                await page.press("ArrowRight", ["shift"]);
            }
            await waitState(page, "s => s.selectedText === 'let'");
            await page.waitFor(`(() => {
                const input = document.activeElement;
                return input instanceof HTMLInputElement
                    && input.value === "let browser_smoke = 42;"
                    && input.selectionStart === 0 && input.selectionEnd === 3;
            })()`, 5_000);
            const laterLineCommit = "編😀";
            await commitComposition(page, laterLineCommit);
            const composedText = `${unicode}\n${laterLineCommit} browser_smoke = 42;`;
            await waitState(page, `s => s.text === ${JSON.stringify(composedText)}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardText)}`);

            // The proxy input is a single line, but its offsets still map to
            // every UTF-16 unit of a backward selection across editor lines.
            await page.press("End", ["ctrl"]);
            await page.press("Home", ["ctrl", "shift"]);
            await waitState(page, "s => s.selection.anchor === s.text.length && s.selection.caret === 0");
            const flattened = clipboardText.replace(/[\r\n]/g, " ");
            await page.waitFor(`(() => {
                const input = document.activeElement;
                return input instanceof HTMLInputElement
                    && input.value === ${JSON.stringify(flattened)}
                    && input.selectionStart === 0
                    && input.selectionEnd === ${clipboardText.length}
                    && input.selectionDirection === "backward";
            })()`, 5_000);
            await commitComposition(page, "段😀");
            await waitState(page, "s => s.text === '段😀'");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardText)}`);

            // Native rich copy/cut/paste uses the browser Clipboard API. Read
            // both formats back independently of the editor's own export code.
            await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
            await page.setPermission("clipboard-read", "granted");
            await page.setPermission("clipboard-write", "granted");
            await page.press("a", ["ctrl"]);
            await page.press("c", ["ctrl"], clipboardKey);
            // Chromium on Windows publishes CRLF in the OS clipboard.
            // Compare logical lines for export and exact clipboard text for paste.
            await page.waitFor(`navigator.clipboard.readText().then(text => text.replace(/\\r\\n/g, "\\n") === ${JSON.stringify(clipboardText)})`, 15_000);
            const clipboardPayload = await page.evaluate("navigator.clipboard.readText()");
            const richCopy = await page.evaluate(`navigator.clipboard.read().then(async items => {
                const item = items.find(item => item.types.includes('text/html'));
                return item ? (await item.getType('text/html')).text() : null;
            })`);
            assert(richCopy?.includes("browser_smoke"), "native copy did not publish HTML beside plain text");
            await page.press("x", ["ctrl"], clipboardKey);
            await waitState(page, "s => s.text === ''");
            await page.press("v", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardPayload)}`);
            await checkPasteEventFallback(page);

            const lineCutText = "first\nsecond";
            await replaceDocument(page, lineCutText);
            await page.press("Home", ["ctrl"]);
            await page.press("d", ["ctrl"]);
            await waitState(page, "s => s.text === 'second' && s.caret === 0");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(lineCutText)} && s.caret === 0`);
            await page.press("x", ["ctrl"], clipboardKey);
            await waitState(page, "s => s.text === 'second' && s.caret === 0");
            await page.waitFor("navigator.clipboard.readText().then(text => text.replace(/\\r\\n/g, '\\n') === 'first\\n')", 15_000);
            await waitForClipboardMarker(page, "FerroUI.Edit.WholeLineSelection");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(lineCutText)} && s.caret === 0`);
            await page.press("End", ["ctrl"]);
            await page.press("ArrowLeft");
            await page.press("v", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text === ${JSON.stringify("first\nfirst\nsecond")}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(lineCutText)} && s.caret === ${lineCutText.length - 1}`);

            const wordDeleteText = "one  two";
            await replaceDocument(page, wordDeleteText);
            await page.press("Home", ["ctrl"]);
            await page.press("Delete", ["ctrl"]);
            await waitState(page, "s => s.text === 'two' && s.caret === 0");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(wordDeleteText)} && s.caret === 0`);
            await page.press("End", ["ctrl"]);
            await page.press("Backspace", ["ctrl"]);
            await waitState(page, "s => s.text === 'one  ' && s.caret === 5");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(wordDeleteText)} && s.caret === 8`);

            await replaceDocument(page, "a\nb");
            await page.press("a", ["ctrl"]);
            await page.press("Tab");
            await waitState(page, `s => s.text === ${JSON.stringify("\ta\n\tb")}
                && s.selectedText === s.text`);
            await page.press("Tab", ["shift"]);
            await waitState(page, `s => s.text === ${JSON.stringify("a\nb")}
                && s.selectedText === s.text`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify("\ta\n\tb")}
                && s.selectedText === s.text`);

            // Completion keeps editor focus and accepts Tab after IME commit.
            await replaceDocument(page, "pr");
            await page.press("Space", ["ctrl"]);
            await waitState(page, `s => {
                const option = s.elements.find(e => e.text === 'println!');
                const description = s.elements.find(e => e.text === 'Print a line to standard output.');
                return option && description && description.x >= option.x + option.width
                    && Math.abs(description.y - option.y) <= option.height && !description.hit;
            }`);
            await beginComposition(page, "i");
            await pressEnterDuringComposition(page);
            const composing = await state(page);
            assert(composing.text === "pr" && composing.focus.endsWith("TextArea")
                && composing.elements.some(e => e.text === "Print a line to standard output."),
                "Enter changed the editor or accepted completion during composition");
            await finishComposition(page, "i");
            await waitState(page, `s => s.text === 'pri'
                && s.elements.some(e => e.text === 'Print a line to standard output.')`);
            await page.press("Tab");
            await waitState(page, `s => s.text === 'println!("Hello, FerroUI!");'
                && !s.elements.some(e => e.text === 'Print a line to standard output.')`);
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === 'pri'");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === 'pr'");
            await checkCompletionPointerInput(page);

            // Linked fields update together and Tab advances to the next field.
            await replaceDocument(page, "");
            await checkEditorWatermark(page);
            await toolbar(page, "Snippet");
            await waitState(page, "s => s.selectedText === 'name'");
            await beginComposition(page, "browser_");
            await pressEnterDuringComposition(page);
            assert((await state(page)).selectedText === "name",
                "Enter moved out of the linked snippet field during composition");
            await finishComposition(page, "browser_");
            await waitState(page, "s => (s.text.match(/browser_/g) ?? []).length === 2");
            await page.type("name");
            await waitState(page, "s => (s.text.match(/browser_name/g) ?? []).length === 2");
            await page.press("Tab");
            await waitState(page, "s => s.selectedText === 'value'");
            await page.type("123");
            await page.press("Tab");
            await waitState(page, "s => s.text.includes('browser_name = 123;') && s.selectedText === ''");

            // Search uses the actual popup TextBox and routes Escape to the editor.
            await toolbar(page, "RustLanguage");
            await toolbar(page, "Find");
            await waitState(page, "s => s.searchOpen && s.focus.endsWith('TextBox')");
            await page.type("FerroUI");
            await waitState(page, "s => s.searchPattern === 'FerroUI'");
            await page.press("Enter");
            const firstMatch = await waitState(page, "s => s.selectedText === 'FerroUI'");
            await page.press("F3");
            await waitState(page, `s => s.selectedText === 'FerroUI' && s.caret !== ${firstMatch.caret}`);
            await page.press("F3", ["shift"]);
            await waitState(page, `s => s.selectedText === 'FerroUI' && s.caret === ${firstMatch.caret}`);
            await page.press("Escape");
            await waitState(page, "s => !s.searchOpen && s.selectedText === 'FerroUI'");

            await toolbar(page, "Replace");
            await clickElement(page, "e => e.placeholder === 'Replace'");
            await page.type("BrowserEdit");
            await focusEditor(page);
            await page.press("End", ["ctrl"]);
            await waitState(page, "s => s.searchOpen && s.caret === s.text.length && s.selectedText === ''");
            await page.press("r", ["alt"]);
            await waitState(page, "s => s.text.includes('BrowserEdit') && s.text.includes('FerroUI')");
            await clickElement(page, "e => e.type.endsWith('Button') && e.text === 'Replace all'");
            await waitState(page, "s => s.text.includes('BrowserEdit') && !s.text.includes('FerroUI')");
            await page.press("Escape");
            await waitState(page, "s => !s.searchOpen");

            // Folding and visual-line regeneration remain valid after changes.
            await clickFirstFoldingMarker(page);
            await waitState(page, "s => s.foldedCount === 1");
            await clickFirstFoldingMarker(page);
            await waitState(page, "s => s.foldedCount === 0");
            await toolbar(page, "Fold");
            await waitState(page, "s => s.foldedCount === 5");
            await toolbar(page, "Unfold");
            await waitState(page, "s => s.foldedCount === 0");
            await replaceDocument(page, "// #region 😀\nfn main() {\n    work();\n}\n// #endregion");
            await toolbar(page, "Fold");
            await waitState(page, "s => s.foldedCount === 2");
            await focusEditor(page);
            await page.press("Home", ["ctrl"]);
            await page.press("Enter");
            await waitState(page, `s => s.text.startsWith(${JSON.stringify("\n// #region")}) && s.foldedCount === 2`);
            await toolbar(page, "Unfold");
            await waitState(page, "s => s.foldedCount === 0");
            await toolbar(page, "Wrap");
            await waitState(page, "s => s.wordWrap");
            await toolbar(page, "Theme");
            await waitState(page, "s => s.theme && s.theme !== null && s.grammarError === null");
            assert((await state(page)).theme !== initial.theme, "theme did not change");
            await toolbar(page, "CSharpLanguage");
            await waitState(page, "s => s.language === 'CSharp' && s.grammar === 'source.cs' && s.tokenCount > 10");

            const literal = '  { "emoji": {{{Format("} // ignored")}}} }  \n  """;';
            const modernCSharp = `class C {\nstring Value() {\nreturn $$"""\n${literal}\n}\n}`;
            const indentedCSharp = `class C {\n\tstring Value() {\n\t\treturn $$"""\n${literal}\n\t}\n}`;
            await replaceDocument(page, modernCSharp);
            await toolbar(page, "Reindent");
            await waitState(page, `s => s.text === ${JSON.stringify(indentedCSharp)}`);
            await focusEditor(page);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(modernCSharp)}`);

            await toolbar(page, "JavaScriptLanguage");
            await waitState(page, "s => s.language === 'JavaScript' && s.grammar === 'source.js' && s.tokenCount > 10");
            await replaceDocument(page, "\t/** 🌐  */");
            for (let i = 0; i < 4; i++) await page.press("ArrowLeft");
            await waitState(page, `s => s.caret === ${"\t/** 🌐".length}`);
            const docComment = "\t/** 🌐\n\t * \n\t  */";
            await page.press("Enter");
            await waitState(page, `s => s.text === ${JSON.stringify(docComment)} && s.caret === ${"\t/** 🌐\n\t * ".length}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify("\t/** 🌐  */")} && s.caret === ${"\t/** 🌐".length}`);

            const boxText = "abcdef\nghijkl\nmnopqr";
            await replaceDocument(page, boxText);
            await page.press("Home", ["ctrl"]);
            await page.press("ArrowRight");
            for (let i = 0; i < 4; i++) await page.press("ArrowRight", ["alt", "shift"]);
            for (let i = 0; i < 2; i++) await page.press("ArrowDown", ["alt", "shift"]);
            await page.press("x", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text === ${JSON.stringify("af\ngl\nmr")}`);
            await page.waitFor(`navigator.clipboard.readText().then(text => text.replace(/\\r\\n/g, "\\n") === ${JSON.stringify("bcde\nhijk\nnopq")})`, 15_000);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(boxText)}`);
            await page.evaluate(`navigator.clipboard.writeText(${JSON.stringify("X\nY")})`);
            await page.press("v", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text === ${JSON.stringify("aXf\ngYl\nmnopqr")} && s.caret === 10`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(boxText)}`);
            await page.press("Enter");
            await waitState(page, `s => s.text === ${JSON.stringify("af\ngl\nmnopqr")} && s.caret === 7`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(boxText)}`);

            await page.press("c", ["ctrl"], clipboardKey);
            await waitForClipboardMarker(page, "FerroUI.Edit.RectangularSelection");
            const columnTarget = "12\n34\n56";
            await replaceDocument(page, columnTarget);
            await page.press("Home", ["ctrl"]);
            await page.press("ArrowRight");
            await page.press("v", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text === ${JSON.stringify("1bcde2\n3hijk4\n5nopq6")} && s.caret === 19`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(columnTarget)} && s.caret === 1`);

            await replaceDocument(page, "/*");
            await page.press("*", [], { virtualKeyCodes: false });
            await waitState(page, "s => s.text === '/** */' && s.caret === 3");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '/**' && s.caret === 3");
            await page.press("y", ["ctrl"]);
            await waitState(page, "s => s.text === '/** */' && s.caret === 3");
            await page.press("z", ["ctrl"]);
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '/*' && s.caret === 2");

            await replaceDocument(page, "\nemoji 😀 )");
            await page.press("Home", ["ctrl"]);
            await page.press("(", [], { virtualKeyCodes: false });
            await waitState(page, `s => s.text === ${JSON.stringify("(\nemoji 😀 )")} && s.caret === 1`);

            await replaceDocument(page, '\nconst example = ")";\n()');
            await page.press("Home", ["ctrl"]);
            await page.press("(", [], { virtualKeyCodes: false });
            await waitState(page, `s => s.text === ${JSON.stringify('()\nconst example = ")";\n()')} && s.caret === 1`);

            await replaceDocument(page, "value = ");
            await page.press("(", [], { virtualKeyCodes: false });
            await waitState(page, "s => s.text === 'value = ()' && s.caret === 9");
            await page.press("[", [], { virtualKeyCodes: false });
            await waitState(page, "s => s.text === 'value = ([])' && s.caret === 10");
            await page.press("Backspace");
            await waitState(page, "s => s.text === 'value = ()' && s.caret === 9");
            await page.press(")", [], { virtualKeyCodes: false });
            await waitState(page, "s => s.text === 'value = ()' && s.caret === 10");
            await replaceDocument(page, "(");
            await waitState(page, "s => s.text === '(' && s.caret === 1");

            await replaceDocument(page, "😀");
            await page.press("a", ["ctrl"]);
            await page.press("[", [], { virtualKeyCodes: false });
            await waitState(page, "s => s.text === '[😀]' && s.selection.anchor === 1 && s.selection.caret === 3");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '😀' && s.selection.anchor === 0 && s.selection.caret === 2");

            await replaceDocument(page, "");
            await toolbar(page, "Snippet");
            await waitState(page, `s => s.selectedText === 'name'
                && s.elements.some(e => e.text === 'entry' && e.hit)
                && s.elements.some(e => e.text === 'result' && e.hit)`);
            await page.press("ArrowDown");
            await page.press("Enter");
            await waitState(page, `s => s.text.includes('const entry = value;')
                && s.text.includes('console.log(entry);') && s.selectedText === 'value'
                && !s.elements.some(e => e.text === 'result' && e.hit)`);
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text.includes('const name = value;') && s.text.includes('console.log(name);')");
            await page.press("y", ["ctrl"]);
            await waitState(page, "s => s.text.includes('const entry = value;') && s.text.includes('console.log(entry);')");

            await replaceDocument(page, "");
            await toolbar(page, "Snippet");
            await waitState(page, "s => s.selectedText === 'name' && s.elements.some(e => e.text === 'result' && e.hit)");
            await page.type("customChoice");
            await waitState(page, `s => s.text.includes('const customChoice = value;')
                && s.text.includes('console.log(customChoice);') && !s.elements.some(e => e.text === 'result' && e.hit)`);
            await page.press("Tab");
            await waitState(page, "s => s.selectedText === 'value'");

            await page.type("localValue");
            await waitState(page, "s => s.text.includes('const customChoice = localValue;') && s.text.includes('// localValue')");
            await page.press("Tab");
            await waitState(page, "s => s.text.endsWith('// LOCALVALUE') && s.selectedText === ''");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text.endsWith('// localValue')");
            await page.press("y", ["ctrl"]);
            await waitState(page, "s => s.text.endsWith('// LOCALVALUE')");

            await toolbar(page, "HtmlLanguage");
            await waitState(page, "s => s.language === 'Html' && s.grammar === 'text.html.derivative' && s.tokenCount > 10");
            await replaceDocument(page, "<script>/**  */</script>");
            await page.press("Home", ["ctrl"]);
            for (let i = 0; i < "<script>/**".length; i++) await page.press("ArrowRight");
            await page.press("Enter");
            await waitState(page, `s => s.text === ${JSON.stringify("<script>/**\n * \n  */</script>")}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify("<script>/**  */</script>")}`);

            const embeddedBlock = "<script>\n\tif (value) { // continue here\n</script>";
            await replaceDocument(page, embeddedBlock);
            await page.press("ArrowUp");
            await page.press("End");
            await page.press("Enter");
            const indentedBlock = "<script>\n\tif (value) { // continue here\n\t\t\n</script>";
            await waitState(page, `s => s.text === ${JSON.stringify(indentedBlock)} && s.caret === ${indentedBlock.indexOf("\n</script>")}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(embeddedBlock)}`);

            const mixedIndent = "<section>\n<script>\nif (value) {\nwork();\n  }\n  </script>\n</section>";
            await replaceDocument(page, mixedIndent);
            await toolbar(page, "Reindent");
            const reindented = "<section>\n\t<script>\n\t\tif (value) {\n\t\t\twork();\n\t\t}\n\t</script>\n</section>";
            await waitState(page, `s => s.text === ${JSON.stringify(reindented)}`);
            await focusEditor(page);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(mixedIndent)}`);
            await page.press("i", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(reindented)}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(mixedIndent)}`);

            const beforeCloser = "<script>\n\tif (value) {\n\t\twork();\n\t\t\n</script>";
            await replaceDocument(page, beforeCloser);
            await page.press("ArrowUp");
            await page.press("End");
            await page.press("}", [], { virtualKeyCodes: false });
            const afterCloser = "<script>\n\tif (value) {\n\t\twork();\n\t}\n</script>";
            await waitState(page, `s => s.text === ${JSON.stringify(afterCloser)} && s.caret === ${afterCloser.indexOf("\n</script>")}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(beforeCloser)}`);

            const pasteTarget = "<script>\n\tif (outer) {\n\t\t\n\t}\n</script>";
            await replaceDocument(page, pasteTarget);
            await page.press("ArrowUp");
            await page.press("ArrowUp");
            await page.press("End");
            const pasteBlock = `if (value) {\n${"\twork();\n".repeat(40)}}`;
            await page.evaluate(`navigator.clipboard.writeText(${JSON.stringify(pasteBlock)})`);
            const pasted = `<script>\n\tif (outer) {\n\t\tif (value) {\n${"\t\t\twork();\n".repeat(40)}\t\t}\n\t}\n</script>`;
            await page.press("v", ["ctrl"], clipboardKey);
            await waitState(page, `s => s.text.replaceAll(${JSON.stringify("\r\n")}, ${JSON.stringify("\n")}) === ${JSON.stringify(pasted)}`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(pasteTarget)}`);

            const snippetPrefix = "<script>\n\tfor\n</script>";
            await replaceDocument(page, snippetPrefix);
            await page.press("ArrowUp");
            await page.press("End");
            await page.press("Space", ["ctrl"]);
            await waitState(page, `s => s.elements.some(e => e.text === 'for' && e.hit)`);
            await page.press("Tab");
            await waitState(page, `s => s.selectedText === 'index' && s.text.includes('for (let index = 0; index < array.length; index++)')`);
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(snippetPrefix)}`);
            await page.press("Space", ["ctrl"]);
            await waitState(page, `s => s.elements.some(e => e.text === 'for' && e.hit)`);
            await page.press("Tab");
            await waitState(page, `s => s.selectedText === 'index'`);
            await page.type("script_index");
            await waitState(page, `s => (s.text.match(/script_index/g) ?? []).length === 4`);
            await page.press("Tab");
            await waitState(page, `s => s.selectedText === 'array'`);
            await page.press("Escape");

            await replaceDocument(page, "😀 abba b b e e c d");
            await toolbar(page, "Replace");
            await clickElement(page, "e => e.text === 'Regex' && e.type.endsWith('CheckBox')");
            await clickElement(page, "e => e.placeholder === 'Find...'");
            await page.press("a", ["ctrl"]);
            await page.type("(?<word>a)(b)\\1\\2");
            await clickElement(page, "e => e.placeholder === 'Replace'");
            await page.press("a", ["ctrl"]);
            await page.type("$1/$2/${word}/$+");
            await setBrowserLanguage(page, "zh-CN");
            await waitState(page, "s => s.searchOpen && s.selectedText === 'abba' && s.elements.some(e => e.placeholder === '查找...' && e.hit) && s.elements.some(e => e.text === '第1项，共1项')");
            await setBrowserLanguage(page, "zh-TW");
            await waitState(page, "s => s.elements.some(e => e.placeholder === 'Find...' && e.hit) && s.elements.some(e => e.text === '1 of 1')");
            await setBrowserLanguage(page, null);
            await clickElement(page, "e => e.type.endsWith('Button') && e.text === 'Replace all'");
            await waitState(page, "s => s.text === '😀 b/a/a/a b b e e c d'");

            await clickElement(page, "e => e.placeholder === 'Find...'");
            await page.press("a", ["ctrl"]);
            await page.type(String.raw`(?<10>[a-z-[aeiou]])\040\10`);
            await waitState(page, "s => s.selectedText === 'b b' && s.elements.some(e => e.text === '1 of 1')");
            await clickElement(page, "e => e.placeholder === 'Replace'");
            await page.press("a", ["ctrl"]);
            await page.type("[$10]");
            await clickElement(page, "e => e.type.endsWith('Button') && e.text === 'Replace all'");
            await waitState(page, "s => s.text === '😀 b/a/a/a [b] e e c d'");

            await clickElement(page, "e => e.placeholder === 'Find...'");
            await page.press("a", ["ctrl"]);
            await page.type(String.raw`(\uD83D\uDE00)`);
            await waitState(page, "s => s.selectedText === '😀' && s.selection.anchor === 0 && s.selection.caret === 2 && s.elements.some(e => e.text === '1 of 1')");
            await clickElement(page, "e => e.placeholder === 'Replace'");
            await page.press("a", ["ctrl"]);
            await page.type("<$1>");
            await clickElement(page, "e => e.type.endsWith('Button') && e.text === 'Replace all'");
            await waitState(page, "s => s.text === '<😀> b/a/a/a [b] e e c d'");
            await page.press("Escape");
            await waitState(page, "s => !s.searchOpen");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '😀 b/a/a/a [b] e e c d'");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '😀 b/a/a/a b b e e c d'");
            await page.press("z", ["ctrl"]);
            await waitState(page, "s => s.text === '😀 abba b b e e c d'");

            await page.resize(800, 620);
            await waitState(page, "s => s.width === 800 && s.height === 620 && s.cachedLines > 0");
            await checkWrappedScrollOnPointerExit(page);
            await page.resize(1320, 900);
            await waitState(page, "s => s.width === 1320 && s.height === 900 && s.cachedLines > 0");
            const final = await state(page);
            assert(final.grammarError === null && final.editorError === null && final.renderPanics === 0,
                `editor/renderer errors: ${JSON.stringify(final)}`);
            assert(page.errors.length === 0, page.errors.join("\n"));
            assert(page.requests.filter(url => new URL(url).pathname.endsWith("ferroui-textmate-oniguruma.js")).length === 1,
                "scanner module was missing or downloaded repeatedly");
        } catch (error) {
            error.message += `\n${page.log.slice(-12).join("\n")}`;
            throw error;
        } finally {
            await page.close();
        }
    }]);
}
checks.push(["Touch: tap, pan and hold-drag selection", async () => {
    const page = await open(site, { query: "?RenderingMode=Software2D", width: 900, height: 560 });
    const contact = async (type, x, y) => {
        const touchPoints = type === "touchEnd" ? [] : [{ x, y, id: 1 }];
        const result = await page.send("Input.dispatchTouchEvent", { type, touchPoints });
        assert(!result.error, `could not dispatch ${type}: ${JSON.stringify(result)}`);
    };
    try {
        await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
        await page.waitForView();
        await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
        const text = "alpha beta gamma\nbravo delta echo\n" + "another editor line\n".repeat(70);
        await replaceDocument(page, text);
        await page.press("Home", ["ctrl"]);
        const top = await waitState(page, "s => s.caret === 0 && s.scrollY === 0");
        const area = top.elements.find(e => e.hit && e.type.endsWith("TextArea"));
        const folding = top.elements.find(e => e.type.endsWith("FoldingMargin"));
        assert(area && folding, "the touch editor has no text viewport");
        const x = Math.ceil(folding.x + folding.width) + 20;
        const y = Math.ceil(area.y) + 8;

        await contact("touchStart", x, y);
        await contact("touchEnd");
        const tapped = await waitState(page, "s => s.caret > 0 && s.caret < 5 && s.selectedText === ''");
        await contact("touchStart", x, y + 150);
        for (const distance of [25, 50, 75, 100, 125]) {
            await contact("touchMove", x, y + 150 - distance);
        }
        await new Promise(resolve => setTimeout(resolve, 250));
        await contact("touchEnd");
        const panned = await waitState(page, "s => s.scrollY > 50");
        assert(panned.caret === tapped.caret && panned.selectedText === "",
            `panning changed caret or selection: ${JSON.stringify(panned.selection)}`);

        await page.press("Home", ["ctrl"]);
        await waitState(page, "s => s.caret === 0 && s.scrollY === 0");
        await contact("touchStart", x, y);
        try {
            await waitState(page, "s => s.selectedText === 'alpha'");
            await contact("touchMove", x + 25, y + 55);
            await waitState(page, "s => s.selectedText.startsWith('alpha') && s.selectedText.includes('bravo')");
            await contact("touchMove", x, y);
            await waitState(page, "s => s.selectedText === 'alpha'");
        } finally {
            await contact("touchEnd");
        }
        const held = await waitState(page, "s => s.selectedText === 'alpha'");
        assert(held.text === text && held.scrollY === 0 && held.editorError === null,
            "holding and dragging must select text without editing or panning the document");

        await page.press("Home", ["ctrl"]);
        await waitState(page, "s => s.caret === 0 && s.selectedText === ''");
        await new Promise(resolve => setTimeout(resolve, 600));
        for (let tap = 0; tap < 2; tap++) {
            await contact("touchStart", x, y);
            await contact("touchEnd");
        }
        await waitState(page, "s => s.selectedText === 'alpha'");
    } finally {
        await page.close();
    }
}]);
checks.push(["Accessibility: live multiline value and named editing focus", async () => {
    const page = await open(site, { query: "?RenderingMode=Software2D", width: 1180, height: 800 });
    try {
        await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
        await page.waitForView();
        await page.evaluate("document.querySelector('.ferroui-enable-accessibility').click()");
        const selector = '[data-ferroui-type="TextEditor"]';
        const textbox = `document.querySelector(${JSON.stringify(selector)})`;
        const inputSelector = ".ferroui-input-element";
        const input = `document.querySelector(${JSON.stringify(inputSelector)})`;
        await page.waitFor(`!!${textbox}`, 10_000);
        await page.evaluate(`${textbox}.click()`);
        await page.waitFor(`${textbox}.closest('[role=application]').getAttribute('aria-activedescendant') === ${textbox}.id`,
            10_000);

        const { root } = await page.send("DOM.getDocument");
        const backendId = async (selector) => {
            const { nodeId } = await page.send("DOM.querySelector", { nodeId: root.nodeId, selector });
            const { node } = await page.send("DOM.describeNode", { nodeId });
            return node.backendNodeId;
        };
        const textboxId = await backendId(selector);
        const inputId = await backendId(inputSelector);
        const accessibilityNode = async (backendNodeId) => {
            const { nodes } = await page.send("Accessibility.getPartialAXTree", {
                backendNodeId, fetchRelatives: false,
            });
            return nodes[0];
        };
        const checkFocusedInput = async (name) => {
            await page.waitFor(`document.activeElement === ${input}`, 10_000);
            const focused = await accessibilityNode(inputId);
            assert(focused.role.value === "textbox" && focused.name.value === name &&
                focused.properties.some(p => p.name === "focused" && p.value.value === true),
                `the focused browser input must be named ${JSON.stringify(name)}, got ${JSON.stringify(focused.name?.value)}`);
        };
        await checkFocusedInput("Enter code");
        assert(await page.evaluate(`${input}.getAttribute('aria-controls') === ${textbox}.id`),
            "the focused input must reference the editor's document textbox");

        const original = (await state(page)).text;
        const initial = await accessibilityNode(textboxId);
        assert(initial.role.value === "textbox" && initial.name.value === "Enter code",
            "the editor is missing its textbox role or accessible name");
        assert(initial.properties.some(p => p.name === "multiline" && p.value.value === true),
            "the accessibility tree treats the editor as a single-line textbox");
        assert(initial.value.value === original, "the initial accessible value differs from the document");

        const replacement = "first 😀 line\nsecond e\u0301 line";
        const checkValue = async (text) => {
            await page.waitFor(`${textbox}.firstChild.nodeValue === ${JSON.stringify(text)}`, 10_000);
            const current = await accessibilityNode(textboxId);
            assert(current.nodeId === initial.nodeId && current.value.value === text,
                "the same accessible textbox must publish the current document value");
        };
        await replaceDocument(page, replacement);
        await checkValue(replacement);
        await page.press("z", ["ctrl"]);
        await waitState(page, `s => s.text === ${JSON.stringify(original)}`);
        await checkValue(original);

        await toolbar(page, "Find");
        await waitState(page, "s => s.searchOpen && s.focus.endsWith('TextBox')");
        await page.waitFor(`document.getElementById(${input}.getAttribute('aria-controls'))
            ?.getAttribute('aria-placeholder') === 'Find...'`, 10_000);
        await checkFocusedInput("Find...");

        await page.evaluate(`(() => {
            const button = document.createElement('button');
            button.textContent = 'Outside the editor';
            document.body.appendChild(button);
            button.focus();
        })()`);
        await page.waitFor(`![...document.querySelectorAll('.ferroui-input-element, .ferroui-text-input-element')]
            .some(element => element.hasAttribute('aria-controls') || element.hasAttribute('aria-label'))`, 10_000);
    } finally {
        await page.close();
    }
}]);
checks.push(["Apple browser: Command shortcuts and Control rectangular selection", async () => {
    const page = await open(site, {
        query: "?RenderingMode=Software2D", width: 1180, height: 800,
        initScript: 'Object.defineProperty(navigator, "platform", { value: "MacIntel" });',
    });
    try {
        await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
        await page.waitForView();
        await page.setPermission("clipboard-read", "granted");
        await page.setPermission("clipboard-write", "granted");
        await checkRectangleNavigation(page, "meta", "ctrl");
        const source = "abcdef\nghijkl\nmnopqr";
        await replaceDocument(page, source, "meta");
        await page.press("Home", ["meta"]);
        await waitState(page, "s => s.caret === 0");
        await page.press("ArrowRight", ["meta"]);
        await waitState(page, "s => s.caret === 6");
        await page.press("ArrowLeft", ["meta"]);
        await waitState(page, "s => s.caret === 0");
        await page.press("ArrowRight");
        await page.press("ArrowRight", ["ctrl", "shift"]);
        await page.press("ArrowDown", ["ctrl", "shift"]);
        await page.press("ArrowDown", ["ctrl", "shift"]);
        await waitState(page, `s => s.selectedText === ${JSON.stringify("b\nh\nn")}`);
        await page.press("c", ["meta"], clipboardKey);
        await waitForClipboardMarker(page, "FerroUI.Edit.RectangularSelection");
        await page.press("x", ["meta"], clipboardKey);
        await waitState(page, `s => s.text === ${JSON.stringify("acdef\ngijkl\nmopqr")}`);
        await page.press("z", ["meta"]);
        await waitState(page, `s => s.text === ${JSON.stringify(source)}`);
        await page.press("a", ["meta"]);
        await waitState(page, "s => s.selectedText === s.text");
        const pasted = "Apple clipboard 😀";
        await page.evaluate(`navigator.clipboard.writeText(${JSON.stringify(pasted)})`);
        await page.press("v", ["meta"], clipboardKey);
        await waitState(page, `s => s.text === ${JSON.stringify(pasted)}`);
        await page.press("z", ["meta"]);
        await waitState(page, `s => s.text === ${JSON.stringify(source)}`);
        await page.press("f", ["meta"]);
        await waitState(page, "s => s.searchOpen && s.focus.endsWith('TextBox')");
    } finally {
        await page.close();
    }
}]);
await run(checks);
