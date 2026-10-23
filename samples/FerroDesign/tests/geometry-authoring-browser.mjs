import assert from "node:assert/strict";

export async function geometryAuthoringRegressions(page, reference, check, helpers, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const near = (a, b, label) => assert.ok(Math.abs(a - b) < 1e-7, label + ": " + a + " != " + b);
    const edit = async (name, value, container = "ModalViewport") => {
        await clickNative(page, name, container);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " must be one transaction");
        await page.keyboard.press("Control+z");
        await frames(page);
        assert.deepEqual(await documentState(page), before, label + " undo restores every field");
        await page.keyboard.press("Control+Shift+z");
        await frames(page);
        assert.deepEqual(await documentState(page), after, label + " redo restores every field");
    };
    const selectParent = async (id, label) => {
        const before = await state(page);
        await page.keyboard.press("Shift+Enter");
        await page.waitForFunction((id) => {
            const selection = JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection;
            return selection.length === 1 && selection[0] === id;
        }, id);
        // Keyboard selection updates the model before the replacement inspector
        // owns the compositor's hit-test scene. Use the same publication
        // contract as clickWorld before sending input to its native fields.
        await waitForNativeFrame(page, before.renderedFrames, label);
        await waitForNativeScene(page, before.sceneFrames, label);
    };

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Affine parent", x: 40, y: 40,
            width: 240, height: 180, children: [3], fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { kind: "group", parent: 2, name: "Editable group", x: 20, y: 15,
            width: 100, height: 60, children: [4], fills: [], corner_radius: 0 }),
        fixtureNode(reference, 4, { parent: 3, name: "Unflattened child", x: 7, y: 9,
            width: 23, height: 17, corner_radius: 0 }),
    ], "Native affine transform"));
    await clickWorld(page, 78, 72);
    assert.deepEqual((await state(page)).selection, [4]);
    await selectParent(3, "Affine parent selection");
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await clickNative(page, "AffineButton", "InspectorViewport");
    await edit("AffineC", "0.5");
    const transformed = await documentState(page);
    near(transformed.nodes[3].affine_transform.c, 0.5, "Native horizontal shear");
    assert.deepEqual(transformed.nodes[4], before.nodes[4], "Matrix editing retains child geometry, properties and IDs");
    assert.deepEqual(transformed.nodes[2].children, [3]);
    assert.deepEqual(transformed.nodes[3].children, [4]);
    await captureNativeScreenshot(mode + "-native-affine-authoring", { controls: ["ModalPanel", "AffineA", "AffineC", "AffineX"] });
    await clickNative(page, "CloseModalButton");
    await history(before, transformed, depth, "Native affine field");
    check("Native affine matrix fields preserve editable child geometry and hierarchy, render the transform, and commit one complete undo/redo transaction");

    await clickNative(page, "AffineButton", "InspectorViewport");
    const invalidDepth = (await state(page)).historyLength;
    for (const [name, invalid, restored] of [["AffineC", "NaN", "0.5"], ["AffineA", "0", "1"]]) {
        await edit(name, invalid);
        const rejected = await state(page);
        assert.equal(rejected.chrome[name].text, invalid);
        assert.equal(rejected.chrome.AffineError.visible, true);
        assert.equal(rejected.historyLength, invalidDepth);
        assert.deepEqual(await documentState(page), transformed);
        await page.keyboard.press("Escape");
        await frames(page);
        assert.equal((await state(page)).chrome[name].text, restored);
    }
    await clickNative(page, "CloseModalButton");
    assert.deepEqual(await documentState(page), transformed);
    assert.equal((await state(page)).historyLength, invalidDepth);
    check("Nonfinite and singular native affine drafts retain validation without touching geometry/history; Escape restores the confirmed matrix and permits closing");

    await clickNative(page, "AffineButton", "InspectorViewport");
    await clickNative(page, "AffineSkewAngle", "ModalViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("20");
    await clickNative(page, "AffineSkewX", "ModalViewport");
    const skewed = await documentState(page), shear = Math.tan(20 * Math.PI / 180);
    near(skewed.nodes[3].affine_transform.c, 0.5 + shear, "Composed shear");
    near(skewed.nodes[3].x + skewed.nodes[3].affine_transform.c * 30,
        transformed.nodes[3].x + transformed.nodes[3].affine_transform.c * 30, "Skew retains local center");
    assert.deepEqual(skewed.nodes[4], transformed.nodes[4]);
    await clickNative(page, "CloseModalButton");
    await history(transformed, skewed, invalidDepth, "Centered native skew");
    check("Native horizontal skew composes with an existing affine matrix around the layer center while preserving child geometry and one-step history");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Constraint frame", x: 40, y: 40,
            width: 200, height: 100, children: [3], fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { parent: 2, name: "Pinned child", x: 20, y: 10,
            width: 40, height: 30, corner_radius: 0,
            constraints: { horizontal: "start", vertical: "start" } }),
    ], "Native constraint diagram"));
    await clickWorld(page, 80, 65);
    await clickNative(page, "ConstraintRight", "InspectorViewport");
    await clickNative(page, "ConstraintCenterY", "InspectorViewport");
    assert.deepEqual((await documentState(page)).nodes[3].constraints, { horizontal: "stretch", vertical: "center" });
    await clickWorld(page, 80, 65);
    await selectParent(2, "Constraint parent selection");
    await edit("WidthField", "300", "InspectorViewport");
    const widened = await documentState(page);
    assert.deepEqual([widened.nodes[2].width, widened.nodes[2].height], [300, 100],
        "The native width field must resize the selected constraint frame");
    assert.deepEqual([widened.nodes[3].x, widened.nodes[3].y, widened.nodes[3].width, widened.nodes[3].height], [20, 10, 140, 30]);
    await edit("HeightField", "160", "InspectorViewport");
    const resized = await documentState(page);
    assert.deepEqual([resized.nodes[2].width, resized.nodes[2].height], [300, 160],
        "The native height field must resize the selected constraint frame");
    assert.deepEqual((await state(page)).selection, [2]);
    const constrained = resized.nodes[3];
    assert.deepEqual([constrained.x, constrained.y, constrained.width, constrained.height], [20, 40, 140, 30]);
    check("Native graphical edge and center constraints drive actual parent resizing: opposing horizontal pins stretch the child and vertical centering preserves its size");

    await clickWorld(page, 80, 95);
    const selectedScene = (await state(page)).sceneFrames;
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selectionCount === 0);
    // Escape replaces the selected-layer inspector with the page inspector.
    // Published native layout bounds alone can precede the compositor's new
    // hit-test scene; clicking those bounds early can hit the old inspector.
    await waitForNativeScene(page, selectedScene, "Page inspector after Escape");
    const beforePreset = await documentState(page), presetDepth = (await state(page)).historyLength;
    await clickNative(page, "PageFramePresetsButton", "InspectorViewport");
    await clickNative(page, "FramePreset1", "ModalViewport");
    const presetDocument = await documentState(page), presetState = await state(page);
    assert.equal(presetState.selectionCount, 1);
    const id = presetState.selection[0], created = presetDocument.nodes[id];
    assert.equal(created.kind, "frame");
    assert.equal(created.name, "Phone");
    assert.deepEqual([created.width, created.height, created.clip_content], [390, 844, true]);
    assert.equal(created.parent, presetDocument.pages[0].id);
    assert.equal(Object.keys(presetDocument.nodes).length, Object.keys(beforePreset.nodes).length + 1);
    await history(beforePreset, presetDocument, presetDepth, "Native frame preset");
    check("The native frame preset picker creates a selected, clipped and editable frame on the active page, fits it in view, and supports complete undo/redo");
}
