import assert from "node:assert/strict";

// Real native controls and pointer gestures. Fixtures only establish starting
// documents; every binding, mode change, constraint and gesture is authored UI.
export async function guideBindingRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, moveWorld, frames, waitForNativeFrame,
        waitForNativeScene, captureNativeScreenshot } = helpers;
    const near = (actual, expected, label) => assert.ok(Math.abs(actual - expected) < 1e-7,
        `${label}: ${actual} != ${expected}`);
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("document", "args", "return (" + source + ")(document,args)")(document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const ready = name => page.waitForFunction(name => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.chrome.ModalBackdrop.visible && view.chrome[name]?.visible;
    }, name, { timeout: 10000 });
    const select = async (id, x, y) => {
        await clickNative(page, "SelectTool");
        const before = await state(page);
        if (before.selection.length) {
            await page.keyboard.press("Escape");
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection.length === 0,
                undefined, { timeout: 10000 });
            await waitForNativeFrame(page, before.renderedFrames, "Guide selection clear");
            await waitForNativeScene(page, before.sceneFrames, "Guide selection clear");
            const cleared = await state(page);
            assert.equal(cleared.chrome.PageInspector.visible, true);
            assert.equal(cleared.chrome.DesignInspector.visible, false);
        }
        await page.keyboard.down("Control");
        try { await clickWorld(page, x, y); } finally { await page.keyboard.up("Control"); }
        assert.deepEqual((await state(page)).selection, [id]);
    };
    const edit = async (name, value, scrollContainer = "ModalViewport") => {
        await clickNative(page, name, scrollContainer);
        assert.equal((await state(page)).chrome[name].focused, true);
        await page.keyboard.press("Control+a"); await page.keyboard.insertText(value);
        await page.keyboard.press("Enter"); await frames(page);
    };
    const choice = (name, index, scrollContainer = "ModalViewport") => chooseNative(page, name, { index, scrollContainer });
    const openGuides = async () => {
        await clickNative(page, "LayoutGridSettingsButton", "InspectorViewport");
        await ready("LayoutGridSize");
    };
    const openBinding = async (index, property) => {
        await clickNative(page, "LayoutGridVariables" + (index || ""), "ModalViewport");
        await ready("GuideVariableProperty");
        if ((await state(page)).chrome.GuideVariableProperty.choice.selectedIndex !== property) {
            await choice("GuideVariableProperty", property);
        }
    };
    const closeBinding = async () => {
        await clickNative(page, "GuideVariableBack", "ModalActions");
        await ready("LayoutGridSize");
        await clickNative(page, "LayoutGridDone", "ModalActions");
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let view = await state(page);
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        await waitForNativeScene(page, view.sceneFrames, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " exact undo");
        view = await state(page);
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        await waitForNativeScene(page, view.sceneFrames, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " exact redo");
    };
    const columns = { size: 20, visible: false, color: { r: 239, g: 68, b: 68, a: 64 },
        pattern: { type: "columns", alignment: "stretch", count: 3, gutter: 10, offset: 10 } };
    const square = { size: 8, visible: false, color: { r: 239, g: 68, b: 68, a: 64 } };
    const fixture = isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "frame", name: "Variable guide stack", x: 40, y: 40, width: 300, height: 220,
        corner_radius: 0, corner_radii: null, layout_grid: columns, layout_grid_overlays: [square], grid_style: null,
    })], "Native guide bindings");
    fixture.variable_mode = "Default";
    fixture.variables = {
        100: { id: 100, name: "Columns", collection: "Guides", values: {
            Default: { type: "number", value: 4 }, Wide: { type: "number", value: 6 } } },
        101: { id: 101, name: "Spacing", collection: "Guides", values: {
            Default: { type: "number", value: 18 }, Wide: { type: "number", value: 22 } } },
        102: { id: 102, name: "Guide color", collection: "Guides", values: {
            Default: { type: "color", value: { r: 22, g: 163, b: 74, a: 128 } },
            Wide: { type: "color", value: { r: 37, g: 99, b: 235, a: 192 } } } },
        103: { id: 103, name: "Fractional count", collection: "Guides", values: {
            Default: { type: "number", value: 2.5 }, Wide: { type: "number", value: 2.5 } } },
    };
    fixture.next_id = 104;
    await importFixture(page, fixture);
    await select(2, 80, 80);
    await openGuides(); await openBinding(0, 1);
    await choice("GuideVariableValue", 1);
    let before = await documentState(page), depth = (await state(page)).historyLength;
    await clickNative(page, "GuideVariableApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].variable_bindings["layout_grid.0.count"] === 100 && doc.nodes[2].layout_grid.pattern.count === 4);
    let after = await documentState(page);
    await closeBinding(); await history(before, after, depth, "Guide count binding");
    await openGuides(); await openBinding(0, 4);
    await choice("GuideVariableValue", 1);
    before = await documentState(page); depth = (await state(page)).historyLength;
    await clickNative(page, "GuideVariableApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].variable_bindings["layout_grid.0.color"] === 102);
    after = await documentState(page);
    assert.deepEqual(after.nodes[2].layout_grid.color, fixture.variables[102].values.Default.value);
    await captureNativeScreenshot(mode + "-guide-variable-picker", { controls: ["ModalPanel", "GuideVariableProperty", "GuideVariableValue", "GuideVariableApply"] });
    await closeBinding(); await history(before, after, depth, "Guide color binding");
    check("Native guide-variable pickers bind numeric count and alpha color through typed controls, resolve the actual stack values and preserve exact single-step history");

    await openGuides(); await openBinding(0, 1);
    await choice("GuideVariableValue", 3);
    before = await documentState(page); depth = (await state(page)).historyLength;
    await clickNative(page, "GuideVariableApply", "ModalActions");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.GuideVariableError.visible,
        undefined, { timeout: 10000 });
    assert.deepEqual(await documentState(page), before);
    assert.equal((await state(page)).historyLength, depth);
    await clickNative(page, "GuideVariableLibrary", "ModalActions");
    await ready("VariablesMode");
    await choice("VariablesMode", 1);
    before = await documentState(page); depth = (await state(page)).historyLength;
    await clickNative(page, "VariablesActivateMode", "ModalViewport");
    await waitDocument(doc => doc.variable_mode === "Wide" && doc.nodes[2].layout_grid.pattern.count === 6);
    after = await documentState(page);
    assert.deepEqual(after.nodes[2].layout_grid.color, fixture.variables[102].values.Wide.value);
    await clickNative(page, "VariablesDone", "ModalActions");
    await history(before, after, depth, "Guide variable mode");
    await openGuides(); before = await documentState(page); depth = (await state(page)).historyLength;
    await edit("LayoutGridCount", "6");
    await waitDocument(doc => !("layout_grid.0.count" in doc.nodes[2].variable_bindings));
    after = await documentState(page);
    assert.equal(after.nodes[2].layout_grid.pattern.count, 6);
    assert.equal(after.nodes[2].variable_bindings["layout_grid.0.color"], 102);
    await clickNative(page, "LayoutGridDone", "ModalActions");
    await history(before, after, depth, "Equal-value literal guide override");
    check("Invalid fractional count bindings are rejected atomically; native mode activation updates bound geometry/color together, and explicit Enter on an equal literal detaches only that field");

    await openGuides(); await openBinding(1, 0);
    await choice("GuideVariableValue", 2);
    await clickNative(page, "GuideVariableApply", "ModalActions");
    await waitDocument(doc => doc.nodes[2].variable_bindings["layout_grid.1.size"] === 101);
    await clickNative(page, "GuideVariableBack", "ModalActions");
    await ready("LayoutGridSize");
    before = await documentState(page); depth = (await state(page)).historyLength;
    await clickNative(page, "LayoutGridDown", "ModalViewport");
    await waitDocument(doc => doc.nodes[2].variable_bindings["layout_grid.1.color"] === 102 && doc.nodes[2].variable_bindings["layout_grid.0.size"] === 101);
    after = await documentState(page);
    assert.equal(after.nodes[2].layout_grid.size, 22);
    assert.equal(after.nodes[2].layout_grid_overlays[0].pattern.type, "columns");
    assert.equal(after.nodes[2].variable_bindings["layout_grid.0.color"], undefined);
    assert.equal(after.nodes[2].variable_bindings["layout_grid.1.size"], undefined);
    await clickNative(page, "LayoutGridDone", "ModalActions");
    await history(before, after, depth, "Guide reorder with variables");
    await importFixture(page, after);
    assert.deepEqual(await documentState(page), after);
    check("Moving a guide remaps its variable keys with the complete stack entry, retains each resolved value and round-trips every binding through native JSON and exact undo/redo");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Responsive guide frame", x: 40, y: 40,
            width: 300, height: 200, corner_radius: 0, corner_radii: null, fills: [], children: [3],
            layout_grid: columns, layout_grid_overlays: [], grid_style: null }),
        fixtureNode(reference, 3, { parent: 2, name: "Column constrained", x: 110, y: 40,
            width: 20, height: 20, corner_radius: 0, corner_radii: null,
            constraints: { horizontal: "start", vertical: "start" } }),
    ], "Guide-relative child constraints"));
    await select(3, 160, 90);
    await choice("HorizontalConstraint", 3, "InspectorViewport");
    await waitDocument(doc => doc.nodes[3].constraints.horizontal === "stretch");
    await select(2, 65, 65);
    before = await documentState(page); depth = (await state(page)).historyLength;
    await edit("WidthField", "600", "InspectorViewport");
    await waitDocument(doc => doc.nodes[2].width === 600);
    after = await documentState(page);
    near(after.nodes[3].x, 210, "Hidden stretch column reference x");
    near(after.nodes[3].width, 120, "Hidden stretch column reference width");
    assert.deepEqual([after.nodes[3].y, after.nodes[3].height], [40, 20]);
    await clickWorld(page, 65, 65);
    await history(before, after, depth, "Guide-relative frame resize");
    await openGuides(); await choice("LayoutGridAlignment", 0);
    await clickNative(page, "LayoutGridDone", "ModalActions");
    await edit("WidthField", "700", "InspectorViewport");
    await waitDocument(doc => doc.nodes[2].width === 700);
    const fixed = await documentState(page);
    near(fixed.nodes[3].x, 210, "Fixed columns retain frame-relative x");
    near(fixed.nodes[3].width, 220, "Fixed columns use frame-relative stretch");
    check("Native child constraints resize relative to hidden stretch guide sections; switching the same guide to a fixed alignment restores ordinary frame-relative resizing without changing hierarchy");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Hidden snap reference", x: 40, y: 40,
            width: 300, height: 220, corner_radius: 0, corner_radii: null, fills: [], children: [3],
            layout_grid: { ...square, size: 25.5 }, layout_grid_overlays: [], grid_style: null }),
        fixtureNode(reference, 3, { parent: 2, name: "Snapped child", x: 36, y: 43, width: 23, height: 17,
            corner_radius: 0, corner_radii: null }),
    ], "Native guide pointer snapping"));
    // Import fits this small frame at almost 3x. The native attraction radius
    // is five screen pixels; the move below needs a four-world-unit correction.
    // Use the actual zoom control so the requested move, resize and draw inputs
    // are within that radius, preserving the exact fractional geometry checks.
    const beforeSnapZoom = await documentState(page);
    const snapZoomHistory = (await state(page)).historyLength;
    for (let attempt = 0; (await state(page)).zoom > 1.25 && attempt < 16; attempt++) {
        const previous = await state(page);
        await clickNative(page, "ZoomOutButton");
        await waitForNativeFrame(page, previous.renderedFrames, "Guide snapping zoom");
        assert.ok((await state(page)).zoom < previous.zoom, "The native zoom control must reduce the actual scale");
    }
    const snapCamera = await state(page);
    assert.ok(Number.isFinite(snapCamera.zoom) && snapCamera.zoom > 0 && 4 * snapCamera.zoom <= 5,
        "The four-unit guide attraction must be inside the five-screen-pixel radius: " + snapCamera.zoom);
    assert.equal(snapCamera.historyLength, snapZoomHistory, "Camera changes preserve guide history");
    assert.deepEqual(await documentState(page), beforeSnapZoom, "Camera changes preserve every authored guide field");
    await select(3, 87.5, 91.5);
    before = await documentState(page); depth = (await state(page)).historyLength;
    await moveWorld(page, 87.5, 91.5); await page.mouse.down();
    await moveWorld(page, 98.5, 99.5); await page.mouse.up(); await frames(page);
    await waitDocument(doc => Math.abs(doc.nodes[3].x - 51) < 1e-7 && Math.abs(doc.nodes[3].y - 51) < 1e-7);
    after = await documentState(page);
    assert.deepEqual([after.nodes[3].width, after.nodes[3].height], [23, 17]);
    await history(before, after, depth, "Guide-snapped move");
    before = await documentState(page); depth = (await state(page)).historyLength;
    // Bottom-right handle: its desired local (75,74) snaps to (76.5,76.5).
    await moveWorld(page, 114, 108); await page.mouse.down();
    await moveWorld(page, 115, 114); await page.mouse.up(); await frames(page);
    await waitDocument(doc => Math.abs(doc.nodes[3].width - 25.5) < 1e-7 && Math.abs(doc.nodes[3].height - 25.5) < 1e-7);
    after = await documentState(page);
    assert.deepEqual([after.nodes[3].x, after.nodes[3].y], [51, 51]);
    await history(before, after, depth, "Guide-snapped resize");
    await clickNative(page, "RectTool");
    before = await documentState(page); depth = (await state(page)).historyLength;
    await moveWorld(page, 141, 141); await page.mouse.down();
    await moveWorld(page, 191, 191); await page.mouse.up(); await frames(page);
    const created = (await state(page)).selection[0];
    await waitDocument((doc, id) => doc.nodes[id] && Math.abs(doc.nodes[id].width - 51) < 1e-7, created);
    after = await documentState(page);
    for (const [field, value] of Object.entries({ x: 142, y: 142, width: 51, height: 51 })) near(after.nodes[created][field], value, "Guide-snapped draw " + field);
    await history(before, after, depth, "Guide-snapped shape draw");
    await captureNativeScreenshot(mode + "-guide-snapped-artwork", { controls: ["CanvasHost"] });
    check("Actual native move, resize and shape-creation gestures snap to fractional hidden-guide positions after pixel rounding, preserve the opposite resize anchor and commit one exact undo/redo record per gesture");
}
