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

function event(type, fields = {}) {
    return Object.assign(new Event(type, { cancelable: true }), fields);
}
function insertion(data, fields = {}) {
    return event("input", { inputType: "insertText", data, isComposing: false, ...fields });
}
function setup(t, { handled = false } = {}) {
    const calls = [];
    const record = (name) => (...args) => { calls.push([name, ...args]); };
    FerroExports.attach({
        InputHelper_OnTextInput: record("text"),
        InputHelper_OnBeforeInput: record("beforeinput"),
        InputHelper_OnCompositionStart: record("compositionstart"),
        InputHelper_OnCompositionUpdate: record("compositionupdate"),
        InputHelper_OnCompositionEnd: record("compositionend"),
        InputHelper_OnKeyDown: (...args) => { record("keydown")(...args); return handled; },
        InputHelper_OnKeyUp: (...args) => { record("keyup")(...args); return handled; },
    });
    const element = Object.assign(new EventTarget(), { value: "", selectionStart: 0, selectionEnd: 0 });
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
