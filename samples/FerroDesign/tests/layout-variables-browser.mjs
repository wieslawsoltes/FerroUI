import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Only the initial fixture supplies document data. All authored bindings,
// mode changes and literal edits use real native controls and pointer/keys.
export async function layoutVariableRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeFrame, waitForNativeScene,
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

    // A separate starting fixture reuses the bindings just authored with native
    // controls. Its click action exercises the real presentation player; pixels
    // establish reflow because exports deliberately expose authored state.
    const previewFixture = structuredClone(afterGap);
    previewFixture.name = "Prototype spacing reflow";
    previewFixture.pages[0].prototype_start = 2;
    previewFixture.nodes[2].fills = [{ paint: { type: "solid", r: 255, g: 255, b: 255, a: 255 }, opacity: 1, visible: true }];
    previewFixture.nodes[2].interactions = [{ trigger: "click",
        action: { type: "set_variable", variable: 100, value: { type: "number", value: 32.5 } },
        transition: { kind: "instant", duration_ms: 0, easing: "linear" }, delay_ms: 0, key: null }];
    for (const [id, color] of [[3, { r: 255, g: 0, b: 0, a: 255 }], [4, { r: 0, g: 0, b: 255, a: 255 }]]) {
        Object.assign(previewFixture.nodes[id], { corner_radius: 0, corner_radii: null, strokes: [], effects: [],
            fills: [{ paint: { type: "solid", ...color }, opacity: 1, visible: true }] });
    }
    await importFixture(page, previewFixture);
    await clickWorld(page, 44, 44);
    await wait(view => view.selection.join() === "2");
    const authored = await documentState(page), editing = await state(page);
    await flushStorage(page);
    await clickNative(page, "PresentButton");
    await wait(view => view.presentation === 2);
    await waitForNativeFrame(page, editing.renderedFrames, "Present bound layout");
    await frames(page);
    const camera = await state(page);
    const pixel = (png, view, x, y) => {
        const sx = Math.floor(view.chrome.CanvasHost.x + view.pan.x + x * view.zoom);
        const sy = Math.floor(view.chrome.CanvasHost.y + view.pan.y + y * view.zoom);
        assert.ok(sx >= 0 && sy >= 0 && sx < png.width && sy < png.height, "Prototype sample is on the actual native canvas");
        return [...png.data.subarray((sy * png.width + sx) * 4, (sy * png.width + sx) * 4 + 4)];
    };
    const sample = async suffix => PNG.sync.read(await page.screenshot({ path: resolve(output, mode + "-prototype-spacing-" + suffix + ".png") }));
    const beforePixels = await sample("before");
    assert.deepEqual(pixel(beforePixels, camera, 76.25, 68.75), [255, 0, 0, 255], "The first child starts at its authored bound padding");
    assert.deepEqual(pixel(beforePixels, camera, 92.5, 85), [255, 255, 255, 255], "The future child center starts on the white frame");
    await clickWorld(page, 340, 200);
    await waitForNativeFrame(page, camera.renderedFrames, "Reflow prototype spacing action");
    await frames(page);
    const reflowed = await state(page), afterPixels = await sample("after");
    assert.equal(reflowed.presentation, 2);
    assert.deepEqual(reflowed.pan, camera.pan); assert.equal(reflowed.zoom, camera.zoom);
    assert.deepEqual(pixel(afterPixels, reflowed, 76.25, 68.75), [255, 255, 255, 255], "Spacing reflow clears the child's previous rendered position");
    assert.deepEqual(pixel(afterPixels, reflowed, 92.5, 85), [255, 0, 0, 255], "Spacing reflow paints the child at the independently calculated new position");
    assert.deepEqual(await documentState(page), authored, "Prototype exports keep authored spacing and variable definitions");
    assert.equal(reflowed.historyLength, editing.historyLength);
    // A clean durable flush can be a no-op. Exercise the actual production
    // Save command and finished download to test serialization during preview.
    // Presentation owns keyboard chords, so this is public Save command
    // coverage, not a claim that the Ctrl+S shortcut runs during playback.
    const [download, saved] = await Promise.all([
        page.waitForEvent("download", { timeout: 30000 }),
        page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("save")),
    ]);
    assert.equal(saved, true, "The public native Save command accepts an active presentation");
    assert.equal(await download.failure(), null);
    assert.equal(download.suggestedFilename(), "FerroDesign.ferrodesign");
    const savedPath = resolve(output, mode + "-prototype-spacing-save.ferrodesign");
    await download.saveAs(savedPath);
    assert.deepEqual(JSON.parse(await readFile(savedPath, "utf8")), authored,
        "The completed native Save download contains exact authored variables, layout and document state");
    const afterSave = await state(page);
    for (const key of ["presentation", "prototypeAnimation", "prototypeOverlays", "prototypeScrollOffsets",
        "selection", "pan", "zoom", "historyLength", "canUndo", "canRedo"]) {
        assert.deepEqual(afterSave[key], reflowed[key], "Native Save preserves transient " + key);
    }
    const savedPixels = await sample("saved");
    assert.deepEqual(pixel(savedPixels, afterSave, 76.25, 68.75), [255, 255, 255, 255],
        "Native Save keeps the preview's old child position cleared");
    assert.deepEqual(pixel(savedPixels, afterSave, 92.5, 85), [255, 0, 0, 255],
        "Native Save keeps the independently verified reflowed child visible");
    assert.deepEqual(await documentState(page), authored);
    const storage = await flushStorage(page);
    assert.equal(storage.saved, true); assert.equal(storage.dirty, false);
    assert.deepEqual(await page.evaluate(async () => JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text)), authored,
        "Durable storage stays isolated from transient prototype spacing");
    await page.keyboard.press("Escape");
    await wait(view => view.presentation === null);
    assert.deepEqual(await documentState(page), authored, "Leaving the prototype restores authored layout and variables");
    assert.deepEqual((await state(page)).selection, editing.selection);
    assert.equal((await state(page)).historyLength, editing.historyLength);
    check("A real prototype click reflows spacing into independently checked pixels; the public native Save download, durable isolation, exit and history preserve the authored design and live preview");
}
