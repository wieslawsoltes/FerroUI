import assert from "node:assert/strict";

// Only the initial fixture supplies document data. All authored bindings,
// mode changes and literal edits use real native controls and pointer/keys.
export async function layoutVariableRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeScene,
        captureNativeScreenshot, flushStorage } = helpers;
    const wait = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("view", "document", "args", "return (" + source + ")(view, document, args)")(view, document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const click = name => clickNative(page, name, "ModalViewport");
    const choose = (name, index) => chooseNative(page, name, { index, scrollContainer: "ModalViewport" });
    const open = async () => {
        const before = await state(page);
        await clickNative(page, "VariablesRailButton");
        await wait(view => view.chrome.ModalBackdrop.visible && view.chrome.VariableBindingProperty?.visible);
        await waitForNativeScene(page, before.sceneFrames, "Open native spacing variables");
    };
    const close = async () => {
        const before = await state(page);
        await clickNative(page, "CloseModalButton");
        await wait(view => !view.chrome.ModalBackdrop.visible && view.chrome.CanvasHost.focusWithin);
        await waitForNativeScene(page, before.sceneFrames, "Close native spacing variables");
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        await page.keyboard.press("Control+z"); await frames(page);
        assert.deepEqual(await documentState(page), before, label + " exact undo");
        assert.equal((await state(page)).canRedo, true);
        await page.keyboard.press("Control+Shift+z"); await frames(page);
        assert.deepEqual(await documentState(page), after, label + " exact redo");
    };
    const layout = { axis: "row", gap: 8, cross_gap: 12,
        padding: { top: 10, right: 10, bottom: 10, left: 10 }, main_align: "start", cross_align: "start",
        width: "fixed", height: "fixed", columns: 3, reverse: false, wrap: false };
    const fixture = isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Responsive spacing", x: 40, y: 40,
            width: 320, height: 180, children: [3, 4], layout, corner_radius: 0, fills: [] }),
        fixtureNode(reference, 3, { parent: 2, name: "First", x: 10, y: 10, width: 40, height: 25 }),
        fixtureNode(reference, 4, { parent: 2, name: "Second", x: 58, y: 10, width: 50, height: 30 }),
    ], "Native auto-layout spacing variables");
    fixture.variables = { 100: { id: 100, name: "Spacing", collection: "Layout", values: {
        Default: { type: "number", value: 16.25 }, Compact: { type: "number", value: 24.5 } } } };
    fixture.variable_mode = "Default"; fixture.next_id = 101;
    await importFixture(page, fixture);
    await clickWorld(page, 44, 44);
    assert.deepEqual((await state(page)).selection, [2]);
    await open();
    for (const [index, key] of [[9, "layout.padding.top"], [12, "layout.padding.left"]]) {
        await choose("VariableBindingProperty", index);
        const before = await state(page); await click("VariableBind");
        await wait((view, document, args) => document.nodes[2].variable_bindings[args.key] === 100 &&
            view.historyLength === args.depth + 1, { key, depth: before.historyLength });
        await waitForNativeScene(page, before.sceneFrames, "Bind native spacing variable");
    }
    await choose("VariableBindingProperty", 13);
    const beforeGap = await documentState(page), gapDepth = (await state(page)).historyLength;
    await click("VariableBind");
    await wait((view, document) => document.nodes[2].variable_bindings["layout.gap"] === 100);
    const afterGap = await documentState(page);
    assert.equal(afterGap.nodes[3].x, 16.25); assert.equal(afterGap.nodes[3].y, 16.25);
    assert.equal(afterGap.nodes[4].x, 72.5);
    assert.deepEqual(afterGap.nodes[2].layout.padding, { top: 16.25, right: 10, bottom: 10, left: 16.25 });
    await captureNativeScreenshot(mode + "-layout-spacing-variables", { controls: ["VariableBindingProperty", "VariableBind"] });
    await close(); await history(beforeGap, afterGap, gapDepth, "Spacing binding");
    check("Native variable controls bind independent padding sides and gap, reflow exact child geometry, and retain one-step complete undo/redo");

    await open(); await choose("VariablesMode", 1);
    const beforeMode = await documentState(page), modeDepth = (await state(page)).historyLength;
    await click("VariablesActivateMode");
    await wait((view, document) => document.variable_mode === "Compact" && document.nodes[2].layout.gap === 24.5);
    const compact = await documentState(page);
    assert.deepEqual(compact.nodes[2].layout.padding, { top: 24.5, right: 10, bottom: 10, left: 24.5 });
    assert.equal(compact.nodes[4].x, 89);
    await close(); await history(beforeMode, compact, modeDepth, "Spacing mode activation");
    check("Native mode activation updates all bound spacing and dependent layout in one exact undo transaction");

    const openLayout = await state(page);
    await clickNative(page, "EditLayoutButton", "InspectorViewport");
    await wait(view => view.chrome.LayoutPaddingLeft?.visible);
    await waitForNativeScene(page, openLayout.sceneFrames, "Open bound padding editor");
    assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, false);
    await click("LayoutPaddingLeft");
    await page.keyboard.press("Control+a"); await page.keyboard.insertText("31.25");
    await page.keyboard.press("Enter");
    await wait((view, document) => document.nodes[2].layout.padding.left === 31.25);
    const literal = await documentState(page);
    assert.equal(literal.nodes[2].variable_bindings["layout.padding.left"], undefined);
    assert.equal(literal.nodes[2].variable_bindings["layout.padding.top"], 100);
    assert.equal(literal.nodes[2].variable_bindings["layout.gap"], 100);
    await close(); await open(); await choose("VariablesMode", 0); await click("VariablesActivateMode");
    await wait((view, document) => document.variable_mode === "Default");
    const restoredMode = await documentState(page);
    assert.deepEqual(restoredMode.nodes[2].layout.padding, { top: 16.25, right: 10, bottom: 10, left: 31.25 });
    assert.equal(restoredMode.nodes[4].x, 87.5);
    await close();
    await captureNativeScreenshot(mode + "-layout-literal-and-bound-spacing", { controls: ["CanvasHost", "PaddingField"] });
    await flushStorage(page);
    const stored = await page.evaluate(async () => JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text));
    assert.deepEqual(stored, restoredMode);
    check("A native literal edit detaches only the changed side; subsequent mode changes and durable JSON preserve the remaining bindings and exact reflow");

    await open(); await choose("VariableBindingProperty", 13);
    const beforeUnbind = await documentState(page), unbindDepth = (await state(page)).historyLength;
    await click("VariableUnbind");
    await wait((view, document) => document.nodes[2].variable_bindings["layout.gap"] === undefined);
    const unbound = await documentState(page);
    assert.equal(unbound.nodes[2].layout.gap, beforeUnbind.nodes[2].layout.gap);
    await close(); await history(beforeUnbind, unbound, unbindDepth, "Spacing unbind");
    check("Native Unbind freezes the resolved gap and preserves exact binding ownership through undo and redo");
}
