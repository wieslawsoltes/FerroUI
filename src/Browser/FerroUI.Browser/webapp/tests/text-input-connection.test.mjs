// Dispatch real events through the production TypeScript subscription. The
// textarea double implements DOM UTF-16 selection and API newline behavior;
// transaction acceptance, formatting and undo are tested by the Rust adapter.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

const source = await readFile(new URL("../modules/ferroui/textInput.ts", import.meta.url), "utf8");
const { code } = await transform(source, { loader: "ts", format: "esm", target: "es2020" });
const { setTextInputState, subscribeTextInput } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

class Textarea extends EventTarget {
    _value = "";
    selectionStart = 0;
    selectionEnd = 0;
    selectionDirection = "none";
    valueWrites = 0;
    selectionWrites = 0;
    get value() { return this._value; }
    set value(value) {
        this._value = value.replace(/\r\n?/g, "\n");
        this.selectionStart = this.selectionEnd = this._value.length;
        this.selectionDirection = "none";
        this.valueWrites++;
    }
    setSelectionRange(start, end, direction) {
        this.selectionStart = Math.max(0, Math.min(start, this.value.length));
        this.selectionEnd = Math.max(this.selectionStart, Math.min(end, this.value.length));
        this.selectionDirection = direction;
        this.selectionWrites++;
    }
    nativeValue(value, anchor, active) {
        this.value = value;
        this.setSelectionRange(Math.min(anchor, active), Math.max(anchor, active),
            anchor === active ? "none" : anchor > active ? "backward" : "forward");
    }
    focus() { assert.fail("mirroring must not move focus"); }
}

function dispatch(input, type, fields = {}, cancelable = false) {
    const event = new Event(type, { cancelable });
    Object.assign(event, fields);
    input.dispatchEvent(event);
    return event;
}

function fixture(value = "", anchor = value.length, active = anchor) {
    const input = new Textarea();
    const calls = [];
    const callbacks = {
        beforeInput: (...args) => { calls.push(["before", ...args]); return 17; },
        input: (...args) => { calls.push(["input", ...args]); },
        compositionStart: (...args) => { calls.push(["start", ...args]); return 31; },
        compositionUpdate: (...args) => { calls.push(["update", ...args]); },
        compositionEnd: (...args) => { calls.push(["end", ...args]); },
        ownsPaste: () => false
    };
    setTextInputState(input, value, anchor, active, 1);
    const dispose = subscribeTextInput(input, callbacks);
    return { input, calls, callbacks, dispose };
}

test("canonical LF and directed UTF-16 selection are mirrored idempotently", () => {
    const { input } = fixture("A😀\nB", 4, 1);
    assert.equal(input.value, "A😀\nB");
    assert.deepEqual([input.selectionStart, input.selectionEnd, input.selectionDirection], [1, 4, "backward"]);
    const writes = [input.valueWrites, input.selectionWrites];
    setTextInputState(input, "A😀\nB", 4, 1, 2);
    assert.deepEqual([input.valueWrites, input.selectionWrites], writes);
});

test("mobile replacement uses actual values and the captured beforeinput token", () => {
    const { input, calls, callbacks } = fixture("teh");
    callbacks.input = (...args) => {
        calls.push(["input", ...args]);
        setTextInputState(input, "the", 3, 3, 2);
    };
    dispatch(input, "beforeinput", { inputType: "insertReplacementText", isComposing: false }, true);
    input.nativeValue("the", 3, 3);
    dispatch(input, "input", { inputType: "insertReplacementText", isComposing: false });
    assert.deepEqual(calls, [
        ["before", 1, "insertReplacementText", "teh", 3, 3],
        ["input", 1, 17, "insertReplacementText", "the", 3, 3]
    ]);
    assert.equal(input.value, "the");
});

test("input without beforeinput retains the last reflected revision and UTF-16 offsets", () => {
    const { input, calls, callbacks } = fixture();
    callbacks.input = (...args) => {
        calls.push(["input", ...args]);
        setTextInputState(input, "😀\n", 3, 3, 2);
    };
    input.nativeValue("😀\r\n", 3, 3);
    dispatch(input, "input", { inputType: "insertText", isComposing: false });
    assert.deepEqual(calls, [["input", 1, 0, "insertText", "😀\n", 3, 3]]);
    assert.equal(input.value, "😀\n");
});

test("an empty replacement is delivered as a real deletion", () => {
    const { input, calls, callbacks } = fixture("a😀b", 4, 0);
    callbacks.input = (...args) => {
        calls.push(["input", ...args]);
        setTextInputState(input, "", 0, 0, 2);
    };
    dispatch(input, "beforeinput", { inputType: "deleteContentBackward", isComposing: false }, true);
    input.nativeValue("", 0, 0);
    dispatch(input, "input", { inputType: "deleteContentBackward", isComposing: false });
    assert.deepEqual(calls, [
        ["before", 1, "deleteContentBackward", "a😀b", 4, 0],
        ["input", 1, 17, "deleteContentBackward", "", 0, 0]
    ]);
});

test("composition owns native input and its trailing input cannot commit twice", () => {
    const { input, calls, callbacks } = fixture("AB", 1, 1);
    callbacks.compositionEnd = (...args) => {
        calls.push(["end", ...args]);
        setTextInputState(input, "AかなB", 3, 3, 2);
    };
    dispatch(input, "compositionstart", { data: "" });
    dispatch(input, "compositionupdate", { data: "かな" });
    dispatch(input, "beforeinput", { inputType: "insertCompositionText", isComposing: true });
    input.nativeValue("AかなB", 3, 3);
    dispatch(input, "input", { inputType: "insertCompositionText", isComposing: true });
    assert.equal(input.value, "AかなB", "active native preview must not be overwritten");
    dispatch(input, "compositionend", { data: "かな" });
    input.nativeValue("AかなかなB", 5, 5);
    dispatch(input, "input", { inputType: "insertFromComposition", isComposing: false });
    assert.deepEqual(calls, [
        ["start", 1, "AB", 1, 1], ["update", 31, "かな"], ["end", 31, "かな", "AかなB", 3, 3]
    ]);
    assert.equal(input.value, "AかなB");
});

