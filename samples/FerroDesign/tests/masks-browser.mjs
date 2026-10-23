import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Exercise native compositing controls, the real canvas picker and PNG export.
// The PNG checks inspect rendered coverage, not a second implementation of masks.
export async function maskAuthoringRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeFrame,
        captureNativeScreenshot } = helpers;
    const fill = (r, g, b, a = 255) => ({ paint: { type: "solid", r, g, b, a }, opacity: 1, visible: true });
    const rectangle = (id, properties = {}) => fixtureNode(reference, id, {
        kind: "rectangle", x: 0, y: 0, width: 128, height: 128,
        corner_radius: 0, corner_radii: null, blend_mode: "normal",
        is_mask: false, mask_type: "alpha", ...properties,
    });
    const group = (id, properties = {}) => rectangle(id, { kind: "group", fills: [], ...properties });
    const click = (name) => clickNative(page, name, "ModalViewport");
    const done = () => clickNative(page, "CompositingDone");
    const open = async () => {
        await clickNative(page, "CompositingButton", "InspectorViewport");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.chrome.CompositingMaskType?.visible && view.chrome.CompositingMaskType?.arrangeValid &&
                view.chrome.CompositingBlend?.choice;
        });
    };
    const choose = (name, index) => chooseNative(page, name, { index, scrollContainer: "ModalViewport" });
    const focusCanvas = async () => {
        // Fitting the 128px document puts world (160,160) outside the
        // application viewport. The native Move tool restores the canvas's
        // actual keyboard focus without relying on an offscreen blank point.
        await clickNative(page, "SelectTool");
        assert.equal((await state(page)).chrome.CanvasHost.focusWithin, true,
            "The native canvas must own selection and history shortcuts");
    };
    const clearSelection = async () => {
        await focusCanvas();
        const previousFrame = (await state(page)).renderedFrames;
        await page.keyboard.press("Escape");
        await waitForNativeFrame(page, previousFrame, "Clear export selection");
        assert.deepEqual((await state(page)).selection, []);
    };
    const waitNode = (id, property, value) => page.waitForFunction(({ id, property, value }) =>
        JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[id]?.[property] === value,
    { id, property, value });
    const exactHistory = async (before, after, label, close = true) => {
        const depth = (await state(page)).historyLength;
        if (close) await done();
        else await focusCanvas();
        let previousFrame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, previousFrame, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " undoes the entire document in one step");
        assert.equal((await state(page)).historyLength, depth - 1);
        previousFrame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, previousFrame, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " redoes the entire document in one step");
        assert.equal((await state(page)).historyLength, depth);
    };
    const selectAll = async () => {
        await focusCanvas();
        await page.keyboard.press("Control+a");
        await frames(page);
        const document = await documentState(page);
        assert.deepEqual([...(await state(page)).selection].sort((a, b) => a - b),
            [...document.pages[0].children].sort((a, b) => a - b));
    };
    const exported = async (suffix) => {
        await selectAll();
        const [download] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, "ExportPngButton", "InspectorViewport"),
        ]);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-masks-" + suffix + ".png");
        await download.saveAs(path);
        const png = PNG.sync.read(await readFile(path));
        assert.deepEqual([png.width, png.height], [128, 128]);
        const view = await state(page);
        assert.equal(view.unsupportedEffects, 0, "Every authored mask must render on this backend");
        assert.equal(view.invalidNodes, 0);
        return png;
    };
    const pixel = (png, x, y) => {
        const offset = (y * png.width + x) * 4;
        return [...png.data.subarray(offset, offset + 4)];
    };
    const near = (actual, expected, label) => assert.ok(
        actual.length === 4 && expected.every((value, i) => Math.abs(value - actual[i]) <= 3),
        label + ": " + JSON.stringify({ actual, expected }));

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Editable green mask source", width: 64, fills: [fill(0, 255, 0, 128)] }),
        rectangle(3, { name: "Red masked content", x: 32, width: 96, fills: [fill(255, 0, 0)] }),
    ], "Native mask authoring"));
    await clickWorld(page, 16, 64);
    assert.deepEqual((await state(page)).selection, [2]);
    const initial = await documentState(page);
    await open();
    await click("CompositingMaskEnabled");
    await waitNode(2, "is_mask", true);
    const alphaDocument = await documentState(page);
    assert.equal(alphaDocument.nodes[2].mask_type ?? "alpha", "alpha");
    await exactHistory(initial, alphaDocument, "Use as mask");
    check("The native Use as mask control creates a sibling mask with exact one-step document undo/redo");

    let png = await exported("alpha");
    near(pixel(png, 48, 64), [255, 0, 0, 128], "Alpha mask uses source transparency");
    near(pixel(png, 16, 64), [0, 0, 0, 0], "The source does not paint its own green color");
    near(pixel(png, 100, 64), [0, 0, 0, 0], "Content outside source coverage is transparent");
    for (const [index, kind, alpha] of [[1, "vector", 255], [2, "luminance", 92]]) {
        // A click inside an existing multiselection intentionally retains all
        // members for a group move; finish that export selection first.
        await clearSelection();
        await clickWorld(page, 16, 64);
        assert.deepEqual((await state(page)).selection, [2], "An unpainted mask source stays independently selectable");
        const before = await documentState(page);
        await open();
        await choose("CompositingMaskType", index);
        await waitNode(2, "mask_type", kind);
        const after = await documentState(page);
        await exactHistory(before, after, "Native " + kind + " mask type");
        png = await exported(kind);
        near(pixel(png, 48, 64), [255, 0, 0, alpha], kind + " produces its own rendered coverage");
        near(pixel(png, 100, 64), [0, 0, 0, 0], kind + " does not expose uncovered content");
    }
    check("Native mask-type ComboBoxes change real PNG alpha from 128 to 255 to 92 for alpha/vector/luminance, with exact undo/redo for each type");

    const masked = await documentState(page);
    await clickWorld(page, 100, 64);
    assert.deepEqual((await state(page)).selection, [], "Transparent masked content must not intercept a native canvas click");
    await clickWorld(page, 48, 64);
    assert.deepEqual((await state(page)).selection, [3], "Visible masked content remains selectable");
    await clickWorld(page, 16, 64);
    assert.deepEqual((await state(page)).selection, [2]);
    await open();
    await captureNativeScreenshot(mode + "-native-mask-controls", {
        controls: ["CanvasHost", "CompositingMaskEnabled", "CompositingMaskType", "CompositingBlend"],
    });
    await click("CompositingMaskEnabled");
    await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].is_mask);
    const disabled = await documentState(page);
    assert.equal(disabled.nodes[2].mask_type, "luminance", "Disabling a mask preserves its authored type");
    await exactHistory(masked, disabled, "Remove sibling mask");
    await clickWorld(page, 100, 64);
    assert.deepEqual((await state(page)).selection, [3], "Removing the mask refreshes native picking immediately");
    const undoFrame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+z");
    await waitForNativeFrame(page, undoFrame, "Restore native mask coverage");
    assert.deepEqual(await documentState(page), masked);
    await clickWorld(page, 100, 64);
    assert.deepEqual((await state(page)).selection, [], "Undo restores the mask's picking scope before another authoring gesture");
    check("Native canvas picking rejects transparent masked content, retains editable sources, and refreshes coverage after a mask edit and undo");

    await selectAll();
    const beforeGroup = await documentState(page);
    await open();
    await clickNative(page, "CompositingCreateGroup");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return !view.chrome.ModalBackdrop.visible && view.nodeCount === 3 && view.selectionCount === 1;
    });
    const grouped = await documentState(page);
    const groupId = (await state(page)).selection[0];
    assert.equal(grouped.nodes[groupId].kind, "group");
    assert.equal(grouped.nodes[groupId].blend_mode, "pass_through");
    assert.deepEqual(grouped.nodes[groupId].children, [2, 3]);
    assert.deepEqual(grouped.pages[0].children, [groupId]);
    assert.equal(grouped.nodes[2].is_mask, true);
    assert.equal(grouped.nodes[2].mask_type ?? "alpha", "alpha");
    assert.equal(grouped.nodes[2].parent, groupId);
    assert.equal(grouped.nodes[3].parent, groupId);
    for (const id of [2, 3]) {
        assert.equal(grouped.nodes[groupId].x + grouped.nodes[id].x, beforeGroup.nodes[id].x);
        assert.equal(grouped.nodes[groupId].y + grouped.nodes[id].y, beforeGroup.nodes[id].y);
        assert.deepEqual(grouped.nodes[id].fills, beforeGroup.nodes[id].fills);
    }
    await exactHistory(beforeGroup, grouped, "Create mask group", false);
    png = await exported("grouped-alpha");
    near(pixel(png, 48, 64), [255, 0, 0, 128], "The native mask group retains the same alpha coverage");
    near(pixel(png, 100, 64), [0, 0, 0, 0], "The contained mask preserves transparent output");
    check("Create mask group wraps the ordered source/content with preserved positions and paints, contains its scope and records one complete undo/redo transaction");

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Authored blue backdrop", fills: [fill(128, 128, 255)] }),
        group(3, { name: "Half-opacity blend group", opacity: 0.5, children: [4, 5] }),
        rectangle(4, { parent: 3, width: 70, fills: [fill(255, 0, 0)], blend_mode: "multiply" }),
        rectangle(5, { parent: 3, x: 40, width: 70, fills: [fill(0, 0, 255)], blend_mode: "multiply" }),
    ], "Native pass-through compositing"));
    png = await exported("isolated-group");
    near(pixel(png, 20, 64), [192, 64, 128, 255], "Normal group isolates its child blend from the authored backdrop");
    await clearSelection();
    await clickWorld(page, 120, 64);
    assert.deepEqual((await state(page)).selection, [3]);
    const isolated = await documentState(page);
    await open();
    await choose("CompositingBlend", 16);
    await waitNode(3, "blend_mode", "pass_through");
    const passed = await documentState(page);
    assert.equal(passed.nodes[3].opacity, 0.5);
    assert.deepEqual(passed.nodes[4], isolated.nodes[4]);
    assert.deepEqual(passed.nodes[5], isolated.nodes[5]);
    await exactHistory(isolated, passed, "Pass through group blend");
    png = await exported("pass-through-group");
    near(pixel(png, 20, 64), [128, 64, 128, 255], "Pass-through child blend uses the authored backdrop");
    near(pixel(png, 50, 64), [64, 64, 128, 255], "Overlapping child blends receive completed group opacity once");
    near(pixel(png, 90, 64), [64, 64, 255, 255], "A separate child uses the same completed group opacity");
    near(pixel(png, 120, 64), [128, 128, 255, 255], "Untouched authored backdrop pixels remain unchanged");
    check("The native Pass through option changes child/backdrop blend pixels while half-opacity applies to the completed group once, with exact one-step undo/redo");
}
