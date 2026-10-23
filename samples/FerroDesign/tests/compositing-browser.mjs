import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Both browser backends render these fixtures through the native canvas and
// the same public FerroUI bitmap path used by the real Export PNG command.
export async function compositingRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, chooseNative, frames, captureNativeScreenshot } = helpers;
    const fill = (r, g, b, a = 255) => ({ paint: { type: "solid", r, g, b, a }, opacity: 1, visible: true });
    const rectangle = (id, properties = {}) => fixtureNode(reference, id, {
        kind: "rectangle", x: 0, y: 0, width: 128, height: 128,
        corner_radius: 0, corner_radii: null, blend_mode: "normal", ...properties,
    });
    const group = (id, properties = {}) => rectangle(id, { kind: "group", fills: [], ...properties });
    const blur = { kind: "background_blur", color: { r: 0, g: 0, b: 0, a: 0 },
        offset: { x: 0, y: 0 }, radius: 6, spread: 0, visible: true };
    const command = async (name) => {
        assert.equal(await page.evaluate((name) => globalThis.ferroDesign.ferroDesignCommand(name), name), true);
        await frames(page);
    };
    const exported = async (suffix) => {
        await command("select-all");
        const [download, accepted] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("export-png")),
        ]);
        assert.equal(accepted, true);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-compositing-" + suffix + ".png");
        await download.saveAs(path);
        const png = PNG.sync.read(await readFile(path));
        assert.deepEqual([png.width, png.height], [128, 128]);
        assert.equal((await state(page)).unsupportedEffects, 0, "The export must apply every authored effect");
        return png;
    };
    const pixel = (png, x, y) => [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    const near = (actual, expected, label, tolerance = 3) => assert.ok(
        expected.every((channel, index) => Math.abs(channel - actual[index]) <= tolerance),
        label + ": " + JSON.stringify({ actual, expected }));

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Opaque white backdrop", fills: [fill(255, 255, 255)] }),
        group(3, { name: "One half-opacity layer", opacity: 0.5, children: [4, 5] }),
        rectangle(4, { parent: 3, width: 88, fills: [fill(255, 0, 0)] }),
        rectangle(5, { parent: 3, x: 40, width: 88, fills: [fill(0, 0, 255)] }),
    ], "Isolated group opacity"));
    let png = await exported("group-opacity");
    near(pixel(png, 20, 64), [255, 128, 128, 255], "Single child under group opacity");
    near(pixel(png, 64, 64), [128, 128, 255, 255], "Overlapping children share one opacity layer");
    near(pixel(png, 110, 64), [128, 128, 255, 255], "Overlap matches the top child's isolated result");
    check("PNG pixels prove group opacity is applied once to overlapping children through public FerroUI compositing");

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Warm backdrop", fills: [fill(200, 160, 120)] }),
        rectangle(3, { name: "Native blend mode", fills: [fill(128, 128, 255)] }),
    ], "Native blend selection"));
    await clickWorld(page, 64, 64);
    assert.deepEqual((await state(page)).selection, [3]);
    const authoredBlend = await documentState(page);
    const blendHistory = (await state(page)).historyLength;
    await chooseNative(page, "BlendModeChoice", { index: 1, scrollContainer: "InspectorViewport" });
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[3].blend_mode === "multiply");
    await frames(page);
    const nativeBlend = await documentState(page);
    assert.equal(nativeBlend.nodes[3].x, authoredBlend.nodes[3].x, "Blend popup keys must not nudge the canvas selection");
    assert.equal(nativeBlend.nodes[3].y, authoredBlend.nodes[3].y, "Blend popup keys must stay in the native ComboBox");
    png = await exported("multiply");
    near(pixel(png, 64, 64), [100, 80, 120, 255], "Native Multiply consumes the existing layer backdrop");
    const committedBlendHistory = (await state(page)).historyLength;
    assert.ok(committedBlendHistory === blendHistory + 1 || committedBlendHistory === blendHistory,
        "A native blend edit adds one history entry, allowing bounded-history eviction");
    await command("undo");
    assert.deepEqual(await documentState(page), authoredBlend, "One undo restores the native blend selection and overrides");
    assert.equal((await state(page)).historyLength, committedBlendHistory - 1);
    await command("redo");
    assert.equal((await documentState(page)).nodes[3].blend_mode, "multiply");
    assert.equal((await state(page)).historyLength, committedBlendHistory);
    check("The native blend-mode ComboBox changes real Multiply pixels and retains exact one-step undo/redo");

    const divided = [
        rectangle(2, { name: "White half", width: 64, fills: [fill(255, 255, 255)] }),
        rectangle(3, { name: "Black half", x: 64, width: 64, fills: [fill(0, 0, 0)] }),
        rectangle(4, { name: "Rounded background blur", x: 32, y: 32, width: 64, height: 64,
            corner_radius: 16, fills: [], effects: [blur] }),
    ];
    await importFixture(page, isolatedDocument(reference, divided, "Cached background blur"));
    const authoredBlur = await documentState(page);
    png = await exported("background-blur");
    const left = pixel(png, 61, 64), right = pixel(png, 66, 64);
    assert.ok(left[0] > 128 && left[0] < 255, "White transitions smoothly into the sampled black backdrop");
    assert.ok(right[0] > 0 && right[0] < 128, "Black transitions smoothly into the sampled white backdrop");
    near(left, [left[0], left[0], left[0], 255], "Blur remains neutral and opaque");
    near(right, [right[0], right[0], right[0], 255], "Blur remains neutral and opaque");
    near(pixel(png, 33, 33), [255, 255, 255, 255], "Rounded corners preserve the unfiltered prefix");
    near(pixel(png, 61, 20), [255, 255, 255, 255], "Pixels outside the blur geometry stay sharp");
    const beforePan = await state(page);
    const canvas = beforePan.chrome.CanvasHost;
    await page.mouse.move(canvas.x + canvas.width / 2, canvas.y + canvas.height / 2);
    await page.keyboard.down("Space");
    await page.mouse.down();
    await page.mouse.move(canvas.x + canvas.width / 2 + 37, canvas.y + canvas.height / 2 + 19, { steps: 5 });
    await page.mouse.up();
    await page.keyboard.up("Space");
    await frames(page);
    const panned = await state(page);
    assert.notDeepEqual(panned.pan, beforePan.pan);
    assert.ok(panned.backdropCacheHits >= 1, "Camera-only frames must reuse completed backdrop surfaces");
    assert.equal(panned.backdropRasterizations, 0);
    assert.equal(panned.unsupportedEffects, 0);
    assert.deepEqual(await documentState(page), authoredBlur, "Panning must preserve the authored document");
    await captureNativeScreenshot(mode + "-background-blur-native", { controls: ["CanvasHost"] });
    check("Real background-blur exports soften only the rounded geometry; native canvas pan reuses its cached surface without changing the document");

    await importFixture(page, isolatedDocument(reference, [
        group(2, { name: "Transparent half-opacity group", opacity: 0.5, children: [3, 4] }),
        rectangle(3, { name: "Translucent source", parent: 2, fills: [fill(255, 0, 0, 128)] }),
        rectangle(4, { name: "Transparent glass", parent: 2, x: 40, y: 40, width: 48, height: 48,
            opacity: 0.4, fills: [], effects: [blur] }),
    ], "Transparent background replacement"));
    png = await exported("transparent-backdrop");
    // PNG stores straight RGBA. The native backend's premultiplied test for
    // these pixels is [64, 0, 0, 64]; the encoded PNG must unpremultiply red.
    near(pixel(png, 64, 64), [255, 0, 0, 64], "Background replacement applies ancestor opacity once and retains alpha");
    near(pixel(png, 20, 20), [255, 0, 0, 64], "Filtered and unfiltered translucent pixels have identical alpha");
    check("Background blur preserves transparent PNG output and composes nested and own opacity without alpha inflation or host-background capture");
}
