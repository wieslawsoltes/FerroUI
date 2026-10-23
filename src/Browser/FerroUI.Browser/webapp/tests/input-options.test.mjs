// Exercise the actual TypeScript attribute bridge without replacing a composing
// text input. Keyboard availability itself is decided by the browser/platform.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

const source = await readFile(new URL("../modules/ferroui/inputOptions.ts", import.meta.url), "utf8");
const { code } = await transform(source, { loader: "ts", format: "esm", target: "es2020" });
const { applyTextInputOptions } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

function composingInput() {
    const attributes = new Map([["type", "text"]]);
    const writes = [];
    const input = {
        getAttribute(name) { return attributes.get(name) ?? null; },
        setAttribute(name, value) {
            assert.notEqual(name, "type", "scope must not replace the input type");
            assert.notEqual(name, "value", "scope must not replace composing text");
            writes.push([name, value]);
            attributes.set(name, value);
        },
        removeAttribute(name) { writes.push([name, null]); attributes.delete(name); },
        focus() { assert.fail("scope must not move focus"); },
        blur() { assert.fail("scope must not move focus"); },
        setSelectionRange() { assert.fail("scope must not move the UTF-16 selection"); }
    };
    for (const [name, value] of Object.entries({
        type: "text", value: "A😀e\u0301かなZ", selectionStart: 1, selectionEnd: 3,
        selectionDirection: "backward", isConnected: true
    })) {
        Object.defineProperty(input, name, {
            get() { return value; },
            set() { assert.fail(`scope must not assign ${name}`); }
        });
    }
    return { input, attributes, writes };
}

test("keyboard and action hints preserve the composing input and its UTF-16 selection", () => {
    const { input, attributes } = composingInput();
    applyTextInputOptions(input, "numeric", "next", "none", false, false, "pl-PL");
    assert.deepEqual(Object.fromEntries(attributes), {
        type: "text", inputmode: "numeric", enterkeyhint: "next", autocapitalize: "none",
        spellcheck: "false", autocorrect: "off", writingsuggestions: "false",
        autocomplete: "off", lang: "pl-PL"
    });
    assert.equal(input.value, "A😀e\u0301かなZ");
    assert.equal(input.selectionStart, 1);
    assert.equal(input.selectionEnd, 3);
    assert.equal(input.selectionDirection, "backward");
});

test("clearing a scope and locale resets stale hints on the same input", () => {
    const { input, attributes } = composingInput();
    applyTextInputOptions(input, "tel", "next", "none", false, true, "ja-JP");
    applyTextInputOptions(input, "text", "enter", "sentences", true, false, "");
    assert.equal(attributes.get("inputmode"), "text");
    assert.equal(attributes.get("enterkeyhint"), "enter");
    assert.equal(attributes.get("autocapitalize"), "sentences");
    assert.equal(attributes.get("spellcheck"), "true");
    assert.equal(attributes.get("autocorrect"), "on");
    assert.equal(attributes.get("writingsuggestions"), "true");
    assert.equal(attributes.has("lang"), false);
});

test("sensitive hints suppress browser suggestions even if requested by a caller", () => {
    const { input, attributes } = composingInput();
    applyTextInputOptions(input, "numeric", "done", "none", true, true, "en-US");
    assert.equal(attributes.get("spellcheck"), "false");
    assert.equal(attributes.get("autocorrect"), "off");
    assert.equal(attributes.get("writingsuggestions"), "false");
    assert.equal(attributes.get("autocomplete"), "off");
    assert.equal(attributes.get("type"), "text");
});

test("unchanged attributes do not generate repeated DOM mutations", () => {
    const { input, writes } = composingInput();
    applyTextInputOptions(input, "email", "send", "none", false, false, "");
    writes.length = 0;
    applyTextInputOptions(input, "email", "send", "none", false, false, "");
    assert.deepEqual(writes, []);
    applyTextInputOptions(input, "url", "go", "none", false, false, "");
    assert.deepEqual(writes, [["inputmode", "url"], ["enterkeyhint", "go"]]);
});
