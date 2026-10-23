import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Import through the actual Ctrl+O file picker and export through native XAML
// buttons. No DOM canvas or synthetic input substitutes for the FerroUI scene.
export async function svgCompositingRegressions(page, reference, check, helpers, output, mode) {
    const { isolatedDocument, importFixture, documentState, state, clickNative,
        frames, waitForNativeFrame, captureNativeScreenshot } = helpers;
    const source = `<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128">
      <defs>
        <mask id="alpha" mask-type="alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="128" height="128">
          <rect width="32" height="128" fill="lime" opacity=".5"/>
        </mask>
        <mask id="luminance" maskUnits="userSpaceOnUse" x="0" y="0" width="128" height="128">
          <rect x="32" width="32" height="128" fill="lime" opacity=".5"/>
        </mask>
        <clipPath id="geometry">
          <path d="M64 0H96V128H64Z" fill="none" stroke="black" stroke-width="40" opacity="0"/>
        </clipPath>
      </defs>
      <rect id="alpha-content" width="128" height="128" fill="red" mask="url(#alpha)"/>
      <rect id="luma-content" width="128" height="128" fill="red" mask="url(#luminance)"/>
      <rect id="vector-content" width="128" height="128" fill="red" clip-path="url(#geometry)"/>
    </svg>`;
    const empty = () => {
        const document = isolatedDocument(reference, [], "Native SVG compositing");
        document.pages[0].background = { r: 255, g: 255, b: 255, a: 255 };
        return document;
    };
    const focusCanvas = async () => {
        const bounds = (await state(page)).chrome.CanvasHost;
        await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
        await frames(page);
    };
    const openSvg = async (svg, name) => {
        await focusCanvas();
        const before = await state(page);
        const [chooser] = await Promise.all([
            page.waitForEvent("filechooser", { timeout: 10000 }),
            page.keyboard.press("Control+o"),
        ]);
        assert.equal(chooser.isMultiple(), false);
        await chooser.setFiles({ name, mimeType: "image/svg+xml", buffer: Buffer.from(svg) });
        await page.waitForFunction((before) => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.nodeCount > before.nodeCount && view.selectionCount === 1 && view.renderedFrames > before.renderedFrames;
        }, before, { timeout: 15000 });
        await frames(page);
        const view = await state(page);
        assert.equal(view.invalidNodes, 0);
        assert.equal(view.unsupportedEffects, 0);
        assert.equal(view.missingImages, 0);
        assert.deepEqual(view.selectionBounds, { x: 0, y: 0, width: 128, height: 128 });
        return view;
    };
    const exported = async (kind, suffix) => {
        const [download] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, kind === "png" ? "ExportPngButton" : "ExportSvgButton", "InspectorViewport"),
        ]);
        assert.equal(await download.failure(), null);
        assert.match(download.suggestedFilename(), new RegExp("\\." + kind + "$", "i"));
        const path = resolve(output, mode + "-svg-compositing-" + suffix + "." + kind);
        await download.saveAs(path);
        return readFile(path);
    };
    const pixel = (png, x, y) => {
        const offset = (Math.round(y) * png.width + Math.round(x)) * 4;
        return [...png.data.subarray(offset, offset + 4)];
    };
    const near = (actual, expected, label, tolerance = 3) => assert.ok(
        actual.length === 4 && expected.every((channel, index) => Math.abs(channel - actual[index]) <= tolerance),
        label + ": " + JSON.stringify({ actual, expected }));
    const pixels = (bytes) => {
        const png = PNG.sync.read(bytes);
        assert.deepEqual([png.width, png.height], [128, 128]);
        // PNG contains straight RGBA, unlike the native premultiplied buffer.
        near(pixel(png, 16, 64), [255, 0, 0, 128], "Alpha mask retains source alpha");
        near(pixel(png, 48, 64), [255, 0, 0, 92], "Luminance uses green BT.709 coverage times alpha");
        near(pixel(png, 80, 64), [255, 0, 0, 255], "SVG clip geometry ignores fill:none, stroke and opacity:0");
        near(pixel(png, 112, 64), [0, 0, 0, 0], "No mask source or discarded stroke leaks into uncovered pixels");
        return png;
    };

    await importFixture(page, empty());
    const before = await documentState(page);
    // Opening a fixture is itself undoable and keeps the preceding scenarios'
    // transactions. Verify this import's delta against that real history.
    const historyBefore = (await state(page)).historyLength;
    await openSvg(source, "Native SVG mask fixture.svg");
    const imported = await documentState(page);
    assert.deepEqual(Object.values(imported.nodes).filter((node) => node.is_mask).map((node) => node.mask_type || "alpha").sort(),
        ["alpha", "luminance", "vector"]);
    assert.equal((await state(page)).historyLength, historyBefore + 1, "SVG import records exactly one transaction");
    await focusCanvas();
    const undoFrame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+z");
    await waitForNativeFrame(page, undoFrame, "Undo native SVG import");
    assert.deepEqual(await documentState(page), before, "One real Undo removes the whole imported mask graph");
    assert.equal((await state(page)).historyLength, historyBefore, "Undo restores the pre-import history depth");
    const redoFrame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+Shift+z");
    await waitForNativeFrame(page, redoFrame, "Redo native SVG import");
    assert.deepEqual(await documentState(page), imported, "One real Redo restores every mask, source and transform");
    assert.equal((await state(page)).historyLength, historyBefore + 1, "Redo restores exactly the SVG import transaction");
    check("Real Ctrl+O imports editable alpha/luminance/vector SVG mask scopes with exact one-step native undo/redo");

    const view = await state(page);
    await page.mouse.move(view.chrome.CanvasHost.x - 20, view.chrome.CanvasHost.y + 40);
    const screenshot = PNG.sync.read(await captureNativeScreenshot(mode + "-native-svg-masks", { controls: ["CanvasHost"] }));
    for (const [x, expected] of [[16, [255, 127, 127, 255]], [48, [255, 163, 163, 255]], [80, [255, 0, 0, 255]], [112, [255, 255, 255, 255]]]) {
        const screenX = view.chrome.CanvasHost.x + view.pan.x + x * view.zoom;
        const screenY = view.chrome.CanvasHost.y + view.pan.y + 64 * view.zoom;
        near(pixel(screenshot, screenX, screenY), expected, "Native canvas mask coverage at " + x, 4);
    }
    const originalPng = pixels(await exported("png", "original"));
    assert.deepEqual(await documentState(page), imported, "Native screenshots and PNG export leave the document unchanged");
    check("Actual native canvas and PNG download pixels distinguish alpha, luminance and geometry clipping with transparent uncovered output");

    const svg = (await exported("svg", "roundtrip")).toString("utf8");
    const definitions = await page.evaluate((svg) => {
        const xml = new DOMParser().parseFromString(svg, "image/svg+xml");
        if (xml.querySelector("parsererror")) throw new Error("Exported SVG is invalid XML");
        return [...xml.querySelectorAll("mask[id^='layer-mask-']")].map((node) => ({
            type: node.getAttribute("mask-type"), units: node.getAttribute("maskUnits"),
            contentUnits: node.getAttribute("maskContentUnits"), interpolation: node.getAttribute("color-interpolation"),
            vector: node.getAttribute("data-ferro-mask-type"),
        }));
    }, svg);
    assert.equal(definitions.length, 3);
    assert.deepEqual(definitions.map((item) => item.type).sort(), ["alpha", "alpha", "luminance"]);
    assert.equal(definitions.filter((item) => item.vector === "vector").length, 1);
    assert.ok(definitions.every((item) => item.units === "userSpaceOnUse" && item.contentUnits === "userSpaceOnUse" && item.interpolation === "sRGB"));
    await importFixture(page, empty());
    await openSvg(svg, "Exported SVG mask roundtrip.svg");
    const reopenedPng = pixels(await exported("png", "reopened"));
    let maxDifference = 0;
    for (let i = 0; i < originalPng.data.length; i++) maxDifference = Math.max(maxDifference, Math.abs(originalPng.data[i] - reopenedPng.data[i]));
    assert.ok(maxDifference <= 3, "SVG export/reimport must preserve the entire 128×128 mask image: maximum channel difference " + maxDifference);
    check("Native SVG export emits explicit mask units/types/sRGB and reopens through the real file picker with matching full-image PNG pixels");

    const stable = await documentState(page), stableView = await state(page);
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignImportFile(
        '<svg><defs><mask id="cycle"><rect width="10" height="10" mask="url(#cycle)"/></mask></defs><rect width="10" height="10" mask="url(#cycle)"/></svg>',
        "Cyclic mask.svg")), false);
    await frames(page);
    const rejected = await state(page);
    assert.deepEqual(await documentState(page), stable);
    assert.deepEqual(rejected.selection, stableView.selection);
    assert.equal(rejected.documentRevision, stableView.documentRevision);
    assert.equal(rejected.historyLength, stableView.historyLength);
    check("The application rejects a cyclic SVG mask reference without mutating the current document, selection, revision or history");
}
