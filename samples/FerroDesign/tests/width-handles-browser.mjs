import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";

// Starting geometry is imported; edits use native controls and captured input.
export async function widthHandleRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickNative, clickWorld, moveWorld, frames, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const near = (a, b, label) => assert.ok(Math.abs(a - b) < 1e-7, `${label}: ${a} != ${b}`);
    const wait = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("view", "document", "args", "return (" + source + ")(view,document,args)")(view, document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const stroke = { paint: { type: "solid", r: 37, g: 99, b: 235, a: 255 },
        width: 20, opacity: 1, visible: true, align: "center", cap: "butt", join: "round", dash: [] };
    const inputs = [];
    const beginGesture = async start => {
        await moveWorld(page, ...start);
        const before = await state(page);
        await page.evaluate(() => {
            if (globalThis.__ferroDesignWidthInput) throw new Error("A width input observer is already active");
            const observed = { down: null, move: null, count: 0 };
            const record = event => {
                if (!(event.buttons & 1)) return;
                if (++observed.count > 64) return;
                observed[event.type === "pointerdown" ? "down" : "move"] = {
                    trusted: event.isTrusted, x: event.offsetX, y: event.offsetY,
                    clientX: event.clientX, clientY: event.clientY,
                };
            };
            document.addEventListener("pointerdown", record, { capture: true, passive: true });
            document.addEventListener("pointermove", record, { capture: true, passive: true });
            globalThis.__ferroDesignWidthInput = { observed, stop() {
                document.removeEventListener("pointerdown", record, true);
                document.removeEventListener("pointermove", record, true);
                delete globalThis.__ferroDesignWidthInput;
            } };
        });
        await page.mouse.down();
        await wait(view => view.strokeWidth.active);
        const view = await state(page), press = view.pointerPresses.entries.at(-1);
        assert.ok(press.serial > before.pointerPresses.serial && press.buttons === 1,
            "The width expectation requires its actual routed native press");
        return press;
    };
    const moveGesture = async (press, end) => {
        await moveWorld(page, ...end);
        const view = await state(page);
        const observed = await page.evaluate(() => globalThis.__ferroDesignWidthInput.observed);
        assert.ok(observed.count <= 64 && observed.down?.trusted && observed.move?.trusted,
            "Width input must contain bounded real browser press/move observations");
        assert.equal(view.zoom, press.zoom); assert.deepEqual(view.pan, press.pan);
        near(observed.down.x - observed.down.clientX, observed.move.x - observed.move.clientX, "Pointer X coordinate origin");
        near(observed.down.y - observed.down.clientY, observed.move.y - observed.move.clientY, "Pointer Y coordinate origin");
        // Browser/CDP delivery can quantize requested fractional coordinates.
        // Derive the geometric expectation from the delivered input, anchored
        // to its observed native press, without reading the resulting profile.
        const delta = { x: (observed.move.x - observed.down.x) / press.zoom,
            y: (observed.move.y - observed.down.y) / press.zoom };
        const input = { press, observed, delta, end: { x: press.world.x + delta.x, y: press.world.y + delta.y } };
        inputs.push(input);
        await writeFile(resolve(output, mode + "-stroke-width-input.json"), JSON.stringify(inputs, null, 2));
        return input;
    };
    const finishObservation = () => page.evaluate(() => globalThis.__ferroDesignWidthInput?.stop());
    const enter = async (x, y) => {
        await clickNative(page, "SelectTool");
        await clickWorld(page, x, y);
        assert.deepEqual((await state(page)).selection, [2]);
        const before = await state(page);
        await clickNative(page, "StrokeWidthHandlesButton", "InspectorViewport");
        await wait(view => view.strokeWidth.node === 2 && view.chrome.CanvasHost.focusWithin);
        await waitForNativeFrame(page, before.renderedFrames, "Enter native width handles");
        await waitForNativeScene(page, before.sceneFrames, "Enter native width handles");
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let view = await state(page); await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        await waitForNativeScene(page, view.sceneFrames, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " exact undo");
        view = await state(page); await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        await waitForNativeScene(page, view.sceneFrames, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " exact redo");
    };
    const gesture = async (start, end, cancel = false) => {
        let input;
        try {
            const press = await beginGesture(start);
            input = await moveGesture(press, end);
            if (cancel) await page.keyboard.press("Escape");
        } finally { try { await page.mouse.up(); } finally { await finishObservation(); } }
        await wait(view => !view.strokeWidth.active); await frames(page);
        return input;
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "line", name: "Native width handles", x: 80, y: 100, width: 200, height: 0,
        fills: [], strokes: [stroke], corner_radius: 0, corner_radii: null,
    })], "Native width point gestures"));
    await enter(180, 100);
    let before = await documentState(page), view = await state(page), depth = view.historyLength;
    await moveWorld(page, 180, 100); await wait(view => view.strokeWidth.hover);
    const hover = await state(page);
    assert.equal(hover.documentRevision, view.documentRevision);
    assert.equal(hover.historyLength, depth);
    assert.deepEqual(await documentState(page), before);
    const insertionInput = await gesture([180, 100], [180, 115]);
    let after = await documentState(page);
    const insertedProfile = after.nodes[2].strokes[0].width_profile;
    assert.equal(insertedProfile.type, "custom"); assert.equal(insertedProfile.points.length, 3);
    assert.deepEqual([insertedProfile.points[0], insertedProfile.points[2]], [{ position: 0, width: 1 }, { position: 1, width: 1 }]);
    near(insertedProfile.points[1].position, (insertionInput.press.world.x - 80) / 200, "Actual insertion position");
    near(insertedProfile.points[1].width, 1 + insertionInput.delta.y / 10, "Actual normal drag width");
    assert.deepEqual(after.nodes[2].path, before.nodes[2].path);
    assert.equal(after.nodes[2].kind, "line");
    await captureNativeScreenshot(mode + "-stroke-width-handles", { controls: ["CanvasHost"] });
    await history(before, after, depth, "Insert and drag native width point");
    check("Native width handles enter without converting the source line; hover preserves revision/history, and a captured insertion plus normal drag creates one exact undoable custom profile");

    before = await documentState(page); depth = (await state(page)).historyLength;
    const positionInput = await gesture([180, 100], [220, 100]);
    after = await documentState(page);
    near(after.nodes[2].strokes[0].width_profile.points[1].position, (positionInput.end.x - 80) / 200, "Center handle moves along contour");
    assert.equal(after.nodes[2].strokes[0].width_profile.points[1].width,
        before.nodes[2].strokes[0].width_profile.points[1].width, "Center handle retains the exact authored width");
    await history(before, after, depth, "Move native width point along path");
    before = await documentState(page); depth = (await state(page)).historyLength;
    await page.keyboard.press("Delete"); await wait((view, doc) => doc.nodes[2].strokes[0].width_profile.points.length === 2);
    after = await documentState(page);
    assert.equal(after.nodes[2].kind, "line", "Delete removes a width point, not the layer");
    await history(before, after, depth, "Delete native width point");
    before = await documentState(page); depth = (await state(page)).historyLength;
    await gesture([180, 100], [180, 125], true);
    assert.deepEqual(await documentState(page), before);
    assert.equal((await state(page)).historyLength, depth);
    assert.equal((await state(page)).strokeWidth.node, 2, "Escape cancels before leaving the width editor");
    check("Center-handle dragging preserves width while changing normalized position; Delete authors one point-removal edit and Escape rolls a held width gesture back without changing the artwork or history");

    const flush = () => page.evaluate(async () => {
        let timer;
        try {
            const saved = await Promise.race([globalThis.ferroDesignStorage.flush(),
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Width lifecycle storage flush exceeded 30 seconds")), 30000); })]);
            return { saved, status: globalThis.ferroDesignStorage.status(),
                document: JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text) };
        } finally { clearTimeout(timer); }
    });
    const baseline = await flush();
    assert.equal(baseline.saved, true); assert.equal(baseline.status.dirty, false);
    assert.deepEqual(baseline.document, before);
    let lifecycle;
    try {
        const press = await beginGesture([180, 100]);
        const input = await moveGesture(press, [180, 120]);
        const expectedWidth = 1 + input.delta.y / 10;
        await wait((_view, doc, expected) => Math.abs(doc.nodes[2].strokes[0].width_profile.points[1].width - expected) < 1e-7, expectedWidth);
        lifecycle = await documentState(page);
        assert.equal((await state(page)).historyLength, depth);
        assert.equal(await page.evaluate(() => globalThis.ferroDesignStorage.status().draftDirty), true);
        await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
        await wait(view => !view.strokeWidth.active);
        assert.equal((await state(page)).historyLength, depth + 1);
        const saved = await flush();
        assert.equal(saved.saved, true); assert.equal(saved.status.dirty, false); assert.equal(saved.status.draftDirty, false);
        assert.deepEqual(saved.document, lifecycle);
    } finally { try { await page.mouse.up(); } finally { await finishObservation(); } }
    await frames(page);
    assert.equal((await state(page)).historyLength, depth + 1);
    assert.deepEqual(await documentState(page), lifecycle);
    await history(before, lifecycle, depth, "Lifecycle width gesture");
    check("A held width drag marks the host draft from a clean saved baseline; pagehide commits and saves every field to IndexedDB once, and a late pointer release cannot duplicate its history record");

    const path = { fill_rule: "non_zero", segments: [
        { type: "move_to", point: { x: 0, y: 0 } }, { type: "line_to", point: { x: 100, y: 0 } },
        { type: "move_to", point: { x: 0, y: 100 } }, { type: "line_to", point: { x: 200, y: 100 } },
    ] };
    const repeated = { type: "custom", points: [{ position: 0, width: 1 }, { position: 0.5, width: 2 }, { position: 1, width: 1 }] };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "vector", name: "Repeated transformed width profile", x: 400, y: 80, width: 200, height: 100,
        affine_transform: { a: -1.2, b: 0.3, c: 0.4, d: 0.9, e: 0, f: 0 },
        fills: [], strokes: [{ ...stroke, width_profile: repeated }], path, vector_network: null,
    })], "Affine repeated width handles"));
    const world = (x, y) => [400 - 1.2 * x + 0.4 * y, 80 + 0.3 * x + 0.9 * y];
    await enter(...world(100, 100));
    assert.equal((await state(page)).strokeWidth.contours, 2);
    before = await documentState(page); depth = (await state(page)).historyLength;
    const affineWidthInput = await gesture(world(100, 120), world(100, 130));
    after = await documentState(page);
    const localDeltaY = (-0.3 * affineWidthInput.delta.x - 1.2 * affineWidthInput.delta.y) / (-1.2 * 0.9 - 0.3 * 0.4);
    near(after.nodes[2].strokes[0].width_profile.points[1].width, 2 + localDeltaY / 10, "Affine normal-offset width");
    assert.equal((await state(page)).strokeWidth.contour, 1);
    assert.deepEqual(after.nodes[2].path, before.nodes[2].path);
    assert.deepEqual(after.nodes[2].affine_transform, before.nodes[2].affine_transform);
    await history(before, after, depth, "Reflected sheared contour width");
    before = await documentState(page); depth = (await state(page)).historyLength;
    const affinePositionInput = await gesture(world(100, 100), world(150, 100));
    after = await documentState(page);
    const [originX, originY] = world(0, 100), direction = { x: -240, y: 60 };
    const expectedPosition = ((affinePositionInput.end.x - originX) * direction.x +
        (affinePositionInput.end.y - originY) * direction.y) / (direction.x ** 2 + direction.y ** 2);
    near(after.nodes[2].strokes[0].width_profile.points[1].position, expectedPosition, "Second contour normalized position");
    assert.deepEqual(after.nodes[2].path, path);
    await captureNativeScreenshot(mode + "-stroke-width-multiple-contours", { controls: ["CanvasHost"] });
    await history(before, after, depth, "Repeated contour position");
    await page.keyboard.press("Escape");
    await wait(view => view.strokeWidth.node === null);
    check("Width points repeat over independent contours; native side and center drags on a reflected/sheared second contour preserve the source paths and transform while editing one shared normalized profile with exact history");
}
