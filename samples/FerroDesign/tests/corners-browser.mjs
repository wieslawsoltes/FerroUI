import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";
import { areaAverageAlpha, coverageConvergence } from "./alpha-coverage.mjs";

// Scalar diagnostics for the independent decoder comparison. Retain the
// original pixel gate below: these distinguish edge coverage from a displaced
// or stale outline without logging either image or relaxing its assertions.
function alphaDiagnostics(png, svgAlpha) {
    const { width, height } = png;
    const native = index => png.data[index * 4 + 3];
    const partial = alpha => alpha > 0 && alpha < 255;
    const summary = {
        width, height, pixels: width * height, differingOver12: 0,
        edgeCoverageDifferences: 0, oppositeOpaqueTransparent: 0,
        threshold128Differences: 0, maxAlphaDifference: 0,
        nativePartialPixels: 0, svgPartialPixels: 0,
        nativeAlphaArea: 0, svgAlphaArea: 0, absoluteAlphaAreaDifference: 0,
        differenceBounds: null, quadrantDifferences: [0, 0, 0, 0],
        // Each pair is counted once; a centered uniform fixture is symmetric.
        nativeMirrorMax: { horizontal: 0, vertical: 0 },
        svgMirrorMax: { horizontal: 0, vertical: 0 },
        samples: [],
    };
    const sampleCounts = [0, 0, 0, 0];
    for (let index = 0; index < svgAlpha.length; index++) {
        const x = index % width, y = Math.floor(index / width);
        const a = native(index), b = svgAlpha[index], delta = Math.abs(a - b);
        summary.nativePartialPixels += Number(partial(a));
        summary.svgPartialPixels += Number(partial(b));
        summary.nativeAlphaArea += a / 255;
        summary.svgAlphaArea += b / 255;
        summary.absoluteAlphaAreaDifference += delta / 255;
        summary.maxAlphaDifference = Math.max(summary.maxAlphaDifference, delta);
        summary.threshold128Differences += Number((a >= 128) !== (b >= 128));
        if (x < width / 2) {
            const mirror = y * width + width - 1 - x;
            summary.nativeMirrorMax.horizontal = Math.max(summary.nativeMirrorMax.horizontal, Math.abs(a - native(mirror)));
            summary.svgMirrorMax.horizontal = Math.max(summary.svgMirrorMax.horizontal, Math.abs(b - svgAlpha[mirror]));
        }
        if (y < height / 2) {
            const mirror = (height - 1 - y) * width + x;
            summary.nativeMirrorMax.vertical = Math.max(summary.nativeMirrorMax.vertical, Math.abs(a - native(mirror)));
            summary.svgMirrorMax.vertical = Math.max(summary.svgMirrorMax.vertical, Math.abs(b - svgAlpha[mirror]));
        }
        if (delta <= 12) continue;
        summary.differingOver12++;
        summary.edgeCoverageDifferences += Number(partial(a) || partial(b));
        summary.oppositeOpaqueTransparent += Number((a === 0 && b === 255) || (a === 255 && b === 0));
        const quadrant = Number(x >= width / 2) + 2 * Number(y >= height / 2);
        summary.quadrantDifferences[quadrant]++;
        const bounds = summary.differenceBounds ??= { left: x, top: y, right: x, bottom: y };
        bounds.left = Math.min(bounds.left, x); bounds.top = Math.min(bounds.top, y);
        bounds.right = Math.max(bounds.right, x); bounds.bottom = Math.max(bounds.bottom, y);
        if (sampleCounts[quadrant]++ < 3) summary.samples.push({ x, y, native: a, svg: b });
    }
    for (const key of ["nativeAlphaArea", "svgAlphaArea", "absoluteAlphaAreaDifference"]) {
        summary[key] = Number(summary[key].toFixed(6));
    }
    return summary;
}

async function areaAveragedSvgAlpha(page, source, width, height, scale) {
    // Keep the large RGBA buffer in the page. Only 28000 averaged alpha values
    // cross the protocol; the same pure averaging function is unit tested.
    const pixels = await page.evaluateHandle(async ({ source, width, height, scale }) => {
        if (width !== 200 || height !== 140 || ![8, 16].includes(scale)) {
            throw new Error("The corner coverage reference has a fixed bounded fixture and scales.");
        }
        const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
        if (parsed.querySelector("parsererror")) throw new Error("Exported SVG must parse.");
        const root = parsed.documentElement, viewBox = root.getAttribute("viewBox");
        if (!viewBox) throw new Error("An explicit SVG viewBox is required to preserve export coordinates.");
        const paths = [...parsed.querySelectorAll("path")].map(path => path.getAttribute("d"));
        root.setAttribute("width", String(width * scale));
        root.setAttribute("height", String(height * scale));
        const encoded = new XMLSerializer().serializeToString(parsed);
        const verified = new DOMParser().parseFromString(encoded, "image/svg+xml");
        if (verified.documentElement.getAttribute("viewBox") !== viewBox ||
            JSON.stringify([...verified.querySelectorAll("path")].map(path => path.getAttribute("d"))) !== JSON.stringify(paths)) {
            throw new Error("Changing the SVG viewport must preserve its viewBox and every path.");
        }
        const url = URL.createObjectURL(new Blob([encoded], { type: "image/svg+xml" }));
        const image = new Image(), canvas = document.createElement("canvas");
        try {
            image.src = url;
            await image.decode();
            if (image.naturalWidth !== width * scale || image.naturalHeight !== height * scale) {
                throw new Error("SVG must decode at its declared reference resolution.");
            }
            canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
            const context = canvas.getContext("2d");
            if (!context) throw new Error("The independent SVG decoder requires Canvas2D.");
            context.imageSmoothingEnabled = false;
            context.drawImage(image, 0, 0); // Intrinsic 1:1; no canvas downscale filter.
            return context.getImageData(0, 0, canvas.width, canvas.height).data;
        } finally {
            image.removeAttribute("src");
            canvas.width = 0; canvas.height = 0;
            URL.revokeObjectURL(url);
        }
    }, { source, width, height, scale });
    try {
        return await page.evaluate(areaAverageAlpha, { pixels, width, height, scale });
    } finally {
        await pixels.dispose();
    }
}

