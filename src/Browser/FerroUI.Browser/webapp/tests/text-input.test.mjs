// Exercise the production TypeScript input bridge and export lookup without a
// browser or a Wasm build. EventTarget supplies real listener/cancellation
// semantics; only the browser event fields and native callback boundary are
// represented here. The editor's browser suite covers actual native text edits.
import assert from "node:assert/strict";
import { setTimeout as nextTask } from "node:timers/promises";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const bundled = await build({
    stdin: {
        contents: 'export { InputHelper } from "./modules/ferroui/input"; export { FerroExports } from "./modules/ferroui/ferroExports";',
        resolveDir: fileURLToPath(new URL("..", import.meta.url)),
        sourcefile: "text-input-test.ts",
        loader: "ts",
    },
    bundle: true,
    write: false,
    format: "esm",
    platform: "node",
    target: "node22",
});
const { InputHelper, FerroExports } = await import("data:text/javascript;base64," + Buffer.from(bundled.outputFiles[0].text).toString("base64"));

function event(type, fields = {}, cancelable = true) {
    return Object.assign(new Event(type, { cancelable }), fields);
}
function insertion(data, fields = {}) {
    return event("input", { inputType: "insertText", data, isComposing: false, ...fields });
}
function setup(t, { handled = false, focusedLegacyInput = false } = {}) {
    const calls = [];
    const record = (name) => (...args) => { calls.push([name, ...args]); };
    FerroExports.attach({
        InputHelper_OnTextInput: record("text"),
        InputHelper_OnStandaloneDelete: (...args) => { record("delete")(...args); return true; },
        InputHelper_OnReplacementText: record("replace"),
        InputHelper_OnBeforeInput: record("beforeinput"),
        InputHelper_OnCompositionStart: record("compositionstart"),
        InputHelper_OnCompositionUpdate: record("compositionupdate"),
        InputHelper_OnCompositionEnd: record("compositionend"),
        InputHelper_OnConnectionBeforeInput: (...args) => { record("connection beforeinput")(...args); return 5; },
        InputHelper_OnConnectionInput: record("connection input"),
        InputHelper_OnConnectionCompositionStart: (...args) => { record("connection compositionstart")(...args); return 9; },
        InputHelper_OnConnectionCompositionUpdate: record("connection compositionupdate"),
        InputHelper_OnConnectionCompositionEnd: record("connection compositionend"),
        InputHelper_OnKeyDown: (...args) => { record("keydown")(...args); return handled; },
        InputHelper_OnKeyUp: (...args) => { record("keyup")(...args); return handled; },
    });
    const element = Object.assign(new EventTarget(), { value: "", selectionStart: 0, selectionEnd: 0 });
    if (focusedLegacyInput) {
        element.querySelector = (selector) => selector === ":scope > .ferroui-input-element" ? element : null;
        element.matches = (selector) => selector === ":focus";
        element.style = {};
        element.setSelectionRange = (start, end, direction) => Object.assign(element, {
            selectionStart: start, selectionEnd: end, selectionDirection: direction
        });
    }
    const unsubscribe = InputHelper.subscribeTextEvents(element, 17);
    t.after(unsubscribe);
    return { element, calls, unsubscribe };
}

test("input-only Unicode reaches the native text client once without stale DOM surrounding text", (t) => {
    const { element, calls } = setup(t);
    element.value = "stale surrounding DOM text";
    element.selectionStart = 4;
    element.selectionEnd = 9;
    element.dispatchEvent(insertion("FerroUI · café Ω 🚀"));
    assert.deepEqual(calls, [["text", 17, "FerroUI · café Ω 🚀"]]);
});

test("empty, generic and non-insertion input events do not enter the native text client", (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(new Event("input"));
    element.dispatchEvent(insertion(null));
    element.dispatchEvent(insertion(""));
    for (const inputType of ["deleteContentBackward", "deleteByCut", "historyUndo", "insertFromPaste", "insertFromDrop", "insertLineBreak", "insertReplacementText"]) {
        element.dispatchEvent(insertion("ignored", { inputType }));
    }
    assert.deepEqual(calls, []);
});

