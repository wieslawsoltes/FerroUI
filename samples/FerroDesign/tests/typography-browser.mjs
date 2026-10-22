import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { inlineInputState, startInputObservation, stopInputObservation } from "./input-observations.mjs";

// Actual native TextBoxes/ComboBoxes, keyboard selection and TextPresenter
// diagnostics. No DOM text editor or test-only command mutates typography.
export async function typographyRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const typography = async (name) => {
        await page.waitForFunction((name) => {
            const field = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome[name];
            return field?.visible && field.measureValid && field.arrangeValid && field.typography?.runCount > 0;
        }, name, { timeout: 10000 });
        return (await state(page)).chrome[name].typography;
    };
    const sizes = (layout) => layout.runs.flatMap((run) => Array(run.utf16Length).fill(run.fontSize));
    let inlineEntry = 0;
    const openInline = async () => {
        const observation = { schemaVersion: 1, operation: "one native text double-click", mode,
            deadlineMs: 30000, before: inlineInputState(await state(page)) };
        const path = resolve(output, `${mode}-typography-inline-entry-${++inlineEntry}.json`);
        await startInputObservation(page);
        try {
            await clickWorld(page, 460, 250);
            const cleared = await state(page);
            assert.deepEqual(cleared.selection, [], "The reset click must reach the empty native canvas");
            assert.equal(cleared.chrome.CanvasHost.focusWithin, true);
            observation.beforeDoubleClick = inlineInputState(cleared);
            // Keep the same real pointer route, two presses and original delay.
            // Evidence records their actual browser and native timestamps;
            // Playwright's clickCount does not override FerroUI's click clock.
            await clickWorld(page, 66, 65, { clickCount: 2, delay: 80 });
            observation.afterDoubleClick = inlineInputState(await state(page));
            await page.waitForFunction(() => {
                const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                const field = view.chrome.InlineTextBox;
                return view.editingText === 2 && field?.visible && field.enabled && field.focused &&
                    field.measureValid && field.arrangeValid && field.width > 0 && field.height > 0 &&
                    !view.chrome.ModalBackdrop?.visible;
            }, undefined, { timeout: 30000 });
            await waitForNativeFrame(page, cleared.renderedFrames, "Native inline text entry");
            await waitForNativeScene(page, cleared.sceneFrames, "Native inline text entry");
            assert.equal((await state(page)).chrome.InlineTextBox.focused, true,
                "The public native TextBox must own keyboard focus");
            observation.status = "passed";
        } catch (error) {
            observation.status = "failed";
            observation.error = error.stack || String(error);
            throw new Error(`Native inline text entry failed; see ${path}`, { cause: error });
        } finally {
            observation.input = await stopInputObservation(page);
            observation.after = inlineInputState(await state(page));
            await writeFile(path, JSON.stringify(observation, null, 2));
            if (observation.status === "failed") console.error(`${mode} inline input evidence: ${JSON.stringify(observation)}`);
            else console.log(`${mode}: native inline entry ${inlineEntry} ${JSON.stringify({
                beforeSerial: observation.beforeDoubleClick.pointerPresses?.serial,
                presses: observation.after.pointerPresses?.entries.filter(entry =>
                    entry.serial > (observation.beforeDoubleClick.pointerPresses?.serial ?? 0)),
                frame: observation.after.renderedFrames, scene: observation.after.sceneFrames,
            })}`);
        }
    };
    const closeInline = async () => {
        await page.keyboard.press("Control+Enter");
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    };
    const click = (name) => clickNative(page, name, "ModalViewport");
    const draft = async (name, value) => {
        await click(name);
        assert.equal((await state(page)).chrome[name].focused, true, name + " owns native focus");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await frames(page);
    };
    const openTypography = async () => {
        await clickWorld(page, 66, 65);
        await clickNative(page, "RichTextButton", "InspectorViewport");
        await typography("RichTextSelection");
    };

    const text = fixtureNode(reference, 2, {
        kind: "text", name: "Actual styled editor", x: 40, y: 40, width: 440, height: 120,
        text: "AV Ω🦀ffi", font_family: "Inter", font_size: 24, font_weight: 400,
        italic: false, letter_spacing: 0.25, line_height: 1.5, text_auto_resize: "fixed",
        text_vertical_align: "top", text_decoration: "none", text_align: "left",
        fills: [{ paint: { type: "solid", r: 20, g: 20, b: 20, a: 255 }, opacity: 1, visible: true }],
        font_features: [{ tag: "kern", value: 0 }, { tag: "liga", value: 0 }],
        text_runs: [{ start: 3, end: 9, font_family: null, font_size: 36, font_weight: 700,
            color: { r: 196, g: 20, b: 66, a: 255 }, italic: true, decoration: "underline" }],
    });
    await importFixture(page, isolatedDocument(reference, [text], "Native typography parity"));
    const beforeEdit = await documentState(page), editDepth = (await state(page)).historyLength;
    await openInline();
    await page.keyboard.press("Control+Home");
    const plain = await typography("InlineTextBox");
    assert.equal(plain.direction, "LeftToRight");
    assert.deepEqual(sizes(plain), [24, 24, 24, 36, 36, 36, 24, 24, 24]);
    const rich = plain.runs.find((run) => run.text.includes("Ω"));
    assert.ok(rich, "The observed layout must contain the actual styled Unicode run");
    assert.equal(rich.fontWeight, 700);
    assert.equal(rich.italic, true);
    assert.equal(rich.decorationCount, 1);
    assert.deepEqual(rich.color, [196, 20, 66, 255]);
    assert.deepEqual(rich.features, [{ tag: "kern", value: 0 }, { tag: "liga", value: 0 }]);
    for (let i = 0; i < 3; i++) await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    const selected = await typography("InlineTextBox");
    assert.deepEqual([Math.min(selected.selectionStart, selected.selectionEnd), Math.max(selected.selectionStart, selected.selectionEnd)], [3, 6]);
    assert.deepEqual(sizes(selected), sizes(plain), "Selection foreground retains each run's font geometry");
    assert.equal(selected.layoutWidth, plain.layoutWidth);
    assert.equal(selected.layoutHeight, plain.layoutHeight);
    assert.ok(selected.selectionRects.some((rect) => rect.width > 0 && rect.height > 0));
    await captureNativeScreenshot(mode + "-native-rich-text-editing", { controls: ["InlineTextBox"] });
    check("The real on-canvas TextPresenter preserves rich font size, weight, color, decoration, OpenType features and exact UTF-16 selection geometry across a non-BMP character");

    await page.keyboard.insertText("é🧭");
    await waitDocument((doc) => doc.nodes[2].text === "AV é🧭ffi");
    const replaced = await typography("InlineTextBox");
    assert.deepEqual(sizes(replaced), sizes(plain), "Unicode replacement inherits the selected range's complete style");
    await closeInline();
    const afterEdit = await documentState(page);
    assert.deepEqual(afterEdit.nodes[2].text_runs, beforeEdit.nodes[2].text_runs,
        "Equal-byte-length Unicode replacement retains exact UTF-8 run boundaries");
    assert.equal((await state(page)).historyLength, editDepth + 1);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeEdit);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), afterEdit);
    check("Native Unicode range replacement keeps formatting in the visible editor and commits the complete text/range change as one document undo and redo");

    const featureText = fixtureNode(reference, 2, {
        ...text, name: "OpenType advances", text: "AVAVAV", text_runs: [], font_size: 40,
        font_features: [{ tag: "kern", value: 1 }], letter_spacing: 0, line_height: 1.4,
    });
    await importFixture(page, isolatedDocument(reference, [featureText], "Typed OpenType features"));
    await openTypography();
    const beforeFeatures = await documentState(page), featureDepth = (await state(page)).historyLength;
    const kerned = await typography("RichTextSelection");
    await draft("RichTextFeatures", "kern=0");
    await page.keyboard.press("Enter");
    await waitDocument((doc) => doc.nodes[2].font_features[0].value === 0);
    const unkerned = await typography("RichTextSelection");
    assert.ok(unkerned.layoutWidth - kerned.layoutWidth > 0.1,
        "Authoring kern=0 must change actual Inter/HarfBuzz glyph advances");
    assert.ok(unkerned.runs.every((run) => run.features.some((feature) => feature.tag === "kern" && feature.value === 0)));
    assert.equal((await state(page)).historyLength, featureDepth + 1);
    await chooseNative(page, "RichTextDirection", { index: 1, scrollContainer: "ModalViewport" });
    await waitDocument((doc) => doc.nodes[2].text_direction === "rtl");
    assert.equal((await typography("RichTextSelection")).direction, "RightToLeft");
    const afterFeatures = await documentState(page), authoredDepth = (await state(page)).historyLength;
    assert.equal(authoredDepth, featureDepth + 2);
    await draft("RichTextFeatures", "kern=NaN");
    await page.keyboard.press("Enter");
    await frames(page);
    assert.deepEqual(await documentState(page), afterFeatures, "Invalid feature values must not partially edit the document");
    assert.equal((await state(page)).historyLength, authoredDepth);
    assert.equal((await state(page)).chrome.RichTextTypographyError.visible, true);
    await page.keyboard.press("Escape");
    assert.equal((await state(page)).chrome.RichTextFeatures.text, "kern=0");
    assert.equal((await state(page)).chrome.RichTextTypographyError.visible, false);
    await clickNative(page, "RichTextDone");
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.equal((await documentState(page)).nodes[2].text_direction, undefined);
    assert.deepEqual((await documentState(page)).nodes[2].font_features, [{ tag: "kern", value: 0 }]);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeFeatures);
    await page.keyboard.press("Control+Shift+z");
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), afterFeatures);
    check("Native OpenType and direction controls change real shaping, reject invalid feature drafts atomically, restore Escape text and keep one-step history for each authored property");

    const rtlText = fixtureNode(reference, 2, {
        ...text, name: "Auto direction", text: " 123 שלום · Ω", text_runs: [], font_features: [],
        font_size: 28, letter_spacing: 0, line_height: 1.4,
    });
    await importFixture(page, isolatedDocument(reference, [rtlText], "Unicode paragraph direction"));
    await openTypography();
    await chooseNative(page, "RichTextDirection", { index: 2, scrollContainer: "ModalViewport" });
    await waitDocument((doc) => doc.nodes[2].text_direction === "auto");
    assert.equal((await typography("RichTextSelection")).direction, "RightToLeft",
        "Numbers and punctuation do not override the first strong Hebrew character");
    const authoredAuto = await documentState(page);
    await clickNative(page, "RichTextDone");
    await openInline();
    assert.equal((await typography("InlineTextBox")).direction, "RightToLeft");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("Latin · שלום 🧭");
    await waitDocument((doc) => doc.nodes[2].text === "Latin · שלום 🧭");
    assert.equal((await typography("InlineTextBox")).direction, "LeftToRight",
        "Auto paragraph direction follows confirmed native Unicode edits");
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.deepEqual(await documentState(page), authoredAuto, "Cancel restores text and authored Auto direction together");
    check("Auto paragraph direction resolves real Unicode strong classes in the native editor, updates during typing and survives cancellation with the complete document state");
}
