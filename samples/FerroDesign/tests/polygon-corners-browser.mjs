import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";
import { coverageConvergence } from "./alpha-coverage.mjs";
import { alphaDiagnostics, areaAveragedSvgAlpha } from "./svg-alpha-reference.mjs";

// Native radius authoring retains editable primitives; only exported artwork
// contains the shared cubic fillets. No document-writing test bridge is used
// after the initial isolated fixture for each case.
export async function polygonCornerRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("document", "args", "return (" + source + ")(document,args)")(document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const select = async (id, x, y) => {
        await clickNative(page, "SelectTool");
        const before = await state(page);
        if (before.selection.length) {
            await page.keyboard.press("Escape");
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection.length === 0,
                undefined, { timeout: 10000 });
            await waitForNativeFrame(page, before.renderedFrames, "Polygon selection clear");
            await waitForNativeScene(page, before.sceneFrames, "Polygon selection clear");
            const cleared = await state(page);
            assert.equal(cleared.chrome.PageInspector.visible, true);
            assert.equal(cleared.chrome.DesignInspector.visible, false);
        }
        await clickWorld(page, x, y);
        assert.deepEqual((await state(page)).selection, [id]);
    };
    const editRadius = async value => {
        await clickNative(page, "RadiusField", "InspectorViewport");
        assert.equal((await state(page)).chrome.RadiusField.focused, true);
        await page.keyboard.press("Control+a"); await page.keyboard.insertText(value);
        await page.keyboard.press("Enter"); await frames(page);
    };
    const history = async (before, after, depth, x, y, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        await clickWorld(page, x, y);
        let view = await state(page); await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        await waitForNativeScene(page, view.sceneFrames, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " exact undo");
        view = await state(page); await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        await waitForNativeScene(page, view.sceneFrames, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " exact redo");
    };
    const placement = node => Object.fromEntries(["kind", "x", "y", "width", "height", "rotation",
        "flip_x", "flip_y", "affine_transform", "polygon_sides", "star_inner_radius", "path"]
        .map(key => [key, node[key]]));
    const fill = { paint: { type: "solid", r: 37, g: 99, b: 235, a: 255 }, opacity: 1, visible: true };
    const primitive = (id, properties) => fixtureNode(reference, id, {
        kind: "polygon", x: 40, y: 40, width: 180, height: 160, polygon_sides: 3,
        star_inner_radius: 0.45, corner_radius: 0, corner_radii: null, corner_smoothing: 0,
        fills: [fill], strokes: [], effects: [], ...properties,
    });
    const inspectorTab = async (tab, panel, hidden) => {
        const before = await state(page);
        await clickNative(page, tab);
        await page.waitForFunction(({ panel, hidden }) => {
            const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
            const bounds = chrome[panel];
            return bounds?.visible === true && bounds.measureValid === true && bounds.arrangeValid === true &&
                bounds.width > 0 && bounds.height > 0 &&
                [bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) &&
                chrome[hidden]?.visible === false;
        }, { panel, hidden }, { timeout: 10000 });
        if (before.chrome[panel]?.visible !== true) {
            // Inspector changes need the committed native hit-test scene;
            // they need not redraw the retained canvas artwork.
            await waitForNativeScene(page, before.sceneFrames, tab + " inspector activation");
        }
        const view = await state(page);
        assert.equal(view.chrome[panel].visible, true, panel + " must be visible after the native tab click");
        assert.equal(view.chrome[hidden].visible, false, hidden + " must be hidden after the native tab click");
    };
    const download = async (button, name, suffix) => {
        const inspect = button === "ExportXamlButton";
        let primaryError;
        try {
            if (inspect) await inspectorTab("InspectTab", "InspectInspector", "DesignInspector");
            const [item] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
                clickNative(page, button, "InspectorViewport")]);
            assert.equal(await item.failure(), null);
            const path = resolve(output, mode + "-polygon-corners-" + name + "." + suffix);
            await item.saveAs(path);
            return await readFile(path);
        } catch (error) {
            primaryError = error;
            throw error;
        } finally {
            if (inspect) {
                try {
                    await inspectorTab("DesignTab", "DesignInspector", "InspectInspector");
                } catch (restoreError) {
                    if (!primaryError) throw restoreError;
                    // Keep the export error and its stack first if restoring
                    // the actual Design tab also failed. The scenario stays
                    // failed, and its normal session boundary reloads next.
                    try {
                        primaryError.stack = (primaryError.stack || String(primaryError)) +
                            "\nRestoring the native Design inspector also failed:\n" +
                            (restoreError.stack || String(restoreError));
                    } catch {
                        // Diagnostic annotation cannot replace the primary failure.
                    }
                }
            }
        }
    };

    await importFixture(page, isolatedDocument(reference, [primitive(2, { name: "Rounded triangle" })], "Polygon radius authoring"));
    await select(2, 130, 120);
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await editRadius("18.5");
    await waitDocument(document => document.nodes[2].corner_radius === 18.5);
    const rounded = await documentState(page);
    assert.deepEqual(placement(rounded.nodes[2]), placement(before.nodes[2]));
    assert.equal(rounded.nodes[2].corner_radii, null);
    assert.ok(rounded.nodes[2].overridden_properties.includes("corner_radius"));
    assert.equal((await state(page)).historyLength, depth + 1);
    for (const invalid of ["-1", "NaN"]) {
        await editRadius(invalid);
        assert.deepEqual(await documentState(page), rounded);
        assert.equal((await state(page)).historyLength, depth + 1);
        assert.equal((await state(page)).chrome.RadiusField.text, invalid);
        await page.keyboard.press("Escape"); await frames(page);
        assert.equal((await state(page)).chrome.RadiusField.text, "18.50");
    }
    await captureNativeScreenshot(mode + "-polygon-corner-radius", { controls: ["RadiusField", "CanvasHost"] });
    await history(before, rounded, depth, 130, 120, "Polygon radius");
    check("Native fractional polygon radius preserves the editable triangle, box and transform, rejects negative/nonfinite drafts without changing history, and restores the complete document with one undo/redo");

    const native = PNG.sync.read(await download("ExportPngButton", "triangle", "png"));
    const svg = (await download("ExportSvgButton", "triangle", "svg")).toString("utf8");
    const xaml = (await download("ExportXamlButton", "triangle", "xaml")).toString("utf8");
    assert.deepEqual([native.width, native.height], [180, 160]);
    assert.equal(native.data[(4 * native.width + 90) * 4 + 3], 0, "The former pointed tip is transparent");
    assert.deepEqual(Array.from(native.data.subarray((80 * native.width + 90) * 4, (80 * native.width + 90) * 4 + 4)), [37, 99, 235, 255]);
    const external = await page.evaluate(async ({ svg, xaml }) => {
        const parsed = new DOMParser().parseFromString(svg, "image/svg+xml");
        const markup = new DOMParser().parseFromString(xaml, "application/xml");
        const curves = [...parsed.querySelectorAll("path")].map(path => path.getAttribute("d") || "");
        const nativeCurves = [...markup.querySelectorAll("StreamGeometry")].map(geometry => geometry.textContent);
        const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
        try {
            const image = new Image(); image.src = url; await image.decode();
            const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
            return { width: canvas.width, height: canvas.height, curves, nativeCurves,
                parseError: !!markup.querySelector("parsererror"),
                alpha: Array.from({ length: pixels.length / 4 }, (_, index) => pixels[index * 4 + 3]) };
        } finally { URL.revokeObjectURL(url); }
    }, { svg, xaml });
    assert.deepEqual([external.width, external.height], [180, 160]);
    assert.equal(external.parseError, false);
    assert.ok(external.curves.some(curve => (curve.match(/C/g) || []).length === 6));
    assert.ok(external.nativeCurves.some(curve => (curve.match(/C/g) || []).length === 6), "XAML retains public StreamGeometry cubics");
    // Preserve the original 1x observation, then compare independent SVG area
    // coverage using the same converged reference as the box-corner tests.
    // The original 12-alpha / 85-pixel silhouette gate remains unchanged.
    const rawComparison = alphaDiagnostics(native, external.alpha);
    const coveragePath = resolve(output, mode + "-polygon-corners-triangle-coverage.json");
    const coverage = { scales: [16, 32], stage: "raw-1x", rawComparison };
    await writeFile(coveragePath, JSON.stringify(coverage, null, 2) + "\n");
    const coarse = await areaAveragedSvgAlpha(page, svg, native.width, native.height, 16);
    const fine = await areaAveragedSvgAlpha(page, svg, native.width, native.height, 32);
    const convergence = coverageConvergence(coarse, fine);
    const comparison = alphaDiagnostics(native, fine.map(alpha => Math.round(alpha)));
    Object.assign(coverage, { stage: "complete", convergence, comparison });
    await writeFile(coveragePath, JSON.stringify(coverage, null, 2) + "\n");
    console.log("[polygon-corners-coverage " + mode + "] " + JSON.stringify(coverage));
    assert.ok(convergence.converged, "Independent SVG area coverage must converge within 4 alpha units without opposite opaque/transparent pixels");
    const differingEdges = comparison.differingOver12;
    assert.ok(differingEdges <= 85, "Independent SVG and native PNG share the rounded silhouette: " + differingEdges);
    assert.deepEqual(await documentState(page), rounded);
    check("Public FerroUI PNG, independent SVG decoding and native XAML geometry preserve the same six-cubic rounded triangle, transparent corner holes and authored export dimensions");

    const star = primitive(2, { kind: "star", name: "Affine rounded star", polygon_sides: 5,
        x: 100, y: 60, width: 160, height: 140, rotation: 17, flip_x: true,
        affine_transform: { a: 1, b: 0.15, c: 0.2, d: 0.9, e: 0, f: 0 } });
    const world = (node, x, y) => {
        const matrix = node.affine_transform || { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        const localX = (matrix.a * x + matrix.c * y + matrix.e - node.width / 2) * (node.flip_x ? -1 : 1);
        const localY = (matrix.b * x + matrix.d * y + matrix.f - node.height / 2) * (node.flip_y ? -1 : 1);
        const angle = node.rotation * Math.PI / 180;
        return { x: node.x + node.width / 2 + localX * Math.cos(angle) - localY * Math.sin(angle),
            y: node.y + node.height / 2 + localX * Math.sin(angle) + localY * Math.cos(angle) };
    };
    await importFixture(page, isolatedDocument(reference, [star], "Affine star radius"));
    const center = world(star, 80, 70);
    await select(2, center.x, center.y);
    const starBefore = await documentState(page), starDepth = (await state(page)).historyLength;
    await editRadius("20.25"); await waitDocument(document => document.nodes[2].corner_radius === 20.25);
    const starAfter = await documentState(page);
    assert.deepEqual(placement(starAfter.nodes[2]), placement(starBefore.nodes[2]));
    await history(starBefore, starAfter, starDepth, center.x, center.y, "Affine star radius");
    const tip = world(star, 80, 1);
    await clickWorld(page, tip.x, tip.y);
    assert.deepEqual((await state(page)).selection, [], "The rounded-away tip cannot select the star through its rectangular box");
    await select(2, center.x, center.y);
    await captureNativeScreenshot(mode + "-star-corner-radius", { controls: ["RadiusField", "CanvasHost"] });
    const starXaml = (await download("ExportXamlButton", "affine-star", "xaml")).toString("utf8");
    const matrices = await page.evaluate(source => {
        const parsed = new DOMParser().parseFromString(source, "application/xml");
        return [...parsed.querySelectorAll("MatrixTransform")].map(matrix => matrix.getAttribute("Matrix").split(",").map(Number));
    }, starXaml);
    const origin = world(star, 0, 0), basisX = world(star, 1, 0), basisY = world(star, 0, 1);
    const expected = [basisX.x - origin.x, basisX.y - origin.y, basisY.x - origin.x, basisY.y - origin.y, origin.x, origin.y];
    assert.ok(matrices.some(matrix => matrix.length === 6 && matrix.every((value, index) => Math.abs(value - expected[index]) < 1e-9)),
        "XAML retains the composed affine matrix, center rotation and reflection");
    assert.deepEqual(await documentState(page), starAfter);
    check("Native star rounding preserves shear, center rotation, reflection and dimensions; rounded-away tips fail actual canvas picking, and XAML retains the complete affine matrix");

    const bound = isolatedDocument(reference, [primitive(2, { kind: "star", name: "Bound star radius",
        polygon_sides: 5, corner_radius: 12.5, variable_bindings: { corner_radius: 900 } })], "Explicit literal radius");
    bound.variable_mode = "Default"; bound.next_id = 901;
    bound.variables = { 900: { id: 900, name: "Radius", collection: "Shapes",
        values: { Default: { type: "number", value: 12.5 } } } };
    await importFixture(page, bound); await select(2, 130, 120);
    const boundBefore = await documentState(page), boundDepth = (await state(page)).historyLength;
    assert.equal((await state(page)).chrome.RadiusField.text, "12.50");
    await editRadius("12.50");
    await waitDocument(document => !("corner_radius" in document.nodes[2].variable_bindings));
    const detached = await documentState(page);
    assert.equal(detached.nodes[2].corner_radius, 12.5);
    assert.deepEqual(placement(detached.nodes[2]), placement(boundBefore.nodes[2]));
    await history(boundBefore, detached, boundDepth, 130, 120, "Equal-value radius override");
    assert.equal((await state(page)).chrome.RadiusField.text, "12.50");
    await editRadius("12.50");
    assert.deepEqual(await documentState(page), detached);
    assert.equal((await state(page)).historyLength, boundDepth + 1, "An unbound equal literal is a true no-op");
    assert.equal((await state(page)).invalidNodes, 0);
    assert.equal((await state(page)).unsupportedEffects, 0);
    check("Entering an equal literal radius detaches only its variable in one reversible native edit; a later equal unbound value leaves history unchanged");
}