test("legacy proxy deletion is dispatched at most once and leaves composition alone", (t) => {
    const { element, calls } = setup(t, { focusedLegacyInput: true });
    const backward = { inputType: "deleteContentBackward", getTargetRanges: () => [] };
    const forward = { ...backward, inputType: "deleteContentForward" };
    assert.equal(element.dispatchEvent(event("beforeinput", backward)), false);
    element.dispatchEvent(event("beforeinput", forward, false));
    assert.deepEqual(calls, [["delete", 17, true]]);
    element.dispatchEvent(event("input", forward));
    const prevented = event("beforeinput", backward);
    prevented.preventDefault();
    element.dispatchEvent(prevented);
    element.dispatchEvent(event("input", { ...backward, isComposing: true }));
    element.matches = () => false;
    element.dispatchEvent(event("input", backward));
    assert.deepEqual(calls, [["delete", 17, true], ["delete", 17, false]]);
});

test("a correction captures directed selection and restores the authoritative proxy", (t) => {
    const { element, calls } = setup(t, { focusedLegacyInput: true });
    InputHelper.setSurroundingText(element, "ab\r\nteh", 7, 4);
    const replacement = { inputType: "insertReplacementText", data: "the" };
    element.dispatchEvent(event("beforeinput", replacement));
    element.value = "ab  the";
    element.setSelectionRange(7, 7, "none");
    element.dispatchEvent(event("input", replacement));
    assert.deepEqual(calls, [["replace", 17, "ab  teh", 7, 4, 7, 4, "ab  the", 7, 7]]);
    assert.deepEqual([element.value, element.selectionStart, element.selectionEnd, element.selectionDirection],
        ["ab  teh", 4, 7, "backward"]);
});

test("a fresh surrounding projection invalidates an earlier replacement", (t) => {
    const { element, calls } = setup(t, { focusedLegacyInput: true });
    InputHelper.setSurroundingText(element, "teh", 3, 3);
    const replacement = { inputType: "insertReplacementText", data: "the" };
    element.dispatchEvent(event("beforeinput", replacement));
    InputHelper.setSurroundingText(element, "teh", 0, 0);
    element.value = "the";
    element.setSelectionRange(3, 3, "none");
    element.dispatchEvent(event("input", replacement));
    assert.deepEqual(calls, []);
    assert.equal(element.value, "teh");
    assert.equal(element.selectionStart, 0);

    // An input-only correction uses the current projection without a prior range.
    element.value = "the";
    element.setSelectionRange(3, 3, "none");
    element.dispatchEvent(event("input", replacement));
    assert.deepEqual(calls, [["replace", 17, "teh", 0, 0, -1, -1, "the", 3, 3]]);
});

test("composition preedit stays separate and its commit is delivered once despite trailing input", (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionupdate", { data: "日本" }));
    element.dispatchEvent(insertion("日本", { isComposing: true }));
    element.dispatchEvent(insertion("日本", { inputType: "insertCompositionText" }));
    // An input event inside an active composition is still preedit even if a
    // platform omitted its isComposing flag.
    element.dispatchEvent(insertion("日本"));
    element.dispatchEvent(event("compositionend", { data: "日本語" }));
    element.dispatchEvent(insertion("日本語", { inputType: "insertFromComposition" }));
    element.dispatchEvent(insertion("日本語"));
    assert.deepEqual(calls, [
        ["compositionstart", 17],
        ["compositionupdate", 17, "日本"],
        ["compositionend", 17, "日本語"],
    ]);
});

test("an identical input-only edit in a later task survives a composition with no trailing input", async (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionend", { data: "é" }));
    await nextTask();
    element.dispatchEvent(insertion("é"));
    assert.deepEqual(calls, [["compositionstart", 17], ["compositionend", 17, "é"], ["text", 17, "é"]]);
});

test("a distinct input-only edit immediately after composition is an independent insertion", (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionend", { data: "語" }));
    element.dispatchEvent(insertion("!"));
    assert.deepEqual(calls, [["compositionstart", 17], ["compositionend", 17, "語"], ["text", 17, "!"]]);
});

test("native client reset accepts fresh input before a canceled composition sends its delayed end", (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionupdate", { data: "old preedit" }));
    // Production subscribes on the containing view, while native reset clears
    // its hidden input. Model that ancestry without requiring a DOM renderer.
    const hiddenInput = { value: "old preedit", parentElement: element };
    InputHelper.clearInput(hiddenInput);
    assert.equal(hiddenInput.value, "");
    calls.length = 0;

    element.dispatchEvent(insertion("resumed · Ω 🚀"));
    element.dispatchEvent(event("compositionupdate", { data: "late preedit" }));
    element.dispatchEvent(event("compositionend", { data: "late commit" }));
    element.dispatchEvent(insertion("late commit"));
    element.dispatchEvent(insertion("!"));
    assert.deepEqual(calls, [["text", 17, "resumed · Ω 🚀"], ["text", 17, "!"]]);
});

