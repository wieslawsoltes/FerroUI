import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Fixture import supplies the unfilled graph only. Face discovery, painting,
// paint-stack edits, clearing and inheritance all use native pointer controls.
export async function regionPaintRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const fill = (r, g, b) => ({ paint: { type: "solid", r, g, b, a: 255 }, opacity: 1, visible: true });
    const red = fill(239, 68, 68), blue = fill(37, 99, 235), neutral = fill(217, 217, 217);
    const graph = {
        vertices: Object.fromEntries([[0, 0], [100, 0], [200, 0], [200, 120], [100, 120], [0, 120]]
            .map(([x, y], index) => [index + 1, { position: { x, y }, mode: "corner" }])),
        edges: Object.fromEntries([[1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 1], [2, 5]]
            .map(([start, end], index) => [index + 7, { start, end, curve: { type: "line" }, stroke: true }])),
        regions: {}, fill_rule: "even_odd", next_id: 14,
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "vector", name: "Two independent vector faces", x: 40, y: 40, width: 200, height: 120,
        corner_radius: 0, corner_radii: null, fills: [red], strokes: [],
        path: { segments: [], fill_rule: "even_odd" }, vector_network: graph,
    })], "Native region paints"));
    const selectRoot = async () => {
        await clickNative(page, "SelectTool");
        await page.keyboard.press("Escape");
        const before = await state(page);
        await page.keyboard.press("Tab");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.selection.join() === "2" && view.chrome.CanvasHost.focusWithin;
        }, null, { timeout: 10000 });
        // Match clickWorld's publication contract before using the inspector
        // that keyboard selection has just changed.
        await waitForNativeFrame(page, before.renderedFrames, "Region fixture keyboard selection");
        await waitForNativeScene(page, before.sceneFrames, "Region fixture keyboard selection");
    };
    await selectRoot();
    const edit = async (shortcut = false) => {
        const before = await state(page);
        if (before.vectorNode !== 2) {
            await clickNative(page, "EditVectorButton", "InspectorViewport");
            await page.waitForFunction(() => {
                const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return view.vectorNode === 2 && view.chrome.CanvasHost.focusWithin;
            }, null, { timeout: 10000 });
            await waitForNativeFrame(page, before.renderedFrames, "Native vector editing entry");
            await waitForNativeScene(page, before.sceneFrames, "Native vector editing entry");
        }
        if (shortcut) {
            await page.keyboard.press("Shift+B");
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).vectorTool === "Bucket");
        } else {
            await clickNative(page, "NetworkBucketButton", "InspectorViewport");
        }
        assert.equal((await state(page)).vectorTool, "Bucket");
    };
    const roundTrip = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits exactly once");
        let frame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, frame, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " restores every field on undo");
        frame = (await state(page)).renderedFrames;
        await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, frame, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " restores every field on redo");
    };
    const paint = async (x, clear = false) => {
        const before = await state(page);
        if (clear) await page.keyboard.down("Shift");
        try { await clickWorld(page, x, 100); }
        finally { if (clear) await page.keyboard.up("Shift"); }
        await waitForNativeFrame(page, before.renderedFrames, "Native region bucket");
    };
    const regionFills = (document, id) => document.nodes[2].vector_network.regions[id].fills;
    const exportImage = async (suffix, kind = "png") => {
        if ((await state(page)).vectorNode === 2) await clickNative(page, "DoneVectorButton", "InspectorViewport");
        const [download] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, kind === "png" ? "ExportPngButton" : "ExportSvgButton", "InspectorViewport"),
        ]);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-region-" + suffix + "." + kind);
        await download.saveAs(path);
        return readFile(path);
    };
    const pngPixel = (png, x, y) => [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    const nativeColors = async (name, samples) => {
        await page.mouse.move(4, 4);
        const view = await state(page);
        const surface = PNG.sync.read(await captureNativeScreenshot(mode + "-" + name, { controls: ["CanvasHost"] }));
        for (const [x, expected] of samples) {
            const point = { x: Math.round(view.chrome.CanvasHost.x + view.pan.x + x * view.zoom),
                y: Math.round(view.chrome.CanvasHost.y + view.pan.y + 100 * view.zoom) };
            assert.deepEqual(pngPixel(surface, point.x, point.y), expected,
                "The actual browser surface must render the first visible paint of each region");
        }
    };

    await edit(true);
    const unfilled = await documentState(page), fillDepth = (await state(page)).historyLength;
    await paint(90);
    const leftPainted = await documentState(page);
    const [left] = Object.keys(leftPainted.nodes[2].vector_network.regions);
    assert.ok(left);
    assert.equal(Object.keys(leftPainted.nodes[2].vector_network.regions).length, 1);
    assert.deepEqual(regionFills(leftPainted, left), [red]);
    assert.deepEqual(leftPainted.nodes[2].vector_network.vertices, graph.vertices);
    assert.deepEqual(leftPainted.nodes[2].vector_network.edges, graph.edges);
    assert.equal(leftPainted.nodes[2].vector_network.next_id, graph.next_id + 1);
    await roundTrip(unfilled, leftPainted, fillDepth, "Bucket face discovery");
    await edit();
    await paint(90);
    assert.deepEqual(await documentState(page), leftPainted, "Painting the same face with the same stack is a no-op");
    assert.equal((await state(page)).historyLength, fillDepth + 1);
    check("Native Shift+B and Paint bucket discover only the clicked closed face, preserve shared graph vertices/edges and stable IDs, are idempotent, and have one exact undo/redo transaction");

    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    const beforeBlue = await documentState(page), blueDepth = (await state(page)).historyLength;
    await clickNative(page, "PaintColor0", "ModalViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("2563EB");
    await page.keyboard.press("Enter");
    await page.waitForFunction((id) => {
        const paint = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].vector_network.regions[id]?.fills?.[0]?.paint;
        return paint?.r === 37 && paint.g === 99 && paint.b === 235;
    }, left);
    const blueLeft = await documentState(page);
    assert.deepEqual(regionFills(blueLeft, left), [blue]);
    assert.deepEqual(blueLeft.nodes[2].fills, [red], "Region editing cannot replace the layer's inherited paint stack");
    await captureNativeScreenshot(mode + "-native-region-paints", { controls: ["ModalPanel", "PaintColor0", "PaintDone"] });
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(beforeBlue, blueLeft, blueDepth, "Independent region color");

    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    const addDepth = (await state(page)).historyLength;
    await clickNative(page, "PaintAdd", "ModalActions");
    await page.waitForFunction(id => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson())
        .nodes[2].vector_network.regions[id].fills.length === 2, left);
    await page.waitForFunction(() => {
        const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
        return ["PaintCard0", "PaintCard1"].every(name => chrome[name]?.measureValid &&
            chrome[name]?.arrangeValid && chrome[name]?.height > 0);
    });
    const added = await documentState(page);
    assert.deepEqual(regionFills(added, left), [neutral, blue], "Add fill inserts a new front paint");
    const cards = (await state(page)).chrome;
    assert.ok(cards.PaintCard0.y < cards.PaintCard1.y, "The first authored fill is the first visible paint row");
    assert.equal(cards.PaintUp0.enabled, false);
    assert.equal(cards.PaintDown0.enabled, true);
    assert.deepEqual(added.nodes[2].fills, [red]);
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(blueLeft, added, addDepth, "Add front region paint");
    await nativeColors("region-added-front-paint", [[90, [217, 217, 217, 255]]]);

    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    const downDepth = (await state(page)).historyLength;
    const committedColor = (await state(page)).chrome.PaintColor0.text;
    await clickNative(page, "PaintColor0", "ModalViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("not-a-color");
    await page.keyboard.press("Enter");
    await clickNative(page, "PaintVisible0", "ModalViewport");
    assert.deepEqual(await documentState(page), added, "An invalid color draft refuses a visibility change without changing the region target");
    assert.equal((await state(page)).historyLength, downDepth);
    assert.equal((await state(page)).chrome.PaintVisible0.checked, true, "A rejected visibility change restores the committed native checkbox state");
    assert.equal((await state(page)).chrome.PaintColor0.text, "not-a-color");
    await clickNative(page, "PaintDown0", "ModalViewport");
    assert.deepEqual(await documentState(page), added, "An invalid draft refuses structural paint edits without changing the region target");
    assert.equal((await state(page)).historyLength, downDepth);
    assert.equal((await state(page)).chrome.PaintColor0.text, "not-a-color");
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, true);
    await clickNative(page, "PaintColor0", "ModalViewport");
    await page.keyboard.press("Escape");
    await frames(page);
    assert.equal((await state(page)).chrome.PaintColor0.text, committedColor);
    await clickNative(page, "PaintDown0", "ModalViewport");
    const lowered = await documentState(page);
    assert.deepEqual(regionFills(lowered, left), [blue, neutral], "Down moves the visible first row behind the next paint");
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(added, lowered, downDepth, "Lower region paint");
    await nativeColors("region-lowered-paint", [[90, [37, 99, 235, 255]]]);

    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    const upDepth = (await state(page)).historyLength;
    await clickNative(page, "PaintUp1", "ModalViewport");
    assert.deepEqual(await documentState(page), added, "Up restores the lower paint to the visible first row");
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(lowered, added, upDepth, "Raise region paint");
    await nativeColors("region-raised-paint", [[90, [217, 217, 217, 255]]]);
    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    const removeDepth = (await state(page)).historyLength;
    await clickNative(page, "PaintRemove0", "ModalViewport");
    assert.deepEqual(await documentState(page), blueLeft);
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(added, blueLeft, removeDepth, "Remove front region paint");
    check("Native region paint rows match front-to-back rendering: Add inserts at the front, invalid drafts refuse structural edits, Down and Up swap the actual visible canvas color, and removal restores the underlying fill with exact single-step undo/redo");

    await edit();
    await paint(190);
    const twoFaces = await documentState(page);
    const right = Object.keys(twoFaces.nodes[2].vector_network.regions).find(id => id !== left);
    assert.ok(right);
    assert.deepEqual(regionFills(twoFaces, left), [blue]);
    assert.deepEqual(regionFills(twoFaces, right), [red]);
    assert.deepEqual(twoFaces.nodes[2].fills, [red]);
    await nativeColors("native-two-region-colors", [[90, [37, 99, 235, 255]], [190, [239, 68, 68, 255]]]);
    const png = PNG.sync.read(await exportImage("blue-red"));
    assert.deepEqual([png.width, png.height], [200, 120]);
    assert.deepEqual(pngPixel(png, 50, 60), [37, 99, 235, 255]);
    assert.deepEqual(pngPixel(png, 150, 60), [239, 68, 68, 255]);
    const svg = (await exportImage("blue-red", "svg")).toString("utf8");
    const svgPixels = await page.evaluate(async source => {
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        try {
            const image = new Image(); image.src = url; await image.decode();
            const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            return [50, 150].map(x => [...context.getImageData(x, 60, 1, 1).data]);
        } finally { URL.revokeObjectURL(url); }
    }, svg);
    assert.deepEqual(svgPixels, [[37, 99, 235, 255], [239, 68, 68, 255]]);
    assert.deepEqual(await documentState(page), twoFaces);
    check("Native Region paints edits one face independently of the layer and its neighbour; downloaded FerroUI PNG and externally decoded SVG retain the two distinct colors");

    await edit();
    const beforeClear = await documentState(page), clearDepth = (await state(page)).historyLength;
    await paint(90, true);
    const cleared = await documentState(page);
    assert.deepEqual(regionFills(cleared, left), []);
    assert.deepEqual(regionFills(cleared, right), [red]);
    assert.equal(cleared.nodes[2].vector_network.next_id, beforeClear.nodes[2].vector_network.next_id);
    await roundTrip(beforeClear, cleared, clearDepth, "Shift bucket clear");
    const clearedPng = PNG.sync.read(await exportImage("cleared"));
    assert.deepEqual(pngPixel(clearedPng, 50, 60), [0, 0, 0, 0]);
    assert.deepEqual(pngPixel(clearedPng, 150, 60), [239, 68, 68, 255]);
    await edit();
    await paint(90, true);
    assert.deepEqual(await documentState(page), cleared, "Repeated clearing retains topology and history");
    assert.equal((await state(page)).historyLength, clearDepth + 1);
    const resetDepth = (await state(page)).historyLength;
    await clickNative(page, "NetworkRegionResetButton", "InspectorViewport");
    await frames(page);
    const inherited = await documentState(page);
    assert.equal(regionFills(inherited, left) ?? null, null);
    assert.deepEqual(regionFills(inherited, right), [red]);
    await roundTrip(cleared, inherited, resetDepth, "Restore region inheritance");
    const inheritedPng = PNG.sync.read(await exportImage("inherited"));
    assert.deepEqual(pngPixel(inheritedPng, 50, 60), [239, 68, 68, 255]);
    assert.deepEqual(pngPixel(inheritedPng, 150, 60), [239, 68, 68, 255]);
    await importFixture(page, inherited);
    assert.deepEqual(await documentState(page), inherited);
    check("Shift-click makes one region transparent without deleting its identity; Use layer fills restores inheritance, both edits undo/redo exactly, and the authored document round-trips");

    const embedded = new PNG({ width: 2, height: 2 });
    embedded.data.set([239, 68, 68, 255, 37, 99, 235, 255, 34, 197, 94, 255, 250, 204, 21, 255]);
    const imageFill = { paint: { type: "image", scale: "fill",
        href: "data:image/png;base64," + PNG.sync.write(embedded).toString("base64"),
        crop: { x: 0.25, y: 0, width: 0.5, height: 1 } }, opacity: 1, visible: true };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "vector", name: "Embedded region image", x: 40, y: 40, width: 200, height: 120,
        corner_radius: 0, corner_radii: null, fills: [imageFill], strokes: [],
        path: { segments: [], fill_rule: "even_odd" }, vector_network: graph,
    })], "Native regional image source"));
    await selectRoot();
    await edit();
    await paint(90);
    const imageBefore = await documentState(page), imageDepth = (await state(page)).historyLength;
    const [imageRegion] = Object.keys(imageBefore.nodes[2].vector_network.regions);
    assert.deepEqual(regionFills(imageBefore, imageRegion), [imageFill]);
    await clickNative(page, "NetworkRegionPaintsButton", "InspectorViewport");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.PaintCropCanvas0?.visible);
    const directCropEnabled = (await state(page)).chrome.PaintCropCanvas0.enabled;
    await clickNative(page, "PaintCropReset0", "ModalViewport");
    await page.waitForFunction(id => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson())
        .nodes[2].vector_network.regions[id].fills[0].paint.crop === undefined, imageRegion);
    const imageReset = await documentState(page);
    assert.deepEqual(imageReset.nodes[2].fills, [imageFill], "Reset source changes only the active region");
    assert.equal(regionFills(imageReset, imageRegion)[0].paint.href, imageFill.paint.href);
    assert.equal((await state(page)).historyLength, imageDepth + 1);
    await clickNative(page, "PaintOpacity0", "ModalViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("42");
    const opacityFrame = (await state(page)).renderedFrames;
    await page.keyboard.press("Enter");
    await waitForNativeFrame(page, opacityFrame, "Region image opacity after Reset source");
    const imageEdited = await documentState(page);
    assert.deepEqual(imageEdited.nodes[2].fills, [imageFill], "Reset source must retain the region target for the next native paint edit");
    assert.equal(regionFills(imageEdited, imageRegion)[0].opacity, 0.42);
    assert.equal(directCropEnabled, false, "Direct canvas crop supports layer image fills and must be disabled for regional paints");
    await clickNative(page, "PaintDone", "ModalActions");
    await roundTrip(imageReset, imageEdited, imageDepth + 1, "Regional image opacity after reset");
    let imageFrame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+z");
    await waitForNativeFrame(page, imageFrame, "Return to regional source reset");
    assert.deepEqual(await documentState(page), imageReset);
    await roundTrip(imageBefore, imageReset, imageDepth, "Regional image Reset source");
    check("An embedded regional image retains its paint target after Reset source: subsequent native edits leave layer fills intact, both edits have exact single-step undo/redo, and unsupported direct canvas crop is disabled");
}
