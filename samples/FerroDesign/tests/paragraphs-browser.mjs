import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { inlineInputState, startInputObservation, stopInputObservation } from "./input-observations.mjs";
import { captureWithinDeadline } from "./isolated-scenario.mjs";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Native TextBox selection, ComboBoxes, one Apply click per edit, actual
// presenter geometry and downloaded native PNG. No command mutates paragraph
// state or injects a caret/selection into the application.
export async function paragraphRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const click = name => clickNative(page, name, "ModalViewport");
    const choose = (name, index) => chooseNative(page, name, { index, scrollContainer: "ModalViewport" });
    const field = async (name, value) => {
        await click(name);
        assert.equal((await state(page)).chrome[name].focused, true, name + " owns native focus");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(String(value));
    };
    const numeric = async (name, value) => {
        await choose(name + "Mode", 1);
        await field(name, value);
    };
    const typography = async name => {
        await page.waitForFunction(name => {
            const control = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome[name];
            return control?.visible && control.measureValid && control.arrangeValid && control.typography?.lineCount > 0;
        }, name, { timeout: 10000 });
        return (await state(page)).chrome[name].typography;
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let view = await state(page);
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        assert.deepEqual(await documentState(page), before);
        view = await state(page);
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        assert.deepEqual(await documentState(page), after);
    };

    const value = "A🦀 first\nשלום second\nLast";
    const firstEnd = Buffer.byteLength("A🦀 first\n");
    const secondEnd = Buffer.byteLength("A🦀 first\nשלום second\n");
    const text = fixtureNode(reference, 2, {
        kind: "text", name: "Independent paragraphs", x: 40, y: 40, width: 440, height: 200,
        text: value, text_runs: [], text_paragraphs: [], font_family: "Inter", font_size: 20,
        font_weight: 400, italic: false, letter_spacing: 0, line_height: 1.2,
        text_direction: "auto", text_auto_resize: "fixed", text_vertical_align: "top",
        text_decoration: "none", text_align: "left", font_features: [],
        fills: [{ paint: { type: "solid", r: 20, g: 20, b: 20, a: 255 }, opacity: 1, visible: true }],
        strokes: [], effects: [],
    });
    await importFixture(page, isolatedDocument(reference, [text], "Native independent paragraphs"));
    await clickWorld(page, 65, 60);
    assert.deepEqual((await state(page)).selection, [2]);
    let beforeOpen = await state(page);
    await clickNative(page, "RichTextButton", "InspectorViewport");
    await typography("RichTextSelection");
    await waitForNativeScene(page, beforeOpen.sceneFrames, "Rich text dialog publication");
    beforeOpen = await state(page);
    await click("RichTextParagraphs");
    await typography("ParagraphSelection");
    await waitForNativeScene(page, beforeOpen.sceneFrames, "Paragraph dialog publication");
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await click("ParagraphSelection");
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("Shift+End");
    let layout = await typography("ParagraphSelection");
    assert.deepEqual([Math.min(layout.selectionStart, layout.selectionEnd), Math.max(layout.selectionStart, layout.selectionEnd)], [0, 9]);
    await choose("ParagraphAlignment", 2);
    await choose("ParagraphDirection", 3);
    await numeric("ParagraphLineHeight", 1.5);
    await numeric("ParagraphLetterSpacing", 0.5);
    await numeric("ParagraphBefore", 5);
    await numeric("ParagraphAfter", 20);
    await numeric("ParagraphIndent", 14);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.[0]?.indent === 14);
    const first = { start: 0, end: firstEnd, alignment: "center", direction: "auto",
        line_height: 1.5, letter_spacing: 0.5, space_before: 5, space_after: 20, indent: 14 };
    let authored = await documentState(page);
    assert.deepEqual(authored.nodes[2].text_paragraphs, [first]);
    assert.equal(authored.nodes[2].text, value);
    assert.deepEqual(authored.nodes[2].text_runs, []);
    assert.equal((await state(page)).historyLength, depth + 1);
    layout = await typography("ParagraphSelection");
    assert.equal(layout.lineCount, 3);
    assert.deepEqual(layout.lines.map(line => [line.x, line.y, line.height, line.direction]), [
        [14, 5, 30, "LeftToRight"], [0, 55, 24, "RightToLeft"], [0, 79, 24, "LeftToRight"],
    ]);
    assert.equal(layout.layoutHeight, 103);
    check("Native UTF-16 selection across a non-BMP character authors one complete UTF-8 paragraph with independent alignment, Auto direction, line height, letter spacing, before/after spacing and first-line indentation in the actual TextPresenter");

    await click("ParagraphSelection");
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("ArrowDown");
    layout = await typography("ParagraphSelection");
    assert.equal(layout.selectionStart, layout.selectionEnd);
    assert.ok(layout.caretIndex >= 10 && layout.caretIndex < 22, "Native vertical navigation reaches the second paragraph across its spacing");
    assert.equal(layout.caret.y, 55, "The vertical caret uses the actual second-paragraph origin");
    // Make the authored target independent of the physical X that vertical
    // navigation preserved across mixed-direction text.
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowRight");
    assert.equal((await typography("ParagraphSelection")).caretIndex, 10);
    await choose("ParagraphAlignment", 3);
    await choose("ParagraphDirection", 2);
    await numeric("ParagraphLineHeight", 2);
    await choose("ParagraphLetterSpacingMode", 0);
    await numeric("ParagraphBefore", 7);
    await numeric("ParagraphAfter", 11);
    await numeric("ParagraphIndent", 10);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.length === 2);
    authored = await documentState(page);
    assert.deepEqual(authored.nodes[2].text_paragraphs, [first, {
        start: firstEnd, end: secondEnd, alignment: "right", direction: "rtl", line_height: 2,
        space_before: 7, space_after: 11, indent: 10,
    }]);
    assert.equal((await state(page)).historyLength, depth + 2);
    layout = await typography("ParagraphSelection");
    assert.deepEqual(layout.lines.map(line => [line.x, line.y, line.height]), [[14,5,30],[0,62,40],[0,113,24]]);
    await field("ParagraphIndent", -1);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ParagraphError?.visible,
        undefined, { timeout: 10000 });
    assert.deepEqual(await documentState(page), authored, "Invalid paragraph metrics reject the entire patch");
    assert.equal((await state(page)).historyLength, depth + 2);
    await field("ParagraphIndent", 10);
    await clickNative(page, "ParagraphApply", "ModalActions");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ParagraphError?.visible,
        undefined, { timeout: 10000 });
    assert.equal((await state(page)).historyLength, depth + 2, "An unchanged native Apply adds no history");
    await captureNativeScreenshot(mode + "-native-paragraph-authoring", { controls: ["ParagraphSelection", "ParagraphApply"] });
    check("A collapsed native caret styles only its paragraph; RTL alignment and spacing move the real layout, invalid metrics fail atomically and repeated Apply remains a no-op");

    await clickNative(page, "ParagraphInheritAll", "ModalActions");
    await waitDocument(doc => doc.nodes[2].text_paragraphs?.length === 1);
    const inherited = await documentState(page);
    assert.deepEqual(inherited.nodes[2].text_paragraphs, [first]);
    await clickNative(page, "ParagraphDone", "ModalActions");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalBackdrop.visible,
        undefined, { timeout: 10000 });
    await history(authored, inherited, depth + 2, "Inherit second paragraph");
    // Text-layer PNG exports retain conservative glyph-overhang paint bounds.
    // Use an explicitly clipped artboard to test exact paragraph bands in the
    // intended 440x200 viewport instead of treating a selection box as a crop.
    // All paragraph values come unchanged from the native authored document;
    // this separate starting fixture does not replace the history checks above.
    const exportDocument = structuredClone(inherited);
    exportDocument.nodes[2] = { ...exportDocument.nodes[2], parent: 3, x: 0, y: 0 };
    exportDocument.nodes[3] = fixtureNode(reference, 3, {
        kind: "frame", name: "Paragraph export artboard", x: 40, y: 40, width: 440, height: 200,
        children: [2], clip_content: true, corner_radius: 0, corner_radii: null,
        fills: [], strokes: [], effects: [],
    });
    exportDocument.pages[0].children = [3];
    exportDocument.next_id = Math.max(exportDocument.next_id, 4);
    await importFixture(page, exportDocument);
    await clickWorld(page, 45, 45);
    await page.keyboard.press("Control+a");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.selection.length === 1 && view.selection[0] === 3;
    }, undefined, { timeout: 10000 });
    const exportBefore = await documentState(page);
    const [download] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await download.failure(), null);
    const pngPath = resolve(output, mode + "-paragraph-artwork.png");
    await download.saveAs(pngPath);
    const png = PNG.sync.read(await readFile(pngPath));
    assert.deepEqual([png.width, png.height], [440, 200]);
    const ink = (from, to) => {
        let count = 0;
        for (let y = from; y < to; y++) for (let x = 0; x < png.width; x++) count += Number(png.data[(y * png.width + x) * 4 + 3] > 16);
        return count;
    };
    assert.ok(ink(5, 35) > 40 && ink(55, 79) > 40 && ink(79, 103) > 20, "Actual native PNG contains all three independently positioned paragraphs");
    assert.equal(ink(40, 50), 0, "Authored paragraph spacing leaves a real transparent gap in exported artwork");
    assert.deepEqual(await documentState(page), exportBefore, "Native export preserves the exact clipped paragraph fixture");
    await importFixture(page, inherited);
    assert.deepEqual(await documentState(page), inherited, "Restore the exact authored document before native inline editing");
    check("Paragraph Inherit has exact one-step undo/redo and preserves the first paragraph; its native clipped-artboard PNG has exact dimensions, three text bands and the authored transparent spacing gap");

    const enterInline = async () => {
        await clickWorld(page, 65, 60);
        assert.equal((await state(page)).chrome.CanvasHost.focusWithin, true);
        const before = await state(page);
        await page.keyboard.press("Enter");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.editingText === 2 && view.chrome.InlineTextBox?.focused && view.chrome.InlineTextBox?.visible;
        }, undefined, { timeout: 10000 });
        await waitForNativeFrame(page, before.renderedFrames, "Native paragraph inline entry");
        await waitForNativeScene(page, before.sceneFrames, "Native paragraph inline entry");
    };
    const splitInputs = [];
    const splitFirst = async () => {
        const beforeNavigation = await state(page);
        const input = { entry: splitInputs.length + 1, expectedCaretIndex: 3,
            keys: ["Control+Home", "ArrowRight", "ArrowRight"],
            before: inlineInputState(beforeNavigation),
            beforeTypography: beforeNavigation.chrome.InlineTextBox.typography };
        splitInputs.push(input);
        await startInputObservation(page, ["keydown", "keyup"]);
        let navigationError;
        try {
            // Keep the actual input sequence and the immediate native caret
            // assertion. Passive evidence distinguishes delivery from layout
            // when the second entry after cancellation fails this check.
            await page.keyboard.press("Control+Home");
            await page.keyboard.press("ArrowRight");
            await page.keyboard.press("ArrowRight");
            input.typography = await typography("InlineTextBox");
            assert.equal(input.typography.caretIndex, 3);
        } catch (error) {
            input.navigationError = error?.stack || String(error);
            navigationError = error;
        }
        try {
            await captureWithinDeadline(async () => {
                input.observation = await stopInputObservation(page);
                const after = await state(page);
                input.after = inlineInputState(after);
                input.actualText = after.chrome.InlineTextBox.text;
                await writeFile(resolve(output, mode + "-native-paragraph-split-input.json"),
                    JSON.stringify(splitInputs, null, 2));
            }, 5000);
        } catch (error) {
            input.captureError = error?.stack || String(error);
            navigationError ??= error;
        }
        if (navigationError) throw navigationError;
        await page.keyboard.press("Enter");
        await waitDocument(doc => doc.nodes[2].text === "A🦀\n first\nשלום second\nLast");
        const split = await documentState(page);
        assert.deepEqual(split.nodes[2].text_paragraphs, [{ ...first, end: 6 }, { ...first, start: 6, end: firstEnd + 1 }]);
        const native = await typography("InlineTextBox");
        assert.equal(native.lineCount, 4);
        assert.deepEqual(native.lines.slice(0,2).map(line => [line.x, line.y, line.height]), [[14,5,30],[14,60,30]]);
        return split;
    };
    await enterInline();
    await splitFirst();
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.deepEqual(await documentState(page), inherited);
    const splitDepth = (await state(page)).historyLength;
    await enterInline();
    const split = await splitFirst();
    await captureNativeScreenshot(mode + "-native-paragraph-inline", { controls: ["InlineTextBox"] });
    await page.keyboard.press("Control+Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    await history(inherited, split, splitDepth, "Native paragraph split");
    await importFixture(page, split);
    assert.deepEqual(await documentState(page), split);
    assert.equal(before.nodes[2].text, split.nodes[2].text.replace("🦀\n", "🦀"));
    check("Real native Unicode editing splits paragraph styles with correct UTF-8 remapping and shared line/caret offsets; cancel restores the original, commit is one exact undo/redo step and JSON round-trip preserves every paragraph field");

    // Identical text makes a string-diff guess ambiguous. Give each repeated
    // paragraph distinct character and paragraph styling, then edit at an
    // exact native caret/selection and exercise the TextBox's own undo stack.
    const repeatedUnit = "A🦀\n", unitBytes = Buffer.byteLength(repeatedUnit);
    const repeated = fixtureNode(reference, 2, {
        ...text, name: "Repeated styled paragraphs", text: repeatedUnit.repeat(3),
        text_runs: [0, 1, 2].map(index => ({
            start: index * unitBytes, end: (index + 1) * unitBytes,
            font_family: null, font_size: 20 + index * 2, font_weight: 400 + index * 200,
            color: [{ r: 190, g: 30, b: 50, a: 255 }, { r: 20, g: 90, b: 190, a: 255 },
                { r: 25, g: 140, b: 60, a: 255 }][index],
            italic: index === 1, decoration: index === 2 ? "strikethrough" : "none",
        })),
        text_paragraphs: [0, 1, 2, 3].map(index => ({
            start: index * unitBytes, end: Math.min(index + 1, 3) * unitBytes,
            alignment: "left", direction: "ltr", indent: 10 + index * 10,
            space_before: index, space_after: 2 + index, line_height: 1.2 + index * 0.1,
        })),
    });
    const repeatedFixture = isolatedDocument(reference, [repeated], "Exact native rich text history");
    const rich = node => ({ text: node.text, text_runs: node.text_runs, text_paragraphs: node.text_paragraphs });
    const shift = (range, delta) => ({ ...range, start: range.start + delta, end: range.end + delta });
    const assertRich = async (expected, depth, label) => {
        await waitDocument((doc, expected) => doc.nodes[2].text === expected.text &&
            JSON.stringify(doc.nodes[2].text_runs) === JSON.stringify(expected.text_runs) &&
            JSON.stringify(doc.nodes[2].text_paragraphs) === JSON.stringify(expected.text_paragraphs), expected);
        assert.deepEqual(rich((await documentState(page)).nodes[2]), expected, label + " restores both exact UTF-8 style collections");
        const view = await state(page);
        assert.equal(view.editingText, 2, label + " stays inside the native TextBox");
        assert.equal(view.chrome.InlineTextBox.focused, true, label + " retains native keyboard focus");
        assert.equal(view.chrome.InlineTextBox.text, expected.text);
        assert.equal(view.historyLength, depth, label + " does not commit the open document transaction");
        const native = await typography("InlineTextBox");
        assert.equal(native.preedit, null);
        assert.deepEqual(native.lines.map(line => line.x), expected.text_paragraphs.map(style => style.indent),
            label + " restores the actual independently indented TextPresenter lines");
    };
    const nativeHistory = async (key, expected, depth, label) => {
        const before = await state(page);
        await page.keyboard.press(key);
        await assertRich(expected, depth, label);
        await waitForNativeFrame(page, before.renderedFrames, label);
        await waitForNativeScene(page, before.sceneFrames, label);
    };
    const selectFirstParagraph = async () => {
        await page.keyboard.press("Control+Home");
        for (let index = 0; index < 3; index++) await page.keyboard.press("Shift+ArrowRight");
        const native = await typography("InlineTextBox");
        assert.deepEqual([Math.min(native.selectionStart, native.selectionEnd), Math.max(native.selectionStart, native.selectionEnd)],
            [0, repeatedUnit.length], "The actual native selection includes one non-BMP paragraph and its separator");
    };

    await importFixture(page, repeatedFixture);
    const repeatedDocument = await documentState(page), originalRich = rich(repeatedDocument.nodes[2]);
    const insertedRich = {
        text: repeatedUnit.repeat(4),
        text_runs: [{ ...originalRich.text_runs[0], end: unitBytes * 2 },
            ...originalRich.text_runs.slice(1).map(run => shift(run, unitBytes))],
        text_paragraphs: [originalRich.text_paragraphs[0],
            ...originalRich.text_paragraphs.map(style => shift(style, unitBytes))],
    };
    let repeatedDepth = (await state(page)).historyLength;
    await enterInline();
    await page.keyboard.press("Control+Home");
    assert.equal((await typography("InlineTextBox")).caretIndex, 0);
    // CDP insertText targets the bridge's single-line DOM input; use the
    // actual multiline clipboard path here. Native paste applies the complete
    // text/plain payload as one edit, preserving the one-undo contract below.
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"], {
        origin: new URL(page.url()).origin,
    });
    const clipboard = await page.evaluate(async value => {
        await navigator.clipboard.writeText(value);
        const items = await navigator.clipboard.read();
        return Promise.all(items.map(async item => ({
            types: [...item.types],
            text: item.types.includes("text/plain") ? await (await item.getType("text/plain")).text() : null,
        })));
    }, repeatedUnit);
    assert.deepEqual(clipboard.filter(item => item.types.includes("text/plain")).map(item => item.text),
        [repeatedUnit], "The actual clipboard preserves the complete non-BMP paragraph and newline");
    const beforePaste = await state(page);
    await startInputObservation(page, ["keydown", "keyup", "paste", "beforeinput", "input"]);
    let pasteError;
    try {
        await page.keyboard.press("Control+v");
        await assertRich(insertedRich, repeatedDepth, "Repeated paragraph paste at native caret zero");
    } catch (error) {
        pasteError = error;
    }
    try {
        const evidence = await captureWithinDeadline(async () => {
            const input = await stopInputObservation(page);
            const after = await state(page);
            const evidence = {
                path: "trusted Ctrl+V through the native TextBox and browser Clipboard API",
                expectedClipboardText: repeatedUnit, clipboard,
                before: inlineInputState(beforePaste), after: inlineInputState(after),
                actualText: after.chrome.InlineTextBox.text, input,
            };
            await writeFile(resolve(output, mode + "-native-paragraph-paste-input.json"), JSON.stringify(evidence, null, 2));
            return evidence;
        });
        assert.equal(evidence.input.droppedEvents, 0, "Native paste input evidence is complete");
        const pasteKeys = evidence.input.events.filter(event => event.phase === "capture" &&
            event.type === "keydown" && event.code === "KeyV" && event.ctrlKey);
        assert.equal(pasteKeys.length, 1, "One actual paste shortcut initiates the entire paragraph insertion");
        assert.equal(pasteKeys[0].trusted, true);
        // Chrome's async Clipboard API route handles the key before its DOM
        // default, so a synthetic paste/InputEvent is neither needed nor sent.
    } catch (error) {
        if (!pasteError) pasteError = error;
        else console.error(mode + ": paragraph paste evidence failed after the primary error: " + String(error));
    }
    if (pasteError) throw pasteError;
    await nativeHistory("Control+z", originalRich, repeatedDepth, "Native insertion undo");
    await nativeHistory("Control+Shift+z", insertedRich, repeatedDepth, "Native insertion redo");
    const insertedDocument = await documentState(page);
    await page.keyboard.press("Control+Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null,
        undefined, { timeout: 10000 });
    await history(repeatedDocument, insertedDocument, repeatedDepth, "Repeated paragraph edit session");
    check("A real native clipboard paste at the start of identical Unicode paragraphs inherits the first paragraph only; native Ctrl+Z/Ctrl+Shift+Z restore exact character and paragraph ranges and actual line indentation, while the complete session remains one document history step");

    await importFixture(page, repeatedFixture);
    repeatedDepth = (await state(page)).historyLength;
    await enterInline();
    await selectFirstParagraph();
    await page.keyboard.press("Delete");
    const deletedRich = {
        text: repeatedUnit.repeat(2),
        text_runs: originalRich.text_runs.slice(1).map(run => shift(run, -unitBytes)),
        text_paragraphs: originalRich.text_paragraphs.slice(1).map(style => shift(style, -unitBytes)),
    };
    await assertRich(deletedRich, repeatedDepth, "Complete first-paragraph deletion");
    await nativeHistory("Control+z", originalRich, repeatedDepth, "Native deletion undo");
    await nativeHistory("Control+Shift+z", deletedRich, repeatedDepth, "Native deletion redo");
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null,
        undefined, { timeout: 10000 });
    assert.deepEqual(await documentState(page), repeatedDocument, "Cancel restores every deleted character and paragraph style");
    assert.equal((await state(page)).historyLength, repeatedDepth);
    check("Deleting a complete repeated paragraph keeps the surviving suffix's distinct fonts, colors and paragraph metrics; native undo restores the deleted rich data, redo restores the exact deletion, and Escape cancels without a document history entry");

    // Reuse the existing production browser text-input bridge. These synthetic
    // composition lifecycle events exercise the real native client, not an OS
    // IME; no JavaScript assigns Text, a caret, style ranges or document state.
    await enterInline();
    await selectFirstParagraph();
    const ime = page.locator(".ferroui-input-element");
    assert.equal(await ime.count(), 1);
    const preedit = repeatedUnit.repeat(2);
    const beginComposition = async () => {
        await ime.evaluate((input, data) => {
            if (document.activeElement !== input) throw new Error("The native inline TextBox must own the browser input bridge");
            input.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "" }));
            input.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data }));
            input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertCompositionText", data, isComposing: true }));
        }, preedit);
        await page.waitForFunction(preedit => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.editingComposition && view.chrome.InlineTextBox?.typography?.preedit === preedit;
        }, preedit, { timeout: 10000 });
        assert.deepEqual(await documentState(page), repeatedDocument, "Provisional replacement must not alter confirmed text or either rich style collection");
        assert.equal((await state(page)).historyLength, repeatedDepth);
        assert.deepEqual((await typography("InlineTextBox")).lines.map(line => line.x), [10, 10, 20, 30, 40],
            "Repeated preedit uses the captured replacement location in actual native paragraph geometry");
    };
    await beginComposition();
    await ime.evaluate(input => input.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, cancelable: true, data: "" })));
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingComposition,
        undefined, { timeout: 10000 });
    await assertRich(originalRich, repeatedDepth, "Empty composition-end cancellation");
    let restored = await typography("InlineTextBox");
    assert.deepEqual([Math.min(restored.selectionStart, restored.selectionEnd), Math.max(restored.selectionStart, restored.selectionEnd)],
        [0, repeatedUnit.length], "Cancellation restores the confirmed native selection before any later input");
    await beginComposition();
    await ime.evaluate((input, data) => {
        input.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, cancelable: true, data }));
        input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data, isComposing: false }));
    }, preedit);
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingComposition,
        undefined, { timeout: 10000 });
    await assertRich(insertedRich, repeatedDepth, "Committed repeated-paragraph composition");
    await nativeHistory("Control+z", originalRich, repeatedDepth, "Native composition undo");
    await nativeHistory("Control+Shift+z", insertedRich, repeatedDepth, "Native composition redo");
    const composedDocument = await documentState(page);
    await page.keyboard.press("Control+Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null,
        undefined, { timeout: 10000 });
    await history(repeatedDocument, composedDocument, repeatedDepth, "Repeated paragraph composition session");
    check("Production browser composition events keep confirmed rich data unchanged during repeated preedit, restore the selected rich paragraph on cancellation, commit exactly once despite trailing insertText, and preserve exact native and document undo/redo for both style collections");
}