test("a native reset permits a new composition without waiting for the canceled one to end", (t) => {
    const { element, calls } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionupdate", { data: "old" }));
    InputHelper.clearInput(element);
    calls.length = 0;

    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionupdate", { data: "new" }));
    element.dispatchEvent(event("compositionend", { data: "新" }));
    element.dispatchEvent(insertion("新"));
    assert.deepEqual(calls, [["compositionstart", 17], ["compositionupdate", 17, "new"], ["compositionend", 17, "新"]]);
});

for (const handled of [false, true]) {
    test(`ordinary character keydown prevents a duplicate DOM insertion (native handled: ${handled})`, (t) => {
        const { element, calls } = setup(t, { handled });
        t.after(InputHelper.subscribeKeyEvents(element, 17));
        const down = event("keydown", { code: "KeyA", key: "a", isComposing: false });
        element.dispatchEvent(down);
        // A browser generates the text insertion as the key's default action.
        if (!down.defaultPrevented) element.dispatchEvent(insertion("a"));
        element.dispatchEvent(event("keyup", { code: "KeyA", key: "a", isComposing: false }));
        assert.equal(down.defaultPrevented, true);
        assert.deepEqual(calls, [["keydown", 17, "KeyA", "a", 0], ["keyup", 17, "KeyA", "a", 0]]);
    });
}

test("unsubscribe removes every text event callback and permits a fresh subscription", async (t) => {
    const { element, calls, unsubscribe } = setup(t);
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionend", { data: "Ω" }));
    unsubscribe();
    calls.length = 0;
    element.dispatchEvent(insertion("unsubscribed"));
    element.dispatchEvent(event("beforeinput", { inputType: "insertText", getTargetRanges: () => [] }));
    element.dispatchEvent(event("compositionstart", { data: "" }));
    element.dispatchEvent(event("compositionupdate", { data: "removed" }));
    element.dispatchEvent(event("compositionend", { data: "removed" }));
    element.dispatchEvent(event("keydown", { key: "a", isComposing: false }));
    element.dispatchEvent(new Event("blur"));
    await nextTask();
    assert.deepEqual(calls, []);
    t.after(InputHelper.subscribeTextEvents(element, 23));
    element.dispatchEvent(insertion("Ω"));
    assert.deepEqual(calls, [["text", 23, "Ω"]]);
});

// The textarea of a client of the exact-edit connection (docs/porting/browser-platform.md,
// section 14) sits in the same view, so its events bubble to the view's listeners too.
function textarea() {
    return Object.assign(new EventTarget(), {
        value: "", selectionStart: 0, selectionEnd: 0, selectionDirection: "none",
        classList: { contains: (name) => name === "ferroui-text-input-element" },
        setSelectionRange(start, end, direction) {
            this.selectionStart = start; this.selectionEnd = end; this.selectionDirection = direction;
        },
    });
}
function bubbled(type, target, fields = {}) {
    const bubbledEvent = event(type, fields);
    Object.defineProperty(bubbledEvent, "target", { value: target });
    return bubbledEvent;
}

const clipboardDocument = new EventTarget();
function setupClipboard(t, clipboard, read) {
    const previousDocument = globalThis.document;
    globalThis.document = clipboardDocument;
    t.after(() => { globalThis.document = previousDocument; });
    InputHelper.initializeBackgroundHandlers();
    const host = new EventTarget();
    const viewWindow = { navigator: { clipboard }, setTimeout };
    FerroExports.attach({
        InputHelper_OnKeyDown: () => { read?.(viewWindow); return true; },
        InputHelper_OnKeyUp: () => true,
    });
    t.after(InputHelper.subscribeKeyEvents(host, 17));
    return { host, viewWindow };
}

const pasteKey = (type = "keydown") => event(type, { code: "KeyV", key: "v", ctrlKey: true });

test("event-backed clipboard reads without a paste gesture report denied", async (t) => {
    const { viewWindow } = setupClipboard(t, {});
    assert.deepEqual(await Promise.race([InputHelper.readClipboard(viewWindow), nextTask(16)]), { error: "denied" });
});