test("a late composition end retains the old owner token after focus synchronization", () => {
    const { input, calls } = fixture("first");
    dispatch(input, "compositionstart", { data: "" });
    setTextInputState(input, "second", 6, 6, 2);
    input.nativeValue("second古", 7, 7);
    dispatch(input, "compositionupdate", { data: "古" });
    dispatch(input, "compositionend", { data: "古" });
    assert.deepEqual(calls, [
        ["start", 1, "first", 5, 5], ["update", 31, "古"], ["end", 31, "古", "second古", 7, 7]
    ]);
    assert.equal(input.value, "second", "the authoritative new client survives rejected stale input");
});

test("rejected uncancelable beforeinput cannot use the no-beforeinput fallback", () => {
    const { input, calls, callbacks } = fixture("before");
    callbacks.beforeInput = () => 0;
    dispatch(input, "beforeinput", { inputType: "insertReplacementText", isComposing: false });
    input.nativeValue("after", 5, 5);
    dispatch(input, "input", { inputType: "insertReplacementText", isComposing: false });
    assert.deepEqual(calls, [["input", 0, 0, "insertReplacementText", "after", 5, 5]]);
    assert.equal(input.value, "before");
});

test("focus reentry in beforeinput cancels the old edit and preserves the newer snapshot", () => {
    const { input, calls, callbacks } = fixture("first");
    callbacks.beforeInput = () => {
        setTextInputState(input, "second", 6, 6, 2);
        return 17;
    };
    const event = dispatch(input, "beforeinput", { inputType: "insertText", isComposing: false }, true);
    assert.equal(event.defaultPrevented, true);
    input.nativeValue("firstx", 6, 6);
    dispatch(input, "input", { inputType: "insertText", isComposing: false });
    assert.deepEqual(calls, [["input", 0, 0, "insertText", "firstx", 6, 6]]);
    assert.equal(input.value, "second");
});

test("a mirror after beforeinput keeps its old edit rejected instead of falling back to the new owner", () => {
    const { input, calls } = fixture("same", 4, 4);
    dispatch(input, "beforeinput", { inputType: "insertText", isComposing: false }, true);
    // Focus can move to an editor with identical text between the two events.
    // Its authoritative revision still supersedes the captured transaction.
    setTextInputState(input, "same", 4, 4, 2);
    input.nativeValue("samex", 5, 5);
    dispatch(input, "input", { inputType: "insertText", isComposing: false });
    assert.deepEqual(calls, [
        ["before", 1, "insertText", "same", 4, 4],
        ["input", 0, 0, "insertText", "samex", 5, 5]
    ]);
    assert.equal(input.value, "same");
});

test("already canceled beforeinput never becomes an untracked native replacement", () => {
    const { input, calls } = fixture("before");
    const event = new Event("beforeinput", { cancelable: true });
    Object.assign(event, { inputType: "insertReplacementText", isComposing: false });
    event.preventDefault();
    input.dispatchEvent(event);
    input.nativeValue("after", 5, 5);
    dispatch(input, "input", { inputType: "insertReplacementText", isComposing: false });
    assert.deepEqual(calls, [["input", 0, 0, "insertReplacementText", "after", 5, 5]]);
    assert.equal(input.value, "before");
});

test("keyboard-handled edits and subsequent unchanged input preserve one committed value", () => {
    const { input, calls, callbacks } = fixture("a");
    let edits = 0;
    let canonical = "a";
    input.addEventListener("keydown", event => {
        canonical = "ax";
        edits++;
        setTextInputState(input, canonical, 2, 2, 2);
        event.preventDefault();
    });
    callbacks.input = (revision, token, type, value) => {
        calls.push(["input", revision, token, type, value]);
        if (value !== canonical) edits++;
    };
    const key = dispatch(input, "keydown", { key: "x" }, true);
    assert.equal(key.defaultPrevented, true);
    dispatch(input, "input", { inputType: "insertText", isComposing: false });
    assert.equal(edits, 1);
    assert.equal(input.value, "ax");
    assert.deepEqual(calls, [["input", 2, 0, "insertText", "ax"]]);
});

test("framework clipboard reads suppress native insertion while ordinary paste stays available", () => {
    const { input, callbacks } = fixture("a");
    assert.equal(dispatch(input, "paste", {}, true).defaultPrevented, false);
    callbacks.ownsPaste = () => true;
    assert.equal(dispatch(input, "paste", {}, true).defaultPrevented, true);
});

test("unsubscribing removes every input/composition listener and independent targets stay independent", () => {
    const { input, calls, dispose } = fixture("a");
    const other = new Textarea();
    dispatch(other, "input", { inputType: "insertText", isComposing: false });
    dispose();
    dispatch(input, "beforeinput", { inputType: "insertText", isComposing: false }, true);
    dispatch(input, "input", { inputType: "insertText", isComposing: false });
    dispatch(input, "compositionstart", { data: "" });
    dispatch(input, "compositionupdate", { data: "x" });
    dispatch(input, "compositionend", { data: "x" });
    assert.deepEqual(calls, []);
});
