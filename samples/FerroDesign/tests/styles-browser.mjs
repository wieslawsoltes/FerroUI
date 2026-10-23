import assert from "node:assert/strict";

export async function localStyleRegressions(page, reference, check, helpers, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeFrame, captureNativeScreenshot } = helpers;
    const red = { r: 239, g: 68, b: 68, a: 255 };
    const blue = { r: 37, g: 99, b: 235, a: 255 };
    const green = { r: 16, g: 185, b: 129, a: 255 };
    // Preserve the serializer's tagged solid-paint shape rather than assuming
    // whether a tuple variant is flattened by a particular serde version.
    const prototype = Object.values(reference.nodes).find(node => node.fills?.some(fill => fill.paint?.type === "solid"));
    assert.ok(prototype, "The browser style fixture needs a valid serialized solid paint");
    const solid = structuredClone(prototype.fills.find(fill => fill.paint.type === "solid"));
    solid.paint = { ...solid.paint, ...red }; solid.opacity = 1; solid.visible = true;
    const fills = [solid];
    const document = isolatedDocument(reference, [
        fixtureNode(reference, 2, { name: "Primary surface", x: 40, y: 40, width: 100, height: 80,
            fills: structuredClone(fills), strokes: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { name: "Secondary surface", x: 180, y: 40, width: 100, height: 80,
            fills: structuredClone(fills), strokes: [], corner_radius: 0 }),
        fixtureNode(reference, 4, { kind: "text", name: "Title typography", x: 40, y: 190, width: 260, height: 60,
            text: "café Ω", font_family: "Inter", font_size: 24, font_weight: 500,
            text_runs: [{ start: 0, end: 5, font_family: null, font_size: 28, font_weight: null,
                color: red, italic: null, decoration: null }], fills: structuredClone(fills), strokes: [], corner_radius: 0 }),
    ], "Native local style lifecycle");
    document.styles = {};
    await importFixture(page, document);
    const open = async () => {
        await clickNative(page, "ActionsButton");
        await clickNative(page, "Action-styles", "ModalViewport");
    };
    const field = async (name, value) => {
        await clickNative(page, name, "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    const clearSelection = async () => {
        await clickNative(page, "SelectTool");
        const before = await state(page);
        assert.equal(before.chrome.CanvasHost.focusWithin, true);
        await page.keyboard.press("Escape");
        await waitForNativeFrame(page, before.renderedFrames, "Clear local-style selection");
        assert.deepEqual((await state(page)).selection, []);
    };
    const selectSurfaces = async () => {
        // Clicking a member of a multiselection preserves the group for
        // movement. Establish the next style operation's selection explicitly.
        await clearSelection();
        await clickWorld(page, 80, 80);
        await page.keyboard.down("Shift");
        await clickWorld(page, 220, 80);
        await page.keyboard.up("Shift");
        assert.deepEqual([...((await state(page)).selection)].sort((a, b) => a - b), [2, 3]);
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        await page.keyboard.press("Control+z"); await frames(page);
        assert.deepEqual(await documentState(page), before, label + " undo restores all fields");
        await page.keyboard.press("Control+Shift+z"); await frames(page);
        assert.deepEqual(await documentState(page), after, label + " redo restores all fields");
    };

    await selectSurfaces();
    const beforeCreate = await documentState(page), createDepth = (await state(page)).historyLength;
    await open(); await clickNative(page, "StyleCreateFill", "ModalActions");
    let created = await documentState(page);
    const id = Number(Object.keys(created.styles)[0]);
    assert.ok(id > 0);
    assert.equal(created.nodes[2].fill_style, id);
    assert.equal(created.nodes[3].fill_style, id);
    assert.equal(created.nodes[4].fill_style, null);
    assert.deepEqual((await state(page)).selection, [2, 3]);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeCreate, created, createDepth, "Native create-and-apply style");
    check("Native local style creation captures the complete selected paint stack, binds the selection, and has one exact undo/redo transaction");

    await open(); await clickNative(page, "StyleEdit" + id, "ModalViewport");
    await field("StyleName", "Brand / Primary");
    await field("StyleDescription", "Shared surface Ω");
    const beforeColor = await documentState(page), colorDepth = (await state(page)).historyLength;
    await field("StyleFillColor0", "#2563EB");
    const recolored = await documentState(page);
    assert.equal(recolored.styles[id].name, "Brand / Primary");
    assert.equal(recolored.styles[id].description, "Shared surface Ω");
    for (const node of [2, 3]) {
        assert.equal(recolored.nodes[node].fill_style, id);
        for (const [channel, value] of Object.entries(blue)) assert.equal(recolored.nodes[node].fills[0].paint[channel], value);
    }
    assert.deepEqual(recolored.nodes[4], beforeColor.nodes[4]);
    await captureNativeScreenshot(mode + "-native-local-styles", { controls: ["ModalPanel", "StyleName", "StyleFillColor0"] });
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeColor, recolored, colorDepth, "Native shared color edit");
    check("Native style name, description and color editing propagates to both linked layers, preserves their bindings and Unicode metadata, renders, and undoes once");

    // Keep a real redo entry so rejected drafts and action-menu shortcuts
    // cannot silently discard the next valid document transaction.
    await page.keyboard.press("ArrowRight"); await frames(page);
    const redoAfterInvalid = await documentState(page);
    assert.notDeepEqual(redoAfterInvalid, recolored);
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), recolored);
    assert.equal((await state(page)).canRedo, true);
    await open(); await clickNative(page, "StyleEdit" + id, "ModalViewport");
    const beforeInvalid = await state(page), invalidDepth = beforeInvalid.historyLength;
    await field("StyleFillOpacity0", "150");
    assert.equal((await state(page)).chrome.StyleFillOpacity0Error.visible, true);
    assert.deepEqual(await documentState(page), recolored);
    assert.equal((await state(page)).historyLength, invalidDepth);
    await clickNative(page, "StyleDone", "ModalActions");
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, true,
        "Done must retain a style editor with an invalid field");
    assert.equal((await state(page)).chrome.CanvasHost.focusWithin, true,
        "The refused close leaves the canvas owning the actual Ctrl+K command route");
    await page.keyboard.press("Control+k"); await frames(page);
    const blocked = await state(page);
    assert.equal(blocked.chrome.ModalBackdrop.visible, true);
    assert.equal(blocked.chrome.StyleFillOpacity0.text, "150",
        "The action shortcut must not replace or clear the rejected native draft");
    assert.equal(blocked.chrome.StyleFillOpacity0Error.visible, true);
    assert.equal(blocked.chrome.StyleName.text, "Brand / Primary");
    assert.equal(blocked.chrome.ActionSearch, undefined,
        "A rejected style draft must prevent the action menu from replacing its editor");
    assert.deepEqual(blocked.selection, beforeInvalid.selection);
    assert.equal(blocked.historyLength, invalidDepth);
    assert.equal(blocked.canUndo, beforeInvalid.canUndo);
    assert.equal(blocked.canRedo, true);
    assert.deepEqual(await documentState(page), recolored);
    await clickNative(page, "StyleFillOpacity0", "ModalViewport");
    await page.keyboard.press("Escape"); await frames(page);
    assert.equal((await state(page)).chrome.StyleFillOpacity0.text, "100");
    assert.equal((await state(page)).chrome.StyleFillOpacity0Error.visible, false);
    await clickNative(page, "StyleDone", "ModalActions");
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, false);
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), redoAfterInvalid,
        "The transaction that preceded rejected style actions must still redo exactly");
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), recolored);
    await open(); await clickNative(page, "StyleEdit" + id, "ModalViewport");
    assert.equal((await state(page)).chrome.StyleFillOpacity0.text, "100");
    assert.equal((await state(page)).chrome.StyleFillOpacity0Error.visible, false);
    await clickNative(page, "StyleDone", "ModalActions");
    check("Invalid native shared-style drafts survive refused Done and Ctrl+K with their validation, document and redo intact; Escape restores the field and permits normal close/reopen");

    await clearSelection();
    await clickWorld(page, 80, 80);
    assert.deepEqual((await state(page)).selection, [2]);
    await open(); await clickNative(page, "StyleEdit" + id, "ModalViewport");
    const beforeDetach = await documentState(page), detachDepth = (await state(page)).historyLength;
    await clickNative(page, "StyleDetachSelected", "ModalViewport");
    const detached = await documentState(page);
    assert.equal(detached.nodes[2].fill_style, null);
    assert.equal(detached.nodes[3].fill_style, id);
    assert.deepEqual(detached.nodes[2].fills, beforeDetach.nodes[2].fills);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeDetach, detached, detachDepth, "Native detach style");
    await open(); await clickNative(page, "StyleEdit" + id, "ModalViewport");
    await field("StyleFillColor0", "10B981");
    let independent = await documentState(page);
    assert.deepEqual(independent.nodes[2].fills, detached.nodes[2].fills);
    for (const [channel, value] of Object.entries(green)) assert.equal(independent.nodes[3].fills[0].paint[channel], value);
    check("Native style detachment preserves appearance and history while later shared edits change only the remaining linked layer");

    const beforeDelete = await documentState(page), deleteDepth = (await state(page)).historyLength;
    await clickNative(page, "StyleDelete", "ModalActions");
    const deleted = await documentState(page);
    assert.equal(deleted.styles[id], undefined);
    assert.equal(deleted.nodes[3].fill_style, null);
    for (const node of [2, 3]) assert.deepEqual(deleted.nodes[node].fills, beforeDelete.nodes[node].fills);
    await clickNative(page, "CloseModalButton");
    await history(beforeDelete, deleted, deleteDepth, "Native delete style");
    check("Deleting a native local style retains every layer's effective appearance; Undo restores the definition and all links atomically");

    await clearSelection();
    await clickWorld(page, 80, 80);
    await page.keyboard.down("Shift"); await clickWorld(page, 80, 215); await page.keyboard.up("Shift");
    await open(); await clickNative(page, "StyleCreateText", "ModalActions");
    const withTextStyle = await documentState(page);
    const textId = withTextStyle.nodes[4].text_style;
    assert.ok(textId > 0);
    assert.equal(withTextStyle.nodes[2].text_style, null);
    const beforeText = await documentState(page), textDepth = (await state(page)).historyLength;
    await field("StyleFontSize", "36");
    const changedText = await documentState(page);
    assert.equal(changedText.nodes[4].font_size, 36);
    assert.equal(changedText.nodes[4].text_style, textId);
    assert.deepEqual(changedText.nodes[4].text_runs, beforeText.nodes[4].text_runs);
    assert.deepEqual(changedText.nodes[2], beforeText.nodes[2]);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeText, changedText, textDepth, "Native shared typography edit");
    check("Native text styles in a mixed selection affect compatible text only, retain character formatting and Unicode ranges, and preserve one-step history");

    await selectSurfaces(); await open(); await clickNative(page, "StyleCreateEffect", "ModalActions");
    const effectId = (await documentState(page)).nodes[2].effect_style;
    await clickNative(page, "StyleAddEffect", "ModalViewport");
    await field("StyleEffectNumber0_0", "12");
    const effects = await documentState(page);
    for (const node of [2, 3]) {
        assert.equal(effects.nodes[node].effect_style, effectId);
        assert.equal(effects.nodes[node].effects[0].radius, 12);
    }
    const duplicateDepth = (await state(page)).historyLength;
    await clickNative(page, "StyleStack2_0", "ModalViewport");
    const duplicated = await documentState(page);
    assert.equal(duplicated.nodes[2].effects.length, 2);
    assert.deepEqual(duplicated.nodes[2].effects, duplicated.nodes[3].effects);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(effects, duplicated, duplicateDepth, "Native shared effect duplication");
    check("Native shared effects support creation, numeric authoring and stack duplication with linked propagation and exact undo/redo");

    await importFixture(page, document);
    await selectSurfaces(); await open(); await clickNative(page, "StyleCreateFill", "ModalActions");
    const conversionId = (await documentState(page)).nodes[2].fill_style;
    // Keep the same dialog open across the field edit and type conversion.
    // Reopening it would hide an opening-time color captured by the ComboBox.
    await field("StyleFillColor0", "2563EB");
    const beforeLinear = await documentState(page), linearDepth = (await state(page)).historyLength;
    await chooseNative(page, "StyleFillType0", { index: 1, scrollContainer: "ModalViewport" });
    const linear = await documentState(page);
    const expectedLinear = {
        type: "linear_gradient", start: { x: 0, y: 0 }, end: { x: 1, y: 1 },
        stops: [{ offset: 0, color: blue }, { offset: 1, color: { r: 255, g: 255, b: 255, a: 255 } }],
    };
    for (const node of [2, 3]) {
        assert.equal(linear.nodes[node].fill_style, conversionId);
        assert.deepEqual(linear.nodes[node].fills[0].paint, expectedLinear,
            "A native paint type change uses the solid color just committed in the open dialog");
    }
    assert.deepEqual(linear.nodes[4], beforeLinear.nodes[4]);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeLinear, linear, linearDepth, "Native solid-to-gradient conversion");

    await open(); await clickNative(page, "StyleEdit" + conversionId, "ModalViewport");
    await field("StyleStopColor0_0", "10B981");
    const beforeRadial = await documentState(page), radialDepth = (await state(page)).historyLength;
    await chooseNative(page, "StyleFillType0", { index: 2, scrollContainer: "ModalViewport" });
    const radial = await documentState(page);
    const expectedRadial = {
        type: "radial_gradient", center: { x: 0.5, y: 0.5 }, radius: 0.5,
        stops: [{ offset: 0, color: green }, { offset: 1, color: { r: 255, g: 255, b: 255, a: 255 } }],
    };
    for (const node of [2, 3]) {
        assert.equal(radial.nodes[node].fill_style, conversionId);
        assert.deepEqual(radial.nodes[node].fills[0].paint, expectedRadial,
            "A native gradient type change uses the first stop color just committed in the open dialog");
    }
    assert.deepEqual(radial.nodes[4], beforeRadial.nodes[4]);
    await clickNative(page, "StyleDone", "ModalActions");
    await history(beforeRadial, radial, radialDepth, "Native linear-to-radial conversion");
    check("Native paint type changes use the latest solid or gradient-stop color, preserve linked propagation and default geometry, and each undo and redo exactly once");
}