test("an unavailable clipboard interface can read plain text and HTML from a paste event", async (t) => {
    let reads;
    const { host } = setupClipboard(t, undefined, viewWindow => {
        reads = Promise.all([InputHelper.readClipboard(viewWindow), InputHelper.readClipboard(viewWindow)]);
    });
    const down = pasteKey();
    host.dispatchEvent(down);
    assert.equal(down.defaultPrevented, false);
    const clipboardData = {
        items: [{ kind: "string", type: "text/plain" }, { kind: "string", type: "text/html" }],
        getData: format => format === "text/html" ? "<b>paste</b>" : "paste",
    };
    const foreign = bubbled("paste", new EventTarget(), { clipboardData });
    clipboardDocument.dispatchEvent(foreign);
    assert.equal(foreign.defaultPrevented, false);
    const paste = bubbled("paste", host, { clipboardData });
    clipboardDocument.dispatchEvent(paste);
    assert.equal(paste.defaultPrevented, true);
    host.dispatchEvent(pasteKey("keyup"));
    const expected = { result: [
        { type: "string", format: "text/plain", value: "paste" },
        { type: "string", format: "text/html", value: "<b>paste</b>" },
    ] };
    assert.deepEqual(await Promise.race([reads, nextTask(16)]), [expected, expected]);
});

test("a missing paste event releases the request even if the key is never released", async (t) => {
    let read;
    const { host } = setupClipboard(t, {}, viewWindow => { read = InputHelper.readClipboard(viewWindow); });
    const down = pasteKey();
    host.dispatchEvent(down);
    assert.equal(down.defaultPrevented, false);
    assert.deepEqual(await Promise.race([read, nextTask(16)]), { error: "denied" });

    const ordinary = event("keydown", { code: "KeyA", key: "a" });
    host.dispatchEvent(ordinary);
    assert.equal(ordinary.defaultPrevented, true);
    assert.deepEqual(await read, { error: "denied" });
});

test("events of the connection textarea never reach the path of the input element", (t) => {
    const { element, calls } = setup(t);
    const input = textarea();
    element.dispatchEvent(bubbled("compositionstart", input, { data: "" }));
    element.dispatchEvent(bubbled("compositionupdate", input, { data: "か" }));
    element.dispatchEvent(bubbled("beforeinput", input, { inputType: "insertText", getTargetRanges: () => [] }));
    element.dispatchEvent(bubbled("input", input, { inputType: "insertText", data: "x", isComposing: false }));
    const end = bubbled("compositionend", input, { data: "か" });
    element.dispatchEvent(end);
    assert.deepEqual(calls, []);
    assert.equal(end.defaultPrevented, false, "the connection protocol decides about its own composition end");
});

test("the view subscribes the textarea to the connection protocol and leaves the input element to the original path", (t) => {
    const calls = [];
    const record = (name) => (...args) => { calls.push([name, ...args]); };
    FerroExports.attach({
        InputHelper_OnTextInput: record("text"),
        InputHelper_OnConnectionBeforeInput: (...args) => { record("connection beforeinput")(...args); return 5; },
        InputHelper_OnConnectionInput: record("connection input"),
    });
    const input = textarea();
    const host = Object.assign(new EventTarget(), {
        querySelector: (selector) => selector === ":scope > .ferroui-text-input-element" ? input : null,
    });
    t.after(InputHelper.subscribeTextEvents(host, 31));
    InputHelper.setTextInputState(Object.assign(input, { style: {}, scrollWidth: 40 }), "ab", 2, 2, 7);
    assert.equal(input.style.width, "40px");

    input.dispatchEvent(event("beforeinput", { inputType: "insertText", isComposing: false }));
    input.value = "abc";
    input.selectionStart = input.selectionEnd = 3;
    input.dispatchEvent(event("input", { inputType: "insertText", data: "c", isComposing: false }));
    host.dispatchEvent(insertion("legacy"));
    assert.deepEqual(calls, [
        ["connection beforeinput", 31, 7, "insertText", "ab", 2, 2],
        ["connection input", 31, 7, 5, "insertText", "abc", 3, 3],
        ["text", 31, "legacy"],
    ]);
    assert.equal(InputHelper.getTextInputElement({ parentElement: host }), input);
});
