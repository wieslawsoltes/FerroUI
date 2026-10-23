import assert from "node:assert/strict";

// Uses only the real app, native-control bounds and browser input supplied by
// the main gate. No substitute document editor, synthetic selection, or browser
// process is created here.
export async function nativeAuthoringRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, chooseNative, frames, captureNativeScreenshot } = helpers;
    const command = async (name) => {
        assert.equal(await page.evaluate((name) => globalThis.ferroDesign.ferroDesignCommand(name), name), true);
        await frames(page);
    };
    const click = (name) => clickNative(page, name, "ModalViewport");
    const field = async (name, value) => {
        await click(name);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await frames(page);
    };
    const chooseNext = (name, count = 1) => chooseNative(page, name, { offset: count, scrollContainer: "ModalViewport" });
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });

    const text = fixtureNode(reference, 2, {
        kind: "text", name: "Unicode range", x: 40, y: 40, width: 320, height: 60,
        text: "A🦀BC", text_runs: [], font_family: "Inter", font_size: 24, font_weight: 400,
        italic: false, text_decoration: "none", text_auto_resize: "fixed", line_height: 28 / 24,
    });
    await importFixture(page, isolatedDocument(reference, [text], "Native rich text ranges"));
    await clickWorld(page, 100, 60);
    await command("rich-text");
    await click("RichTextSelection");
    // Navigate by native grapheme movement: A then one selected non-BMP emoji.
    // The core must store the resulting UTF-16 [1,3] range as UTF-8 [1,5].
    await page.keyboard.press("Control+Home");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    await chooseNext("RichTextWeightMode");
    await field("RichTextWeight", "700");
    await chooseNext("RichTextColorMode");
    await field("RichTextColor", "#7B61FF80");
    await clickNative(page, "RichTextApply");
    await waitDocument((doc) => doc.nodes[2].text_runs.length === 1 && doc.nodes[2].text_runs[0].font_weight === 700);
    const styled = (await documentState(page)).nodes[2];
    assert.equal(styled.text, "A🦀BC");
    assert.equal(styled.font_weight, 400, "Range authoring must leave the layer's base style unchanged");
    assert.deepEqual(styled.text_runs, [{
        start: 1, end: 5, font_family: null, font_size: null, font_weight: 700,
        color: { r: 123, g: 97, b: 255, a: 128 }, italic: null, decoration: null,
    }]);
    await captureNativeScreenshot(mode + "-rich-text-ranges", {
        controls: ["ModalContent", "RichTextPreview", "RichTextApply"],
    });
    // An unchanged Apply must add no duplicate history entry.
    await clickNative(page, "RichTextApply");
    await clickNative(page, "RichTextInheritAll");
    await waitDocument((doc) => doc.nodes[2].text_runs.length === 0);
    await clickNative(page, "RichTextDone");
    await page.keyboard.press("Control+z");
    assert.deepEqual((await documentState(page)).nodes[2], styled, "One undo restores all range overrides");
    await page.keyboard.press("Control+z");
    assert.deepEqual((await documentState(page)).nodes[2].text_runs, [], "The prior undo removes the single Apply transaction, including its repeated no-op");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual((await documentState(page)).nodes[2], styled);
    check("Native character selection styles one non-BMP emoji at exact UTF-8 boundaries, retains selection across field focus, previews the shared native text layout, clears overrides and restores one-step history");

    const rectangle = fixtureNode(reference, 2, { x: 40, y: 40, width: 240, height: 160, name: "Variable-bound layer" });
    const variables = isolatedDocument(reference, [rectangle], "Native typed variables");
    variables.variable_mode = "Default";
    variables.variables = {
        100: { id: 100, name: "Accent", collection: "Tokens", values: {
            Default: { type: "color", value: { r: 255, g: 0, b: 0, a: 255 } },
            Dark: { type: "color", value: { r: 0, g: 0, b: 255, a: 255 } },
        } },
        101: { id: 101, name: "Alias", collection: "Tokens", values: {
            Default: { type: "alias", value: 100 }, Dark: { type: "alias", value: 100 },
        } },
    };
    variables.next_id = 102;
    await importFixture(page, variables);
    await clickWorld(page, 120, 90);
    await command("variables");
    await field("VariableValue", "#FFAA0080");
    await clickNative(page, "VariableSave");
    await waitDocument((doc) => doc.variables[100].values.Default.value.a === 128);
    await click("VariableBind");
    await waitDocument((doc) => doc.nodes[2].variable_bindings.fill === 100);
    assert.deepEqual((await documentState(page)).nodes[2].fills[0].paint, { type: "solid", r: 255, g: 170, b: 0, a: 128 });
    await click("VariablesAdd");
    await field("VariableName", "Native alias");
    await chooseNext("VariableSource");
    await clickNative(page, "VariableSave");
    await waitDocument((doc) => doc.variables[102]?.name === "Native alias");
    let authored = await documentState(page);
    assert.deepEqual(authored.variables[102].values, {
        Default: { type: "alias", value: 100 }, Dark: { type: "alias", value: 100 },
    });
    await click("VariableBind");
    await waitDocument((doc) => doc.nodes[2].variable_bindings.fill === 102);
    await chooseNext("VariablesMode");
    assert.equal((await documentState(page)).variable_mode, "Default", "Browsing a mode does not change the active document mode");
    await click("VariablesActivateMode");
    await waitDocument((doc) => doc.variable_mode === "Dark");
    assert.deepEqual((await documentState(page)).nodes[2].fills[0].paint, { type: "solid", r: 0, g: 0, b: 255, a: 255 });
    await field("VariablesCollectionName", "Design tokens");
    await click("VariablesRenameCollection");
    await waitDocument((doc) => Object.values(doc.variables).every((variable) => variable.collection === "Design tokens"));
    await click("VariableEdit100");
    const beforeRejectedDelete = await documentState(page);
    await clickNative(page, "VariableDelete");
    assert.deepEqual(await documentState(page), beforeRejectedDelete, "Deleting an alias target must fail atomically");
    assert.equal((await state(page)).chrome.VariablesError.visible, true);
    await click("VariableEdit102");
    await captureNativeScreenshot(mode + "-visual-variables", {
        controls: ["ModalContent", "VariableName", "VariableSave"],
    });
    const beforeDelete = await documentState(page);
    await clickNative(page, "VariableDelete");
    await waitDocument((doc) => !doc.variables[102]);
    authored = await documentState(page);
    assert.deepEqual(authored.nodes[2].variable_bindings, {});
    assert.deepEqual(authored.nodes[2].fills, beforeDelete.nodes[2].fills, "Deleting the bound alias freezes the current appearance");
    await clickNative(page, "VariablesDone");
    await page.keyboard.press("Control+z");
    assert.deepEqual(await documentState(page), beforeDelete, "One undo restores the alias, binding and appearance together");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual(await documentState(page), authored);
    check("Native variable controls edit typed alpha colors, create aliases, bind layers, switch modes, rename collections, reject referenced deletion and preserve appearance with one-step undo");
}
