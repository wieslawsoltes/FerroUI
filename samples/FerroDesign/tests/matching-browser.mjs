import assert from "node:assert/strict";

// Only fixture import sets initial document state. Every selection command is
// reached through real native FerroUI controls or the canvas keyboard route.
export async function matchingRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const fill = (r, g, b) => ({ paint: { type: "solid", r, g, b, a: 255 }, opacity: 1, visible: true });
    const blue = fill(40, 100, 220), red = fill(220, 50, 60);
    const stroke = { paint: { type: "solid", r: 0, g: 0, b: 0, a: 255 }, width: 2,
        opacity: 1, visible: true, align: "center", cap: "butt", join: "miter", dash: [] };
    const effect = { kind: "drop_shadow", color: { r: 0, g: 0, b: 0, a: 100 },
        offset: { x: 0, y: 4 }, radius: 4, spread: 0, visible: true };
    const node = (id, properties = {}) => fixtureNode(reference, id, {
        kind: "rectangle", name: "Matching layer " + id, x: 40, y: 40, width: 80, height: 60,
        fills: [blue], strokes: [], effects: [], corner_radius: 0, corner_radii: null,
        ...properties,
    });
    const selected = async expected => page.waitForFunction(expected =>
        JSON.stringify(JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection) === JSON.stringify(expected),
    expected, { timeout: 10000 });
    const snapshot = async () => {
        const view = await state(page);
        return { document: await documentState(page), history: view.historyLength,
            redo: view.canRedo, revision: view.revision, documentRevision: view.documentRevision };
    };
    const unchanged = async (before, label, actionBefore = before) => {
        const view = await state(page);
        assert.deepEqual(await documentState(page), before.document, label + " preserves all authored fields");
        assert.equal(view.historyLength, before.history, label + " does not create an undo entry");
        assert.equal(view.canRedo, before.redo, label + " preserves redo");
        // Canvas pointer selection opens and cancels a no-move transaction,
        // which refreshes its render revisions. Measure the selection command
        // itself while still comparing authored content/history to the fixture.
        assert.equal(view.revision, actionBefore.revision, label + " leaves editor content revision unchanged");
        assert.equal(view.documentRevision, actionBefore.documentRevision, label + " leaves document revision unchanged");
    };
    const same = async (control, expected) => {
        await clickNative(page, "SelectMatchingButton", "InspectorViewport");
        await page.waitForFunction(() =>
            JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalBackdrop?.visible === true,
        null, { timeout: 10000 });
        await clickNative(page, control, "ModalViewport");
        // Appearance matching may retain the already-selected source. Require
        // the native command's dialog closure as well as its exact selection.
        await page.waitForFunction(() =>
            JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalBackdrop?.visible === false,
        null, { timeout: 10000 });
        await selected(expected);
    };
    const selectSource = async (x = 80, y = 70) => {
        // Clicking inside an existing multi-selection preserves it for drag.
        // Clear through the native canvas before choosing a new source layer.
        const beforeClear = await state(page);
        if (beforeClear.selection.length > 0) await page.keyboard.press("Escape");
        await selected([]);
        if (beforeClear.selection.length > 0) {
            // Escape swaps the inspector. Publish that transition separately
            // before clickWorld establishes its next selection-frame floor.
            await waitForNativeFrame(page, beforeClear.renderedFrames, "Clear matching source selection");
            await waitForNativeScene(page, beforeClear.sceneFrames, "Clear matching source selection");
        }
        const cleared = await state(page);
        assert.equal(cleared.chrome.PageInspector?.visible, true, "Clearing the matching source shows the page inspector");
        assert.equal(cleared.chrome.DesignInspector?.visible, false, "Clearing the matching source hides the layer inspector");
        assert.equal(cleared.chrome.SelectMatchingButton?.visible, false, "Clearing the matching source hides its native action");
        await clickWorld(page, x, y);
        await selected([2]);
    };

    await importFixture(page, isolatedDocument(reference, [
        node(2, { name: "Source", strokes: [stroke], effects: [effect] }),
        node(3, { x: 160, kind: "ellipse", strokes: [stroke] }),
        node(4, { x: 280, fills: [red], strokes: [stroke], effects: [effect] }),
        node(5, { x: 400, visible: false }),
        node(6, { x: 520, locked: true }),
        node(7, { kind: "frame", x: 40, y: 160, fills: [], children: [8] }),
        node(8, { parent: 7, x: 0, y: 0 }),
        node(9, { kind: "frame", x: 160, y: 160, fills: [], children: [10], visible: false }),
        node(10, { parent: 9, x: 0, y: 0 }),
    ], "Native select same"));
    await selectSource();
    await page.keyboard.press("ArrowRight");
    let view = await state(page);
    await page.keyboard.press("Control+z");
    await waitForNativeFrame(page, view.renderedFrames, "Prepare selection redo");
    const original = await snapshot();
    assert.equal(original.redo, true);
    await same("SelectSameFill", [8, 3, 2]);
    await unchanged(original, "Same fill through native dialog");
    await captureNativeScreenshot(mode + "-native-matching-fill", { controls: ["CanvasHost", "SelectMatchingButton"] });
    for (const [control, expected] of [
        ["SelectSameStroke", [4, 3, 2]], ["SelectSameEffect", [4, 2]], ["SelectSameProperties", [2]],
    ]) {
        await selectSource();
        const actionBefore = await snapshot();
        await same(control, expected);
        await unchanged(original, control, actionBefore);
    }
    check("Native Select same compares full fill/stroke/effect stacks and appearance, selects nested matching layers, excludes hidden/locked subtrees and preserves document revisions, undo and redo");

    await selectSource();
    await page.keyboard.down("Shift");
    try { await clickWorld(page, 320, 70); }
    finally { await page.keyboard.up("Shift"); }
    await selected([2, 4]);
    const mixedBefore = await snapshot();
    await same("SelectSameFill", [8, 4, 3, 2]);
    await unchanged(original, "Mixed fill signature union", mixedBefore);
    // The inherited selection includes child8, so its root7 must be excluded
    // too: inversion cannot select an already-selected child via its parent.
    await page.keyboard.press("Control+Shift+A");
    await selected([]);
    await unchanged(original, "Native inverse with selected descendants", mixedBefore);
    check("Native mixed-property selection forms a union and Ctrl+Shift+A inverts eligible roots without reselecting a chosen descendant indirectly");

    const run = (start, end, weight) => ({ start, end, font_family: null, font_size: null,
        font_weight: weight, color: null, italic: null, decoration: null });
    await importFixture(page, isolatedDocument(reference, [
        node(2, { kind: "text", name: "Mixed source", text: "A🙂B", font_size: 24,
            text_runs: [run(1, 5, 700)], width: 180 }),
        node(3, { kind: "text", name: "Matching text", x: 260, text: "prefixBoldEnds", font_size: 24,
            text_runs: [run(0, 6, 400), run(6, 8, 700), run(8, 10, 700), run(10, 14, 400)], width: 220 }),
        node(4, { kind: "text", name: "Same fonts different size", x: 40, y: 160, text: "smallBoldTail", font_size: 16,
            text_runs: [run(5, 9, 700)], width: 180 }),
        node(5, { kind: "text", name: "Other font weight", x: 260, y: 160, text: "Different", font_weight: 900, width: 180 }),
    ], "Native matching typography"));
    await selectSource(75, 60);
    const typography = await snapshot();
    await same("SelectSameText", [3, 2]);
    await unchanged(typography, "Resolved character style matching");
    await selectSource(75, 60);
    const fontBefore = await snapshot();
    await same("SelectSameFont", [4, 3, 2]);
    await unchanged(typography, "Used font face matching", fontBefore);
    check("Native text matching ignores Unicode content and range lengths while resolving inherited character styles; font matching includes the actually used family, weight and italic face set independently of size");
}
