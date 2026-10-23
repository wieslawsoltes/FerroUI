import assert from "node:assert/strict";
import { PNG } from "pngjs";

function cropPixels(bytes, bounds) {
    const png = PNG.sync.read(bytes);
    assert.ok(bounds.left >= 0 && bounds.top >= 0 &&
        bounds.left + bounds.width <= png.width && bounds.top + bounds.height <= png.height,
    "The fixed document crop must be inside the actual screenshot");
    const pixels = Buffer.alloc(bounds.width * bounds.height * 4);
    for (let y = 0; y < bounds.height; y++) {
        const start = ((bounds.top + y) * png.width + bounds.left) * 4;
        png.data.copy(pixels, y * bounds.width * 4, start, start + bounds.width * 4);
    }
    return pixels;
}

function compareTextPixels(before, after) {
    assert.equal(after.length, before.length);
    // The fixture's saturated pink separates actual glyph/decorations ink from
    // the neutral canvas/grid. Evaluate error against that ink as well as the
    // crop, so blank space cannot conceal a moved baseline or missing glyph.
    const hasInk = (pixels, i) => pixels[i] - pixels[i + 1] > 24 &&
        pixels[i + 2] - pixels[i + 1] > 12;
    let beforeInk = 0, afterInk = 0, inkUnion = 0;
    let differingPixels = 0, largeDifferences = 0, channelError = 0;
    for (let i = 0; i < before.length; i += 4) {
        const first = hasInk(before, i), second = hasInk(after, i);
        beforeInk += Number(first); afterInk += Number(second);
        inkUnion += Number(first || second);
        let maximum = 0;
        for (let channel = 0; channel < 4; channel++) {
            const delta = Math.abs(before[i + channel] - after[i + channel]);
            maximum = Math.max(maximum, delta);
            if (channel < 3) channelError += delta;
        }
        differingPixels += Number(maximum > 4);
        largeDifferences += Number(maximum > 32);
    }
    const metrics = {
        pixels: before.length / 4, beforeInk, afterInk, inkUnion,
        differingPixels, largeDifferences,
        meanInkChannelError: channelError / Math.max(1, inkUnion * 3),
    };
    const evidence = JSON.stringify(metrics);
    assert.ok(beforeInk > 200 && afterInk > 200, "Both images must contain real text ink: " + evidence);
    assert.ok(differingPixels < metrics.pixels / 100,
        "Flatten must retain the native >4/255, <1% pixel gate: " + evidence);
    assert.ok(largeDifferences < inkUnion / 100,
        "Fewer than 1% of text-ink pixels may differ by >32/255: " + evidence);
    assert.ok(metrics.meanInkChannelError <= 1,
        "Mean RGB error normalized to text ink must be at most 1/255: " + evidence);
    return metrics;
}

