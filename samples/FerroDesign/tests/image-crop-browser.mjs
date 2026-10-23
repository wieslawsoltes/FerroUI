import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Real FerroUI controls, pointer capture, DrawingContext image brushes and PNG
// downloads. This helper never launches a browser or substitutes a DOM editor.
export async function imageCropRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, moveWorld, frames, captureNativeScreenshot } = helpers;
    const colors = [[255, 20, 30], [255, 200, 0], [0, 90, 255], [0, 200, 90]];
    const image = new PNG({ width: 100, height: 100 });
    for (let y = 0; y < 100; y++) for (let x = 0; x < 100; x++) {
        const offset = (y * 100 + x) * 4;
        const color = colors[Math.floor(y / 25)];
        image.data.set([...color, 255], offset);
    }
    const href = "data:image/png;base64," + PNG.sync.write(image).toString("base64");
    const photo = (extra = {}) => fixtureNode(reference, 2, {
        kind: "rectangle", name: "Image source window", x: 40, y: 60, width: 200, height: 100,
        corner_radius: 0, corner_radii: null, blend_mode: "normal",
        fills: [{ paint: { type: "image", href, scale: "fill" }, opacity: 1, visible: true }],
        ...extra,
    });
    const command = async (name) => {
        assert.equal(await page.evaluate((name) => globalThis.ferroDesign.ferroDesignCommand(name), name), true);
        await frames(page);
    };
    const active = () => page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).imageCrop?.[0] === 2);
    const openCrop = async () => {
        await command("paints");
        await clickNative(page, "PaintCropCanvas0", "ModalViewport");
        await active();
        await frames(page);
    };
    const drag = async (from, to, release = true) => {
        const view = await state(page);
        const start = await moveWorld(page, from.x, from.y);
        await page.mouse.down();
        await page.mouse.move(start.x + (to.x - from.x) * view.zoom,
            start.y + (to.y - from.y) * view.zoom, { steps: 7 });
        if (release) await page.mouse.up();
        await frames(page);
        return start;
    };
    const near = (actual, expected, message, epsilon = 1e-5) => assert.ok(Math.abs(actual - expected) <= epsilon,
        message + ": expected " + expected + ", got " + actual);
    const cropIs = (node, expected) => {
        const crop = node.fills[0].paint.crop;
        assert.ok(crop, "The source window must be serialized in the editable image paint");
        for (const key of ["x", "y", "width", "height"]) near(crop[key], expected[key], "Source " + key);
        assert.equal(node.fills[0].paint.href, href, "Cropping must preserve the original embedded bytes");
    };
    const exportPng = async (suffix) => {
        const [download, exported] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("export-png")),
        ]);
        assert.equal(exported, true);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-image-crop-" + suffix + ".png");
        await download.saveAs(path);
        return PNG.sync.read(await readFile(path));
    };
    const pixel = (png, x, y) => [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    const colorIs = (png, x, y, color, label) => {
        const value = pixel(png, x, y);
        assert.ok(color.every((channel, index) => Math.abs(channel - value[index]) <= 8) && value[3] >= 247,
            label + ": " + JSON.stringify(value));
    };
    const historyLength = async () => {
        const length = (await state(page)).historyLength;
        assert.ok(Number.isSafeInteger(length) && length >= 0, "The native editor must report its actual undo depth");
        return length;
    };
    const committedHistory = (before, after, label) => {
        // Imports are undoable and this long suite can fill the bounded history.
        // Appending then evicts the oldest entry; undo below must still remove
        // exactly one entry and restore the complete pre-gesture document.
        assert.ok(after > 0 && (after === before + 1 || after === before),
            label + ": unexpected undo depth " + before + " -> " + after);
    };

    await importFixture(page, isolatedDocument(reference, [photo()], "Native image panning"));
    await clickWorld(page, 140, 110);
    const authored = await documentState(page);
    const authoredHistory = await historyLength();
    await openCrop();
    // A handle click must not normalize an implicit cover window into a new
    // crop, add overrides, or produce an undo entry.
    await clickWorld(page, 40, 60);
    await frames(page);
    assert.deepEqual(await documentState(page), authored);
    assert.equal(await historyLength(), authoredHistory, "A crop handle click must preserve imported history");
    await drag({ x: 140, y: 110 }, { x: 140, y: 140 });
    const panned = await documentState(page);
    const pannedHistory = await historyLength();
    committedHistory(authoredHistory, pannedHistory, "One pan gesture");
    cropIs(panned.nodes[2], { x: 0, y: 0.1, width: 1, height: 0.5 });
    assert.equal(panned.nodes[2].fills[0].paint.scale, "fill");
    for (const property of ["x", "y", "width", "height", "rotation", "affine_transform"]) {
        assert.deepEqual(panned.nodes[2][property], authored.nodes[2][property], "Panning preserves layer " + property);
    }
    await captureNativeScreenshot(mode + "-image-crop-pan-native", {
        controls: ["CanvasHost", "CropDoneButton"],
    });
    const rendered = await exportPng("pan-export");
    assert.equal(rendered.width, 200);
    assert.equal(rendered.height, 100);
    colorIs(rendered, 100, 10, colors[0], "Native SourceRect reveals the upper red image stripe");
    colorIs(rendered, 100, 50, colors[1], "Native SourceRect retains the middle yellow image stripe");
    colorIs(rendered, 100, 95, colors[2], "Native SourceRect retains the lower blue image stripe");
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), authored, "One undo restores every update in the image pan gesture");
    assert.equal(await historyLength(), pannedHistory - 1, "One undo removes only the pan transaction");
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), panned);
    assert.equal(await historyLength(), pannedHistory, "Redo restores the pan transaction");
    check("Native image panning resolves Fill aspect, stores normalized UVs without changing bytes or bounds, produces verified PNG pixels, and restores one-step history");

    await importFixture(page, isolatedDocument(reference, [photo()], "Canceled image crop"));
    await clickWorld(page, 140, 110);
    const original = await documentState(page);
    const originalHistory = await historyLength();
    await openCrop();
    let start = await drag({ x: 40, y: 60 }, { x: 80, y: 80 }, false);
    assert.equal((await state(page)).imageCrop[2], true);
    await page.keyboard.press("Escape");
    await page.mouse.move(start.x + 4, start.y + 4);
    await page.mouse.up();
    await frames(page);
    assert.deepEqual(await documentState(page), original, "Escape cancels geometry, UVs and override metadata; later pointer events cannot resume it");
    assert.equal(await historyLength(), originalHistory, "Canceled crop leaves imported history intact");
    assert.equal((await state(page)).imageCrop[2], false);
    // Move away and back under the same capture. Exact history must remain
    // unchanged, including implicit Fill crop and authored override metadata.
    start = await drag({ x: 40, y: 60 }, { x: 75, y: 85 }, false);
    await page.mouse.move(start.x, start.y, { steps: 7 });
    await page.mouse.up();
    await frames(page);
    assert.deepEqual(await documentState(page), original);
    assert.equal(await historyLength(), originalHistory, "Dragging back to the original window adds no undo entry");
    await drag({ x: 40, y: 60 }, { x: 80, y: 80 }, false);
    await page.keyboard.press("Control+z");
    await page.mouse.up();
    await frames(page);
    assert.deepEqual(await documentState(page), original, "Undo during capture cancels the gesture before history traversal");
    assert.equal(await historyLength(), originalHistory, "Undo during crop capture cancels without traversing prior history");
    await page.keyboard.press("Escape");
    assert.equal((await state(page)).imageCrop, null);
    check("Native crop Escape, drag-to-origin, and Undo during capture restore the exact authored document and ignore late pointer events");

    const affine = photo({
        width: 160, height: 100,
        affine_transform: { a: 1.1, b: 0.2, c: 0.25, d: 0.85, e: 0, f: 0 },
        fills: [{ paint: { type: "image", href, scale: "stretch" }, opacity: 1, visible: true }],
    });
    await importFixture(page, isolatedDocument(reference, [affine], "Affine image corner crop"));
    const before = (await documentState(page)).nodes[2];
    const beforeAffineHistory = await historyLength();
    const transform = nodeTransform(before);
    const inside = transformPoint(transform, { x: 80, y: 50 });
    await clickWorld(page, inside.x, inside.y);
    await openCrop();
    const corner = transformPoint(transform, { x: 0, y: 0 });
    const destination = transformPoint(transform, { x: 32, y: 20 });
    await drag(corner, destination);
    const resized = (await documentState(page)).nodes[2];
    const affineHistory = await historyLength();
    committedHistory(beforeAffineHistory, affineHistory, "One affine crop gesture");
    cropIs(resized, { x: 0.2, y: 0.2, width: 0.8, height: 0.8 });
    near(resized.width, 128, "Cropped layer width");
    near(resized.height, 80, "Cropped layer height");
    assert.equal(resized.fills[0].paint.scale, "stretch", "Existing Stretch keeps its authored pixel mapping");
    const oldPixel = transformPoint(transform, { x: 120, y: 75 });
    const newPixel = transformPoint(nodeTransform(resized), { x: 88, y: 55 });
    near(newPixel.x, oldPixel.x, "Affine source pixel X remains fixed");
    near(newPixel.y, oldPixel.y, "Affine source pixel Y remains fixed");
    await captureNativeScreenshot(mode + "-image-crop-affine-native", {
        controls: ["CanvasHost", "CropDoneButton"],
    });
    await clickNative(page, "CropDoneButton", "InspectorViewport");
    assert.equal((await state(page)).imageCrop, null);
    await clickWorld(page, oldPixel.x, oldPixel.y);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual((await documentState(page)).nodes[2], before);
    assert.equal(await historyLength(), affineHistory - 1);
    check("Actual native crop corner dragging preserves retained source pixels through affine shear, changes the window bounds, and supports the native Done control and exact undo");

    const fit = photo({ fills: [{ paint: { type: "image", href, scale: "fit" }, opacity: 1, visible: true }] });
    await importFixture(page, isolatedDocument(reference, [fit], "Native numeric image windows"));
    await clickWorld(page, 140, 110);
    const beforeNumeric = await documentState(page);
    const beforeNumericHistory = await historyLength();
    await command("paints");
    const field = async (name, value) => {
        await clickNative(page, name, "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await frames(page);
    };
    for (const [suffix, value] of [["X", "10"], ["Y", "20"], ["Width", "80"], ["Height", "60"]]) {
        await field("PaintCrop" + suffix + "0", value);
    }
    assert.deepEqual(await documentState(page), beforeNumeric, "Source-window fields stage an atomic rectangle until Apply");
    assert.equal(await historyLength(), beforeNumericHistory, "Staged source-window fields add no history");
    await clickNative(page, "PaintCropApply0", "ModalViewport");
    const numeric = await documentState(page);
    const numericHistory = await historyLength();
    committedHistory(beforeNumericHistory, numericHistory, "Applying one source window");
    cropIs(numeric.nodes[2], { x: 0.1, y: 0.2, width: 0.8, height: 0.6 });
    assert.equal(numeric.nodes[2].fills[0].paint.scale, "fit");
    await field("PaintCropWidth0", "95");
    await clickNative(page, "PaintCropApply0", "ModalViewport");
    assert.deepEqual(await documentState(page), numeric, "An out-of-bounds source window is rejected atomically");
    assert.equal(await historyLength(), numericHistory, "Rejected source-window fields add no history");
    assert.equal((await state(page)).chrome.PaintCropError0.visible, true);
    await captureNativeScreenshot(mode + "-image-crop-percentages-native", {
        controls: ["ModalContent", "PaintCropApply0", "PaintCropError0"],
    });
    await clickNative(page, "PaintDone");
    const fitPixels = await exportPng("fit-export");
    assert.equal(fitPixels.width, 200);
    assert.equal(fitPixels.height, 100);
    assert.ok(pixel(fitPixels, 10, 50)[3] <= 8, "Fit retains transparent letterboxing around the source window");
    colorIs(fitPixels, 100, 75, colors[2], "Fit source-window pixels use the authored crop at native aspect");
    await command("paints");
    await clickNative(page, "PaintCropReset0", "ModalViewport");
    const reset = await documentState(page);
    const resetHistory = await historyLength();
    committedHistory(numericHistory, resetHistory, "Resetting the source window");
    assert.equal(reset.nodes[2].fills[0].paint.crop, undefined);
    assert.equal(reset.nodes[2].fills[0].paint.scale, "fit");
    assert.equal(reset.nodes[2].fills[0].paint.href, href);
    await clickNative(page, "PaintDone");
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), numeric, "Reset source is a single undoable paint edit");
    assert.equal(await historyLength(), resetHistory - 1);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeNumeric, "Rejected source fields added no history between Apply and Reset");
    assert.equal(await historyLength(), resetHistory - 2);
    check("Native percentage fields atomically apply and validate source windows, retain Fit letterboxing in actual PNG output, reset embedded source bytes without conversion, and preserve undo ordering");

    const tiled = photo({ fills: [{ paint: { type: "image", href, scale: "tile",
        crop: { x: 0.1, y: 0.25, width: 0.5, height: 0.25 } }, opacity: 1, visible: true }] });
    await importFixture(page, isolatedDocument(reference, [tiled], "Cropped image tiles"));
    await clickWorld(page, 140, 110);
    const tilePixels = await exportPng("tile-export");
    for (const [x, y] of [[10, 10], [90, 60], [190, 85]]) {
        colorIs(tilePixels, x, y, colors[1], "The cropped source tiles continuously without transparent gaps");
    }
    check("Native Tile source windows repeat at the cropped raster size with verified opaque pixels across the layer");
}

function multiply(a, b) {
    return {
        a: a.a * b.a + a.c * b.b, b: a.b * b.a + a.d * b.b,
        c: a.a * b.c + a.c * b.d, d: a.b * b.c + a.d * b.d,
        e: a.a * b.e + a.c * b.f + a.e, f: a.b * b.e + a.d * b.f + a.f,
    };
}
function transformPoint(matrix, point) {
    return { x: matrix.a * point.x + matrix.c * point.y + matrix.e, y: matrix.b * point.x + matrix.d * point.y + matrix.f };
}
function nodeTransform(node) {
    const identity = { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
    const translation = (e, f) => ({ ...identity, e, f });
    const radians = node.rotation * Math.PI / 180;
    const rotation = { a: Math.cos(radians), b: Math.sin(radians), c: -Math.sin(radians), d: Math.cos(radians), e: 0, f: 0 };
    return [translation(node.x + node.width / 2, node.y + node.height / 2), rotation,
        { ...identity, a: node.flip_x ? -1 : 1, d: node.flip_y ? -1 : 1 }, translation(-node.width / 2, -node.height / 2),
        node.affine_transform || identity].reduce(multiply);
}
