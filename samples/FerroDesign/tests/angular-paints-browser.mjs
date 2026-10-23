import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Real native controls, retained FerroUI drawing and downloaded exports only.
export async function angularPaintRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, captureNativeScreenshot } = helpers;
    const field = async (name, value) => {
        await clickNative(page, name, "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    const red = { r: 255, g: 0, b: 0, a: 255 };
    const blue = { r: 0, g: 0, b: 255, a: 255 };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        name: "Angular surface", x: 40, y: 40, width: 120, height: 100,
        corner_radius: 0, corner_radii: null, effects: [], strokes: [],
        fills: [{ paint: { type: "solid", ...red }, opacity: 1, visible: true }],
    })], "Angular native authoring"));
    await clickWorld(page, 100, 80);
    await clickNative(page, "EditPaintsButton", "InspectorViewport");
    await field("PaintColor0", "#EF245780");
    await chooseNative(page, "PaintType0", { index: 3, scrollContainer: "ModalViewport" });
    let fill = (await documentState(page)).nodes[2].fills[0];
    assert.equal(fill.paint.type, "angular_gradient");
    assert.deepEqual(fill.paint.stops[0].color, { r: 239, g: 36, b: 87, a: 128 },
        "Converting the current native field retains its most recently committed RGBA color");
    await field("PaintStopColor0_0", "#FF0000");
    await field("PaintStopColor0_1", "#0000FF");
    await field("PaintCenterX0", "0.5");
    await field("PaintCenterY0", "0.5");
    const beforeAngle = await documentState(page), beforeDepth = (await state(page)).historyLength;
    await field("PaintAngle0", "90");
    const afterAngle = await documentState(page);
    assert.equal(afterAngle.nodes[2].fills[0].paint.angle, 90);
    assert.equal((await state(page)).historyLength, beforeDepth + 1);
    await field("PaintAngle0", "NaN");
    assert.deepEqual(await documentState(page), afterAngle, "Invalid angle has no document effect");
    assert.equal((await state(page)).historyLength, beforeDepth + 1);
    await page.keyboard.press("Escape"); await frames(page);
    assert.equal((await state(page)).chrome.PaintAngle0.text, "90");
    await captureNativeScreenshot(mode + "-angular-paint-authoring", {
        controls: ["ModalPanel", "PaintType0", "PaintAngle0", "PaintDone"],
    });
    await clickNative(page, "PaintDone");
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), beforeAngle);
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), afterAngle);
    check("Native angular paint controls retain current RGBA colors, edit stops/center/clockwise angle, reject NaN, and preserve exact one-step angle undo/redo");

    const [pngDownload] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await pngDownload.failure(), null);
    const pngPath = resolve(output, mode + "-angular-paint.png");
    await pngDownload.saveAs(pngPath);
    const png = PNG.sync.read(await readFile(pngPath));
    assert.deepEqual([png.width, png.height], [120, 100]);
    const pixel = (x, y) => [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    const start = pixel(110, 53), end = pixel(110, 46), half = pixel(10, 50);
    assert.ok(start[0] > 235 && start[2] < 20 && start[3] === 255, "90° begins clockwise from the right ray: " + start);
    assert.ok(end[2] > 235 && end[0] < 20 && end[3] === 255, "The end reaches the other side of the seam: " + end);
    assert.ok(Math.abs(half[0] - half[2]) < 12 && half[3] === 255, "Half of the native sweep blends both stops: " + half);
    assert.equal((await state(page)).unsupportedEffects, 0);
    check("PNG export renders native angular-gradient start/end seam and halfway colors through FerroUI with no unsupported rendering effects");

    const [svgDownload] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportSvgButton", "InspectorViewport"),
    ]);
    assert.equal(await svgDownload.failure(), null);
    const svgPath = resolve(output, mode + "-angular-paint.svg");
    await svgDownload.saveAs(svgPath);
    const svg = await readFile(svgPath, "utf8");
    assert.ok(svg.includes('data-ferrodesign-approximation="angular-gradient"'));
    const sectorCounts = await page.evaluate((source) => {
        const document = new DOMParser().parseFromString(source, "image/svg+xml");
        return [...document.querySelectorAll('pattern[data-ferrodesign-approximation="angular-gradient"]')]
            .map(pattern => pattern.querySelectorAll("path").length);
    }, svg);
    assert.equal(sectorCounts.length, 1);
    assert.ok(sectorCounts[0] >= 512 && sectorCounts[0] <= 1024, "SVG sectors have a fixed work budget");
    assert.match((await state(page)).chrome.ToastText.text, /angular-gradient.*1024 vector sectors/);
    const svgPixels = await page.evaluate(async (source) => {
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        try {
            const image = new Image();
            image.src = url;
            await image.decode();
            const canvas = document.createElement("canvas");
            canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d");
            context.drawImage(image, 0, 0);
            const samples = [[110, 53], [110, 46], [10, 50]].map(([x, y]) =>
                [...context.getImageData(x, y, 1, 1).data]);
            const all = context.getImageData(1, 1, canvas.width - 2, canvas.height - 2).data;
            let gaps = 0;
            for (let i = 3; i < all.length; i += 4) if (all[i] < 250) gaps++;
            return { samples, gaps };
        } finally { URL.revokeObjectURL(url); }
    }, svg);
    for (const [i, actual] of svgPixels.samples.entries()) {
        const expected = [start, end, half][i];
        assert.ok(expected.every((value, channel) => Math.abs(value - actual[channel]) <= 4),
            "External browser SVG angular colors agree with native PNG samples: " + JSON.stringify({ actual, expected }));
    }
    assert.equal(svgPixels.gaps, 0, "Shared opaque SVG sector edges must not expose transparent seams");
    assert.deepEqual(await documentState(page), afterAngle, "Approximate SVG export never mutates the exact native document");
    check("Angular SVG export carries a bounded vector approximation and a visible native warning while the editable document retains the exact paint");

    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("create-fill-style")), true);
    await frames(page);
    const created = await documentState(page), styleId = Number(Object.keys(created.styles)[0]);
    assert.equal(created.nodes[2].fill_style, styleId);
    assert.equal(created.styles[styleId].value.fills[0].paint.type, "angular_gradient");
    await field("StyleGradientCoordinate0_2", "180");
    const shared = await documentState(page);
    assert.equal(shared.styles[styleId].value.fills[0].paint.angle, 180);
    assert.deepEqual(shared.nodes[2].fills, shared.styles[styleId].value.fills);
    await chooseNative(page, "StyleFillType0", { index: 1, scrollContainer: "ModalViewport" });
    await chooseNative(page, "StyleFillType0", { index: 3, scrollContainer: "ModalViewport" });
    fill = (await documentState(page)).nodes[2].fills[0];
    assert.deepEqual(fill.paint.stops, [{ offset: 0, color: red }, { offset: 1, color: blue }],
        "Converting a shared angular gradient through linear retains both complete stop values");
    await clickNative(page, "StyleDone", "ModalActions");
    check("Local style authoring captures angular paints, propagates angle edits, and preserves all stops when changing gradient types");

    // Keep each stroke inside a transparent frame so export viewports include
    // the exterior halves; sample the external decoder, not just SVG strings.
    const angularStroke = (align = "center") => ({
        paint: { type: "angular_gradient", center: { x: 0.25, y: 0.75 }, angle: 0,
            stops: [{ offset: 0, color: red }, { offset: 1, color: blue }] },
        width: 20, opacity: 1, visible: true, align, cap: "butt", join: "miter", dash: [],
    });
    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Angular stroke export", width: 480, height: 160,
            children: [3, 4, 5], fills: [], corner_radius: 0, corner_radii: null }),
        fixtureNode(reference, 3, { parent: 2, name: "Centered angular stroke", x: 20, y: 30, width: 100, height: 80,
            fills: [], strokes: [angularStroke()], corner_radius: 0, corner_radii: null }),
        fixtureNode(reference, 4, { parent: 2, name: "Outside angular stroke", x: 170, y: 30, width: 100, height: 80,
            fills: [], strokes: [angularStroke("outside")], corner_radius: 0, corner_radii: null }),
        fixtureNode(reference, 5, { parent: 2, kind: "line", name: "Horizontal angular line", x: 320, y: 70,
            width: 120, height: 0, fills: [], strokes: [angularStroke()] }),
    ], "Angular stroke bounds"));
    await clickWorld(page, 460, 140);
    assert.deepEqual((await state(page)).selection, [2]);
    const strokeBefore = await documentState(page);
    const downloads = {};
    for (const kind of ["png", "svg"]) {
        const [download] = await Promise.all([
            page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, kind === "png" ? "ExportPngButton" : "ExportSvgButton", "InspectorViewport"),
        ]);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-angular-strokes." + kind);
        await download.saveAs(path);
        downloads[kind] = await readFile(path);
    }
    const nativeStroke = PNG.sync.read(downloads.png);
    assert.deepEqual([nativeStroke.width, nativeStroke.height], [480, 160]);
    const points = [[44, 24], [35, 24], [17, 80], [189, 16], [180, 16], [156, 80], [349, 63], [340, 63]];
    const expected = points.map(([x, y]) => {
        const offset = (y * nativeStroke.width + x) * 4;
        return [...nativeStroke.data.subarray(offset, offset + 4)];
    });
    assert.ok(expected.every(pixel => pixel[3] === 255), "Native stroke exterior samples must be covered");
    const externalStroke = await page.evaluate(async ({ source, points }) => {
        const xml = new DOMParser().parseFromString(source, "image/svg+xml");
        const patterns = [...xml.querySelectorAll('pattern[data-ferrodesign-approximation="angular-gradient"]')];
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        try {
            const image = new Image(); image.src = url; await image.decode();
            const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            return {
                size: [canvas.width, canvas.height],
                userSpace: patterns.length === 3 && patterns.every(pattern => pattern.getAttribute("patternUnits") === "userSpaceOnUse"),
                samples: points.map(([x, y]) => [...context.getImageData(x, y, 1, 1).data]),
                interior: [...context.getImageData(60, 60, 1, 1).data],
            };
        } finally { URL.revokeObjectURL(url); }
    }, { source: downloads.svg.toString("utf8"), points });
    assert.deepEqual(externalStroke.size, [480, 160]);
    assert.ok(externalStroke.userSpace, "Stroke paints must not repeat at the unstroked object bounds");
    for (const [i, actual] of externalStroke.samples.entries()) {
        assert.ok(expected[i].every((value, channel) => Math.abs(value - actual[channel]) <= 4),
            "SVG exterior angular stroke agrees with native PNG at " + points[i] + ": " + JSON.stringify({ actual, expected: expected[i] }));
    }
    assert.equal(externalStroke.interior[3], 0, "Angular stroke export preserves a transparent interior");
    assert.deepEqual(await documentState(page), strokeBefore);
    check("Browser-decoded angular SVG strokes match native PNG beyond center/outside geometry edges and on a zero-height horizontal line");

}
