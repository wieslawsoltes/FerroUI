// Exercise the actual TypeScript attribute bridge without replacing a composing
// text input. Keyboard availability itself is decided by the browser/platform.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

const source = await readFile(new URL("../modules/ferroui/inputOptions.ts", import.meta.url), "utf8");
const { code } = await transform(source, { loader: "ts", format: "esm", target: "es2020" });
const {
    applyMaskedInput, applyTextInputOptions, isUnforwardedMaskedInsertion, rememberMaskedText, restoreMaskedText
} = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

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

test("a masked field uses a password input without spelling or autofill services", () => {
    const attributes = new Map([["spellcheck", "false"], ["autocapitalize", "none"]]);
    const input = {
        type: "text",
        getAttribute(name) { return attributes.get(name) ?? null; },
        setAttribute(name, value) { attributes.set(name, value); },
        removeAttribute(name) { attributes.delete(name); }
    };

    applyMaskedInput(input, true);
    assert.equal(input.type, "password");
    assert.equal(attributes.get("spellcheck"), "false");
    assert.equal(attributes.get("autocorrect"), "off");
    assert.equal(attributes.get("autocapitalize"), "off");
    assert.equal(attributes.get("autocomplete"), "new-password");

    applyMaskedInput(input, false);
    assert.equal(input.type, "text");
    assert.equal(attributes.get("spellcheck"), "false");
    assert.equal(attributes.has("autocorrect"), false);
    assert.equal(attributes.get("autocapitalize"), "none");
    assert.equal(attributes.has("autocomplete"), false);
});

test("a masked field refuses the insertions that are not forwarded to the real text", () => {
    for (const inputType of ["insertFromPaste", "insertFromDrop", "insertReplacementText", "insertFromYank"]) {
        assert.equal(isUnforwardedMaskedInsertion("password", inputType), true, inputType);
        assert.equal(isUnforwardedMaskedInsertion("text", inputType), false, inputType);
    }
    for (const inputType of ["insertText", "insertCompositionText", "deleteContentBackward"]) {
        assert.equal(isUnforwardedMaskedInsertion("password", inputType), false, inputType);
    }
});

test("composed text does not stay in a masked element", () => {
    const selections = [];
    const input = { type: "password", value: "", setSelectionRange(start, end) { selections.push([start, end]); } };

    rememberMaskedText(input, "\u2022\u2022", 1, 2);
    input.value = "\u2022composed\u2022";
    restoreMaskedText(input);
    assert.equal(input.value, "\u2022\u2022");
    assert.deepEqual(selections, [[1, 2]]);

    // Nothing to do when the element already holds the mask.
    restoreMaskedText(input);
    assert.equal(selections.length, 1);

    // An element the framework never wrote a mask into holds nothing.
    const fresh = { type: "password", value: "composed", setSelectionRange() {} };
    restoreMaskedText(fresh);
    assert.equal(fresh.value, "");

    // A text input is not touched.
    const plain = { type: "text", value: "visible", setSelectionRange() { assert.fail("not a masked input"); } };
    rememberMaskedText(plain, "visible", 0, 0);
    plain.value = "edited";
    restoreMaskedText(plain);
    assert.equal(plain.value, "edited");
});