export async function flattenRegressions(page, reference, check, helpers, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const fill = (r, g, b) => ({ paint: { type: "solid", r, g, b, a: 255 }, opacity: 1, visible: true });
    const action = async (rejection = null) => {
        await clickNative(page, "ActionsButton");
        await clickNative(page, "ActionSearch", "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText("Flatten to vector");
        await page.waitForFunction(() => {
            const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
            const visible = Object.entries(chrome)
                .filter(([name, bounds]) => name.startsWith("Action-") && bounds.visible);
            return chrome.ActionSearch?.text === "Flatten to vector" && visible.length === 1 &&
                visible[0][0] === "Action-flatten" &&
                ["ModalPanel", "ModalContent", "ModalViewport"].every(name =>
                    chrome[name]?.measureValid && chrome[name]?.arrangeValid);
        });
        // Live modal bounds can precede compositor hit-test publication. One
        // older coalesced completion may still be pending; two completions
        // after the final layout also cover that case. The focused native
        // search caret continues publishing scenes while we wait.
        const filtered = await state(page);
        if (rejection !== null) {
            assert.equal(filtered.chrome.Toast?.visible === true &&
                filtered.chrome.ToastText?.text === rejection, false,
            "The expected rejection must not already be visible before invoking Flatten");
        }
        await waitForNativeScene(page, filtered.sceneFrames + 1, "Filtered Flatten action");
        await clickNative(page, "Action-flatten", "ModalViewport");
        // Failed Flatten intentionally retains the palette. Require its exact
        // native error, so a missed click cannot satisfy unchanged-state tests.
        await page.waitForFunction(rejection => {
            const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
            return rejection === null ? chrome.ModalBackdrop?.visible === false :
                chrome.Toast?.visible === true && chrome.ToastText?.text === rejection;
        }, rejection, { timeout: 10000 });
    };
    const firstRoot = async (id) => {
        await page.keyboard.press("Escape");
        await page.keyboard.press("Tab");
        await page.waitForFunction((id) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection.join() === String(id), id);
    };
    const roundTrip = async (before, after, depth) => {
        assert.equal((await state(page)).historyLength, depth + 1, "Flatten records exactly one transaction");
        await page.keyboard.press("Control+z"); await frames(page);
        assert.deepEqual(await documentState(page), before, "Flatten undo restores exact original document");
        await page.keyboard.press("Control+Shift+z"); await frames(page);
        assert.deepEqual(await documentState(page), after, "Flatten redo restores every outlined field");
    };

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "text", name: "Shaped outline", x: 40, y: 40,
            width: 240, height: 130, text: "office\nFerroUI", font_family: "Inter", font_size: 32,
            text_align: "center", text_decoration: "underline", letter_spacing: 1.5,
            text_runs: [], fills: [fill(210, 50, 100)], rotation: 8, corner_radius: 0 }),
    ], "Native flatten text"));
    await firstRoot(2);
    const textBefore = await documentState(page), textDepth = (await state(page)).historyLength;
    await action();
    const textAfter = await documentState(page), outlined = textAfter.nodes[2];
    assert.equal(outlined.kind, "vector");
    assert.equal(outlined.parent, textBefore.nodes[2].parent);
    assert.deepEqual(textAfter.pages[0].children, [2]);
    assert.deepEqual(outlined.fills, textBefore.nodes[2].fills);
    assert.equal(outlined.rotation, 8);
    assert.ok(outlined.path.segments.length > 20, "Shaped glyphs and decorations become editable curve commands");
    await captureNativeScreenshot(mode + "-flatten-text", { controls: ["CanvasHost"] });
    await roundTrip(textBefore, textAfter, textDepth);
    check("Native Flatten to vector converts real shaped, rotated multiline text and decorations through public FerroUI outlines while retaining identity, paint, parent and one-step undo/redo");

    // Exercise the visible browser surface, including its fractional camera.
    // PNG export alone would use an independent CPU surface at one pixel per
    // document unit and could miss a WebGL2 glyph/geometry discrepancy.
    const fractionalText = fixtureNode(reference, 2, {
        kind: "text", name: "Fractional rich text ink", x: 40.25, y: 40.5,
        width: 260, height: 152, text: "office\nAV ffi", font_family: "Inter",
        font_size: 32.5, font_weight: 400, line_height: 1.4, letter_spacing: 0.375,
        text_auto_resize: "fixed", text_align: "center", text_vertical_align: "center",
        text_decoration: "underline", rotation: 0, corner_radius: 0,
        fills: [fill(210, 50, 100)],
        text_runs: [{ start: 0, end: 6, font_family: null, font_size: 36.25,
            font_weight: 600, color: null, italic: null, decoration: "strikethrough" }],
    });
    const fractionalFixture = isolatedDocument(reference, [fractionalText], "Browser flatten pixel parity");
    fractionalFixture.pages[0].background = { r: 255, g: 255, b: 255, a: 255 };
    await importFixture(page, fractionalFixture);
    for (let attempt = 0; (await state(page)).zoom > 1.5 && attempt < 16; attempt++) {
        const previous = await state(page);
        await clickNative(page, "ZoomOutButton");
        await waitForNativeFrame(page, previous.renderedFrames, "Fractional text zoom");
    }
    let camera = await state(page);
    if (Math.abs(camera.zoom - Math.round(camera.zoom)) < 0.001) {
        await clickNative(page, "ZoomOutButton");
        await waitForNativeFrame(page, camera.renderedFrames, "Noninteger text zoom");
    }
    await clickWorld(page, 10, 10);
    const beforePan = await state(page), host = beforePan.chrome.CanvasHost;
    const start = { x: host.x + host.width * 0.7, y: host.y + host.height * 0.7 };
    await page.mouse.move(start.x, start.y);
    await page.keyboard.down("Space");
    try {
        await page.mouse.down();
        try { await page.mouse.move(start.x + 17.375, start.y + 9.625, { steps: 4 }); }
        finally { await page.mouse.up(); }
    } finally { await page.keyboard.up("Space"); }
    await waitForNativeFrame(page, beforePan.renderedFrames, "Fractional text pan");
    camera = await state(page);
    assert.notDeepEqual(camera.pan, beforePan.pan, "The native pan must change the camera");
    assert.ok(camera.zoom > 0.5 && camera.zoom <= 1.5 &&
        Math.abs(camera.zoom - Math.round(camera.zoom)) > 0.001,
    "The actual browser surface must use a noninteger scale: " + camera.zoom);
    const cameraIdentity = (view) => ({
        zoom: view.zoom, pan: view.pan,
        host: { x: view.chrome.CanvasHost.x, y: view.chrome.CanvasHost.y,
            width: view.chrome.CanvasHost.width, height: view.chrome.CanvasHost.height },
    });
    const fixedCamera = cameraIdentity(camera);
    const screen = (x, y) => ({
        x: fixedCamera.host.x + fixedCamera.pan.x + x * fixedCamera.zoom,
        y: fixedCamera.host.y + fixedCamera.pan.y + y * fixedCamera.zoom,
    });
    const topLeft = screen(fractionalText.x - 4, fractionalText.y - 4);
    const bottomRight = screen(fractionalText.x + fractionalText.width + 4,
        fractionalText.y + fractionalText.height + 4);
    const crop = { left: Math.floor(topLeft.x), top: Math.floor(topLeft.y),
        width: Math.ceil(bottomRight.x) - Math.floor(topLeft.x),
        height: Math.ceil(bottomRight.y) - Math.floor(topLeft.y) };
    assert.ok(crop.left >= host.x && crop.top >= host.y &&
        crop.left + crop.width <= host.x + host.width && crop.top + crop.height <= host.y + host.height,
    "The complete text crop must remain inside the native canvas");
    for (const name of ["Toolbar", "ZoomPanel"]) {
        const overlay = camera.chrome[name];
        if (!overlay || overlay.visible === false) continue;
        assert.ok(crop.left + crop.width <= overlay.x || crop.left >= overlay.x + overlay.width ||
            crop.top + crop.height <= overlay.y || crop.top >= overlay.y + overlay.height,
        name + " must not obscure the text pixel comparison");
    }
    const captureText = async (suffix) => {
        // Pointer exit alone does not clear the canvas hover highlight. Move
        // outside first, then use native Escape to clear hover and selection.
        await page.mouse.move(4, 4);
        const previous = await state(page);
        await page.keyboard.press("Escape");
        await waitForNativeFrame(page, previous.renderedFrames, "Clear text capture adorners");
        const view = await state(page);
        assert.deepEqual(cameraIdentity(view), fixedCamera, "Every text capture uses the identical camera");
        assert.equal(view.selectionCount, 0, "Selection adorners must not enter the pixel comparison");
        assert.equal(view.editingText, null, "A native text caret must not enter the pixel comparison");
        assert.equal(view.vectorNode, null, "Vector handles must not enter the pixel comparison");
        assert.equal(view.chrome.ModalBackdrop.visible, false, "The action palette must be closed");
        const bytes = await captureNativeScreenshot(mode + "-flatten-rich-" + suffix,
            { controls: ["CanvasHost"], afterFrame: previous.renderedFrames });
        assert.deepEqual(cameraIdentity(await state(page)), fixedCamera);
        return cropPixels(bytes, crop);
    };
    const fractionalBeforePixels = await captureText("before");
    const fractionalBefore = await documentState(page), fractionalDepth = (await state(page)).historyLength;
    await firstRoot(2);
    await action();
    const fractionalAfter = await documentState(page);
    assert.equal(fractionalAfter.nodes[2].kind, "vector");
    assert.ok(fractionalAfter.nodes[2].path.segments.length > 20);
    assert.equal((await state(page)).historyLength, fractionalDepth + 1);
    const fractionalAfterPixels = await captureText("after");
    const pixelMetrics = compareTextPixels(fractionalBeforePixels, fractionalAfterPixels);
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), fractionalBefore, "Pixel fixture undo restores every document field");
    assert.deepEqual(await captureText("undo"), fractionalBeforePixels,
        "Undo must restore the exact pre-Flatten glyph/decorations pixels at the same fractional camera");
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), fractionalAfter, "Pixel fixture redo restores every outlined field");
    assert.deepEqual(await captureText("redo"), fractionalAfterPixels,
        "Redo must restore the exact post-Flatten vector pixels at the same fractional camera");
    check("Actual " + mode + " surface pixels retain fractional rich multiline text, underline and strikethrough through Flatten, with byte-exact Undo/Redo captures: " +
        JSON.stringify({ zoom: fixedCamera.zoom, pan: fixedCamera.pan, crop, ...pixelMetrics }));

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "boolean", name: "Outlined difference", x: 40, y: 40,
            width: 200, height: 140, children: [3, 4], boolean_operation: "subtract", fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { parent: 2, name: "Outer shape", x: 0, y: 0,
            width: 200, height: 140, fills: [fill(30, 170, 100)], corner_radius: 0 }),
        fixtureNode(reference, 4, { parent: 2, kind: "ellipse", name: "Hole", x: 60, y: 30,
            width: 80, height: 80, fills: [fill(230, 90, 30)], corner_radius: 0 }),
    ], "Native flatten boolean"));
    await firstRoot(2);
    const booleanBefore = await documentState(page), booleanDepth = (await state(page)).historyLength;
    await action();
    const booleanAfter = await documentState(page);
    assert.equal(booleanAfter.nodes[2].kind, "vector");
    assert.deepEqual(Object.keys(booleanAfter.nodes), ["2"]);
    assert.deepEqual(booleanAfter.nodes[2].fills, booleanBefore.nodes[3].fills);
    assert.ok(booleanAfter.nodes[2].path.segments.length >= 8);
    await roundTrip(booleanBefore, booleanAfter, booleanDepth);
    check("Native Flatten evaluates Boolean difference into editable curves, retains the operand-inherited fill and surviving ID, removes operands atomically, and exactly restores them on Undo");

    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { name: "Red region", x: 40, y: 40, width: 80, height: 80,
            fills: [fill(220, 40, 60)], corner_radius: 0 }),
        fixtureNode(reference, 3, { name: "Blue region", x: 160, y: 40, width: 80, height: 80,
            fills: [fill(20, 120, 230)], corner_radius: 0 }),
    ], "Atomic flatten rejection"));
    await clickWorld(page, 70, 70);
    await page.keyboard.down("Shift");
    try { await clickWorld(page, 190, 70); } finally { await page.keyboard.up("Shift"); }
    const invalidBefore = await documentState(page), invalidDepth = (await state(page)).historyLength;
    assert.equal((await state(page)).selectionCount, 2);
    await action("Different layer colors need independently painted vector regions; flatten those layers separately.");
    assert.deepEqual(await documentState(page), invalidBefore, "A heterogeneous paint batch must never discard one region's paint");
    assert.equal((await state(page)).historyLength, invalidDepth);
    check("Native Flatten rejects independently colored regions without changing the document or history instead of silently discarding their appearance");
}
