import assert from "node:assert/strict";

// Real native pointer ownership and track controls. Fixture import only sets
// the initial document; every operation under test uses mouse or keyboard.
export async function layoutTransferRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const sizing = (extra = {}) => ({
        width: "fixed", height: "fixed", min_width: 0, max_width: null,
        min_height: 0, max_height: null, absolute: false, grow: 1, ...extra,
    });
    const layout = (axis, extra = {}) => ({
        axis, columns: 3, gap: 8, cross_gap: 9,
        padding: { top: 10, right: 10, bottom: 10, left: 10 },
        main_align: "start", cross_align: "start", width: "fixed", height: "fixed",
        reverse: false, wrap: false, ...extra,
    });
    const close = (actual, expected, label) => assert.ok(Math.abs(actual - expected) < 1e-6,
        label + ": " + actual + " != " + expected);
    const screen = (view, point) => ({
        x: view.chrome.CanvasHost.x + view.pan.x + point.x * view.zoom,
        y: view.chrome.CanvasHost.y + view.pan.y + point.y * view.zoom,
    });
    const waitDrag = (active) => page.waitForFunction((active) => {
        const drag = JSON.parse(globalThis.ferroDesign.ferroDesignState()).layoutDrag;
        return active ? drag?.active === true && drag.target !== null : drag === null;
    }, active, { timeout: 10000 });
    const drag = async (from, to) => {
        const view = await state(page), start = screen(view, from), end = screen(view, to);
        await page.mouse.move(start.x, start.y);
        await page.mouse.down();
        await page.mouse.move(end.x, end.y, { steps: 16 });
        await waitDrag(true);
        return { start, end, view, preview: (await state(page)).layoutDrag };
    };
    const exactHistory = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " has one history transaction");
        await page.keyboard.press("Control+z");
        await frames(page);
        assert.deepEqual(await documentState(page), before, label + " undo restores every field");
        assert.equal((await state(page)).historyLength, depth);
        await page.keyboard.press("Control+Shift+z");
        await frames(page);
        assert.deepEqual(await documentState(page), after, label + " redo restores every field");
        assert.equal((await state(page)).historyLength, depth + 1);
    };

    const affine = { a: 1.1, b: 0.2, c: 0.25, d: 0.9, e: 0, f: 0 };
    const destinationPoint = (x, y) => ({ x: 400 + affine.a * x + affine.c * y, y: 40 + affine.b * x + affine.d * y });
    const multiNodes = [
        fixtureNode(reference, 2, { kind: "frame", name: "Source row", x: 40, y: 40, width: 260, height: 160,
            children: [3, 4, 5, 6], layout: layout("row"), fills: [], corner_radius: 0 }),
        ...[3, 4, 5, 6].map((id, index) => fixtureNode(reference, id, {
            parent: 2, name: "Source " + index, x: 10 + index * 48, y: 10, width: 40, height: 24,
            layout_child: sizing(),
        })),
        fixtureNode(reference, 10, { kind: "frame", name: "Affine destination", x: 400, y: 40, width: 260, height: 180,
            affine_transform: affine, children: [11, 12], layout: layout("column"), fills: [], corner_radius: 0 }),
        ...[11, 12].map((id, index) => fixtureNode(reference, id, {
            parent: 10, name: "Destination " + index, x: 10, y: 10 + index * 32, width: 40, height: 24,
            layout_child: sizing(),
        })),
    ];
    await importFixture(page, isolatedDocument(reference, multiNodes, "Multi-frame transfer"));
    await clickWorld(page, 214, 62);
    await page.keyboard.down("Shift");
    await clickWorld(page, 118, 62);
    await page.keyboard.up("Shift");
    assert.deepEqual((await state(page)).selection, [6, 4], "Selection click order differs from visual flow");
    const beforeMulti = await documentState(page), multiDepth = (await state(page)).historyLength;
    const multi = await drag({ x: 118, y: 62 }, destinationPoint(30, 39));
    assert.deepEqual(multi.preview.selection, [6, 4]);
    assert.equal(multi.preview.target.parent, 10);
    assert.equal(multi.preview.target.kind, "flow");
    assert.equal(multi.preview.target.index, 1);
    for (const [endpoint, point] of [["start", destinationPoint(10, 42)], ["end", destinationPoint(50, 42)]]) {
        close(multi.preview.target.marker[endpoint].x, point.x, "Affine insertion " + endpoint + " x");
        close(multi.preview.target.marker[endpoint].y, point.y, "Affine insertion " + endpoint + " y");
    }
    assert.deepEqual(await documentState(page), beforeMulti, "Multi-frame preview cannot mutate either frame");
    assert.equal((await state(page)).historyLength, multiDepth);
    await captureNativeScreenshot(mode + "-multi-frame-layout-transfer", {
        controls: ["CanvasHost"], afterFrame: multi.view.renderedFrames,
    });
    await page.mouse.up();
    await waitDrag(false);
    const afterMulti = await documentState(page);
    assert.deepEqual(afterMulti.nodes[2].children, [3, 5]);
    assert.deepEqual(afterMulti.nodes[10].children, [11, 4, 6, 12], "Visual order survives opposite selection click order");
    assert.deepEqual((await state(page)).selection, [6, 4]);
    for (const [index, id] of [11, 4, 6, 12].entries()) {
        assert.equal(afterMulti.nodes[id].parent, 10);
        close(afterMulti.nodes[id].x, 10, "Transferred local x");
        close(afterMulti.nodes[id].y, 10 + index * 32, "Transferred local y");
    }
    await exactHistory(beforeMulti, afterMulti, multiDepth, "Affine multi-child transfer");
    check("Native multi-selection drags insert into an affine frame in visual order, keep both models immutable during preview, preserve selection, and restore all hierarchy/geometry with one undo/redo");

    const freeNodes = [
        fixtureNode(reference, 2, { name: "Free source", x: 20, y: 40, width: 30, height: 24, layout_child: sizing() }),
        fixtureNode(reference, 10, { kind: "frame", name: "Grid destination", x: 260, y: 40, width: 220, height: 180,
            children: [11, 12], layout: layout("grid", { columns: 2, gap: 10, cross_gap: 10 }), fills: [], corner_radius: 0 }),
        ...[11, 12].map((id, index) => fixtureNode(reference, id, {
            parent: 10, name: "Grid " + index, x: 10 + index * 105, y: 10, width: 60, height: 24, layout_child: sizing(),
        })),
    ];
    await importFixture(page, isolatedDocument(reference, freeNodes, "Free-to-grid handoff"));
    await clickWorld(page, 35, 52);
    const beforeFree = await documentState(page), freeDepth = (await state(page)).historyLength;
    const free = await drag({ x: 35, y: 52 }, { x: 372, y: 62 });
    assert.equal(free.preview.target.parent, 10);
    assert.equal(free.preview.target.index, 1);
    assert.deepEqual(await documentState(page), beforeFree, "Entering a layout restores the ordinary free preview before deferred insertion");
    assert.equal((await state(page)).historyLength, freeDepth);
    await page.mouse.up();
    await waitDrag(false);
    const afterFree = await documentState(page);
    assert.deepEqual(afterFree.pages[0].children, [10]);
    assert.deepEqual(afterFree.nodes[10].children, [11, 2, 12]);
    assert.equal(afterFree.nodes[2].parent, 10);
    close(afterFree.nodes[2].x, 115, "Grid insertion x");
    close(afterFree.nodes[2].y, 10, "Grid insertion y");
    close(afterFree.nodes[12].x, 10, "Grid sibling wrap x");
    close(afterFree.nodes[12].y, 44, "Grid sibling wrap y");
    await exactHistory(beforeFree, afterFree, freeDepth, "Free-to-grid handoff");
    await page.keyboard.press("Control+z");
    await frames(page);
    const redoBefore = (await state(page)).canRedo;
    let canceled = await drag({ x: 35, y: 52 }, { x: 372, y: 62 });
    await page.mouse.move(canceled.start.x, canceled.start.y, { steps: 10 });
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).layoutDrag?.target?.kind === "restore");
    await page.mouse.up();
    await waitDrag(false);
    assert.deepEqual(await documentState(page), beforeFree);
    assert.equal((await state(page)).historyLength, freeDepth);
    assert.equal((await state(page)).canRedo, redoBefore);
    canceled = await drag({ x: 35, y: 52 }, { x: 372, y: 62 });
    await page.keyboard.press("Escape");
    await waitDrag(false);
    await page.mouse.move(canceled.end.x + 15, canceled.end.y + 10, { steps: 4 });
    await page.mouse.up();
    assert.deepEqual(await documentState(page), beforeFree);
    assert.equal((await state(page)).historyLength, freeDepth);
    assert.equal((await state(page)).canRedo, redoBefore);
    check("Native free movement hands off into grid insertion with one history transaction; return-to-source, Escape and late release preserve document and existing redo");

    const gridNodes = [
        fixtureNode(reference, 2, { kind: "frame", name: "Track authoring", x: 40, y: 40, width: 340, height: 180,
            children: [3, 4, 5], layout: layout("grid", { gap: 10, cross_gap: 5 }), fills: [], corner_radius: 0 }),
        ...[3, 4, 5].map((id, index) => fixtureNode(reference, id, {
            parent: 2, name: "Fill " + index, x: 10 + index * 110, y: 10, width: 100, height: 24,
            layout_child: sizing({ width: "fill" }),
        })),
    ];
    await importFixture(page, isolatedDocument(reference, gridNodes, "Native explicit grid tracks"));
    await clickWorld(page, 44, 44);
    assert.deepEqual((await state(page)).selection, [2]);
    const beforeTracks = await documentState(page), trackDepth = (await state(page)).historyLength;
    const open = () => clickNative(page, "EditLayoutButton", "InspectorViewport");
    const field = async (name, value, commit = true) => {
        await clickNative(page, name, "ModalViewport");
        assert.equal((await state(page)).chrome[name].focused, true);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        if (commit) await page.keyboard.press("Enter");
        await frames(page);
    };
    await open();
    await field("LayoutColumnTracks", "60, 1fr, 2fr");
    const afterTracks = await documentState(page);
    assert.deepEqual(afterTracks.nodes[2].layout.column_tracks,
        [{ kind: "fixed", value: 60 }, { kind: "fraction", value: 1 }, { kind: "fraction", value: 2 }]);
    for (const [index, id] of [3, 4, 5].entries()) {
        close(afterTracks.nodes[id].x, [10, 80, 170][index], "Explicit track x");
        close(afterTracks.nodes[id].width, [60, 80, 160][index], "Explicit track width");
    }
    await captureNativeScreenshot(mode + "-explicit-grid-track-controls", {
        controls: ["ModalContent", "LayoutColumnTracks", "LayoutApplyTracks"],
    });
    const validDepth = (await state(page)).historyLength;
    await field("LayoutColumnTracks", "60, 0fr, 2fr");
    assert.equal((await state(page)).chrome.LayoutError.visible, true);
    assert.deepEqual(await documentState(page), afterTracks);
    assert.equal((await state(page)).historyLength, validDepth);
    await page.keyboard.press("Escape");
    await frames(page);
    assert.equal((await state(page)).chrome.LayoutColumnTracks.text, "60, 1fr, 2fr");
    await clickNative(page, "LayoutDone");
    await exactHistory(beforeTracks, afterTracks, trackDepth, "Native explicit column tracks");
    check("Native fixed and weighted grid-track fields arrange real Fill children, retain invalid drafts without partial mutation, cancel them with Escape, and restore exact one-step history");

    const empty = fixtureNode(reference, 2, { kind: "frame", name: "Empty grid", x: 40, y: 40, width: 340, height: 180,
        children: [], layout: layout("grid", { gap: 10, cross_gap: 5 }), fills: [], corner_radius: 0 });
    await importFixture(page, isolatedDocument(reference, [empty], "Empty grid authoring"));
    await clickWorld(page, 44, 44);
    await open();
    assert.equal((await state(page)).chrome.LayoutColumnTracks.visible, true);
    await field("LayoutColumnTracks", "80, auto, 2fr");
    const beforeRow = await documentState(page), rowDepth = (await state(page)).historyLength;
    await field("LayoutRowTracks", "30, 50", false);
    assert.deepEqual(await documentState(page), beforeRow, "A focused row-track draft has not committed yet");
    await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
    await frames(page);
    const afterRow = await documentState(page);
    assert.deepEqual(afterRow.nodes[2].layout.row_tracks, [{ kind: "fixed", value: 30 }, { kind: "fixed", value: 50 }]);
    assert.equal((await state(page)).historyLength, rowDepth + 1);
    assert.equal(await page.evaluate(() => globalThis.ferroDesignStorage.status().draftDirty), false);
    await clickNative(page, "LayoutDone");
    await exactHistory(beforeRow, afterRow, rowDepth, "Empty-grid row-track lifecycle draft");
    check("Empty grid frames expose native track authoring before children exist, and pagehide commits a focused row-track draft exactly once through the real inspector lifecycle");
}