// Author through the native inspector, export through the public FerroUI path.
export async function cornerRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const edit = async value => {
        await clickNative(page, "CornerSmoothingField", "InspectorViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await page.keyboard.press("Enter");
        await frames(page);
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        name: "Continuous corners", x: 40, y: 40, width: 200, height: 140,
        corner_radius: 40, corner_radii: null, corner_smoothing: 0, effects: [], strokes: [],
        fills: [{ paint: { type: "solid", r: 77, g: 63, b: 230, a: 255 }, opacity: 1, visible: true }],
    })], "Native corner smoothing"));
    await clickWorld(page, 140, 100);
    const before = await documentState(page), depth = (await state(page)).historyLength;
    await edit("60");
    const after = await documentState(page);
    assert.equal(after.nodes[2].corner_smoothing, 0.6);
    assert.equal((await state(page)).historyLength, depth + 1);
    await edit("101");
    assert.deepEqual(await documentState(page), after);
    assert.equal((await state(page)).historyLength, depth + 1);
    assert.equal((await state(page)).chrome.CornerSmoothingField.text, "101");
    await page.keyboard.press("Escape");
    await frames(page);
    assert.equal((await state(page)).chrome.CornerSmoothingField.text, "60");
    await captureNativeScreenshot(mode + "-corner-smoothing", { controls: ["CornerSmoothingField"] });
    await clickWorld(page, 140, 100);
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), before);
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), after);
    check("Native smoothing commits one history step, rejects out-of-range drafts atomically, and restores exact document fields through undo/redo");

    const download = async (button, suffix) => {
        const [item] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, button, "InspectorViewport")]);
        assert.equal(await item.failure(), null);
        const path = resolve(output, mode + "-smoothed-corners." + suffix);
        await item.saveAs(path);
        return readFile(path);
    };
    const png = PNG.sync.read(await download("ExportPngButton", "png"));
    const svg = (await download("ExportSvgButton", "svg")).toString("utf8");
    assert.deepEqual([png.width, png.height], [200, 140]);
    assert.equal(png.data[3], 0);
    assert.equal(png.data[(70 * png.width + 100) * 4 + 3], 255);
    const external = await page.evaluate(async source => {
        const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
        const paths = [...parsed.querySelectorAll("path")].map(p => p.getAttribute("d"));
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        const image = new Image(), canvas = document.createElement("canvas");
        try {
            image.src = url; await image.decode();
            canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
            return { width: canvas.width, height: canvas.height, paths,
                alpha: Array.from({ length: pixels.length / 4 }, (_, i) => pixels[i * 4 + 3]) };
        } finally {
            image.removeAttribute("src");
            canvas.width = 0; canvas.height = 0;
            URL.revokeObjectURL(url);
        }
    }, svg);
    assert.deepEqual([external.width, external.height], [200, 140]);
    assert.ok(external.paths.some(path => (path.match(/C/g) || []).length === 12),
        "SVG retains the four cubic/arc/cubic corners");
    // Preserve the original 1x evidence, but do not use its unverified edge AA
    // samples as a pixel-area oracle. Both fixed references must converge.
    console.log("[corners-alpha " + mode + "] " + JSON.stringify(alphaDiagnostics(png, external.alpha)));
    const coarse = await areaAveragedSvgAlpha(page, svg, png.width, png.height, 8);
    const fine = await areaAveragedSvgAlpha(page, svg, png.width, png.height, 16);
    const convergence = coverageConvergence(coarse, fine);
    console.log("[corners-coverage-convergence " + mode + "] " + JSON.stringify({ scales: [8, 16], ...convergence }));
    assert.ok(convergence.converged, "Independent SVG area coverage must converge within 4 alpha units without opposite opaque/transparent pixels");
    const referenceAlpha = fine.map(alpha => Math.round(alpha));
    const comparison = alphaDiagnostics(png, referenceAlpha);
    console.log("[corners-area-alpha " + mode + "] " + JSON.stringify({ referenceScale: 16, ...comparison }));
    const edgeDifferences = comparison.differingOver12;
    assert.ok(edgeDifferences <= 48, "Independent SVG decoder and FerroUI PNG share the smooth silhouette: " + edgeDifferences);
    assert.deepEqual(await documentState(page), after, "Both exports preserve editable corner parameters");
    check("Public FerroUI PNG and independently decoded SVG retain the same twelve-cubic smoothed silhouette and transparent exterior");
}
