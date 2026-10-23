import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Fixture import supplies only a checked unfilled network. All authoring and
// history operations below go through real FerroUI controls and pointer input.
export async function networkBucketRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickNative, moveWorld, frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const storage = () => page.evaluate(() => globalThis.ferroDesignStorage.status());
    const durable = () => page.evaluate(async () =>
        JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text));
    const flushDurable = () => page.evaluate(async () => {
        let timeout;
        try {
            const saved = await Promise.race([
                globalThis.ferroDesignStorage.flush(),
                new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("The bucket autosave did not finish within 30 seconds.")), 30000); }),
            ]);
            return { saved, status: globalThis.ferroDesignStorage.status(),
                document: JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text) };
        } finally { clearTimeout(timeout); }
    });
    const blue = { paint: { type: "solid", r: 37, g: 99, b: 235, a: 255 }, opacity: 1, visible: true };
    const graph = {
        vertices: Object.fromEntries([[0, 0], [80, 0], [160, 0], [240, 0], [0, 120], [80, 120], [160, 120], [240, 120]]
            .map(([x, y], index) => [index + 1, { position: { x, y }, mode: "corner" }])),
        edges: Object.fromEntries([[1, 2], [2, 3], [3, 4], [5, 6], [6, 7], [7, 8], [1, 5], [2, 6], [3, 7], [4, 8]]
            .map(([start, end], index) => [index + 9, { start, end, curve: { type: "line" }, stroke: true }])),
        regions: {}, fill_rule: "even_odd", next_id: 19,
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "vector", name: "Three bucket faces", x: 40, y: 40, width: 240, height: 120,
        corner_radius: 0, corner_radii: null, fills: [blue], strokes: [],
        path: { segments: [], fill_rule: "even_odd" }, vector_network: graph,
    })], "Native bucket gestures"));
    await clickNative(page, "SelectTool");
    await page.keyboard.press("Escape");
    const beforeSelection = await state(page);
    await page.keyboard.press("Tab");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.selection.join() === "2" && view.chrome.CanvasHost.focusWithin;
    });
    // Keyboard selection rebuilds the inspector just as pointer selection
    // does. Its live bounds can precede the published native hit-test scene.
    await waitForNativeFrame(page, beforeSelection.renderedFrames, "Bucket fixture keyboard selection");
    await waitForNativeScene(page, beforeSelection.sceneFrames, "Bucket fixture keyboard selection");
    const beforeVector = await state(page);
    await clickNative(page, "EditVectorButton", "InspectorViewport");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.vectorNode === 2 && view.chrome.CanvasHost.focusWithin &&
            view.chrome.DoneVectorButton.visible && view.chrome.NetworkBucketButton.visible;
    });
    await waitForNativeFrame(page, beforeVector.renderedFrames, "Bucket fixture vector entry");
    await waitForNativeScene(page, beforeVector.sceneFrames, "Bucket fixture vector entry");
    await page.keyboard.press("Shift+B");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).vectorTool === "Bucket");

    const before = await documentState(page);
    const entry = await state(page);
    await moveWorld(page, 80, 100);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.face !== null);
    await waitForNativeFrame(page, entry.renderedFrames, "Bucket hover");
    const hover = await state(page);
    assert.equal(hover.networkBucket.active, false);
    assert.equal(hover.documentRevision, entry.documentRevision);
    assert.equal(hover.historyLength, entry.historyLength);
    assert.deepEqual(await documentState(page), before);
    await captureNativeScreenshot(mode + "-bucket-hover-face", { controls: ["CanvasHost"] });
    check("Native bucket hover displays a retained FerroUI face adorner while preserving the complete document, allocator, revision and undo history");

    const drag = async (clear, cancel = false) => {
        await moveWorld(page, 80, 100);
        if (clear) await page.keyboard.down("Shift");
        try {
            await page.mouse.down();
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
            // One move deliberately crosses the middle face between events.
            await moveWorld(page, 240, 100);
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.visited === 3);
            if (cancel) await page.keyboard.press("Escape");
            await page.mouse.up();
        } finally {
            if (clear) await page.keyboard.up("Shift");
        }
        await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
        await frames(page);
    };
    await drag(false);
    const painted = await documentState(page);
    assert.equal((await state(page)).historyLength, entry.historyLength + 1);
    const regions = painted.nodes[2].vector_network.regions;
    assert.equal(Object.keys(regions).length, 3);
    for (const region of Object.values(regions)) assert.deepEqual(region.fills, [blue]);
    assert.deepEqual(painted.nodes[2].vector_network.vertices, graph.vertices);
    assert.deepEqual(painted.nodes[2].vector_network.edges, graph.edges);
    assert.equal(painted.nodes[2].vector_network.next_id, 22);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), before);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), painted);
    await drag(false);
    assert.deepEqual(await documentState(page), painted);
    assert.equal((await state(page)).historyLength, entry.historyLength + 1);
    check("A captured native drag paints all three crossed faces, including the face between pointer events, once per gesture; unchanged drags preserve history and exact undo/redo retains every stable graph ID");

    await drag(true, true);
    assert.deepEqual(await documentState(page), painted);
    assert.equal((await state(page)).historyLength, entry.historyLength + 1);
    assert.equal((await state(page)).vectorNode, 2, "Escape cancels painting before leaving vector editing");
    await drag(true);
    const cleared = await documentState(page);
    assert.equal((await state(page)).historyLength, entry.historyLength + 2);
    for (const region of Object.values(cleared.nodes[2].vector_network.regions)) assert.deepEqual(region.fills, []);
    assert.deepEqual(Object.keys(cleared.nodes[2].vector_network.regions), Object.keys(regions));
    assert.equal(cleared.nodes[2].vector_network.next_id, painted.nodes[2].vector_network.next_id);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), painted);
    check("Shift-drag clears region coverage without deleting identities; Escape rolls the entire stroke back and releasing after cancellation cannot create a later history entry");

    // Start completely saved: otherwise a previous edit's pending serialization
    // could accidentally open the lifecycle gate for an untracked live stroke.
    const baselineSave = await flushDurable();
    assert.equal(baselineSave.saved, true);
    assert.equal(baselineSave.status.dirty, false);
    assert.deepEqual(baselineSave.document, painted);
    const lifecycleDepth = (await state(page)).historyLength;
    await moveWorld(page, 80, 100);
    await page.keyboard.down("Shift");
    let pointerHeld = false;
    try {
        await page.mouse.down();
        pointerHeld = true;
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
        await moveWorld(page, 240, 100);
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.visited === 3);
        assert.deepEqual(await documentState(page), cleared, "The held native stroke previews all crossed faces");
        assert.equal((await state(page)).historyLength, lifecycleDepth, "A live stroke has not committed its history entry");
        assert.equal((await storage()).draftDirty, true, "A held bucket stroke must open the host lifecycle commit gate");
        await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
        await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
        assert.deepEqual(await documentState(page), cleared);
        assert.equal((await state(page)).historyLength, lifecycleDepth + 1, "Pagehide commits the entire stroke exactly once");
        const lifecycleSave = await flushDurable();
        assert.equal(lifecycleSave.saved, true);
        assert.equal(lifecycleSave.status.dirty, false);
        assert.equal(lifecycleSave.status.draftDirty, false);
        assert.deepEqual(lifecycleSave.document, cleared, "IndexedDB contains every field of the lifecycle-committed stroke");
        await page.mouse.up();
        pointerHeld = false;
    } finally {
        if (pointerHeld) await page.mouse.up();
        await page.keyboard.up("Shift");
    }
    await frames(page);
    assert.deepEqual(await documentState(page), cleared);
    assert.equal((await state(page)).historyLength, lifecycleDepth + 1, "A late native release cannot duplicate the lifecycle commit");
    assert.equal((await storage()).draftDirty, false);
    const beforePendingUndo = await state(page);
    assert.equal(beforePendingUndo.serializationPending, false);
    assert.equal((await storage()).dirty, false);
    // Undo schedules the prior document edit. Press immediately on the same
    // face, before the 220 ms debounce can run; keep modifier release after
    // pointer capture so setup needs just one browser input round trip.
    await page.keyboard.down("Control");
    try {
        await page.keyboard.down("z");
        await page.mouse.down();
        pointerHeld = true;
        await page.keyboard.up("z");
        await page.keyboard.up("Control");
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
        const pending = await state(page);
        assert.equal(pending.serializationPending, true, "Undo must still have a pending snapshot when the unchanged stroke captures input");
        assert.equal(pending.saveSerializations, beforePendingUndo.saveSerializations,
            "The regression must begin before the prior edit's debounce serialization");
        assert.equal(pending.networkBucket.visited, 1);
        assert.equal(pending.historyLength, lifecycleDepth);
        assert.deepEqual(await documentState(page), painted, "The held stroke copies already-identical paints without editing the document");
        // This is a timer regression: allow more than two full debounce periods
        // and native frame turns while the pointer remains captured.
        await page.waitForTimeout(500);
        await frames(page);
        const deferred = await state(page);
        assert.equal(deferred.networkBucket.active, true);
        assert.equal(deferred.serializationPending, true);
        assert.equal(deferred.saveSerializations, beforePendingUndo.saveSerializations,
            "A background timer cannot serialize the document during a held bucket preview");
        assert.equal(deferred.historyLength, lifecycleDepth);
        assert.deepEqual(await documentState(page), painted);
        assert.deepEqual(await durable(), cleared, "The last durable snapshot remains unchanged while serialization is deferred");
        await page.mouse.up();
        pointerHeld = false;
    } finally {
        if (pointerHeld) await page.mouse.up();
        await page.keyboard.up("z");
        await page.keyboard.up("Control");
    }
    // Deliberately no host flush or commit-edit: release must restart the
    // background timer even though this gesture emitted no document change.
    await page.waitForFunction(() => {
        const native = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const stored = globalThis.ferroDesignStorage.status();
        return !native.networkBucket.active && !native.serializationPending && !stored.dirty && stored.phase === "saved";
    }, undefined, { timeout: 10000 });
    const resumed = await state(page);
    assert.equal(resumed.saveSerializations, beforePendingUndo.saveSerializations + 1,
        "Unchanged release resumes exactly one previously deferred serialization");
    assert.deepEqual(await documentState(page), painted, "One undo restores every field from before the lifecycle stroke");
    assert.equal(resumed.historyLength, lifecycleDepth);
    assert.deepEqual(await durable(), painted, "Ordinary autosave reaches IndexedDB without a later edit or explicit flush");
    check("A held native bucket stroke marks its draft from a clean saved baseline; real pagehide commits and durably saves it once, and a late pointer release preserves exact undo history");
    check("An unchanged captured bucket stroke defers an earlier edit's autosave beyond its debounce deadline, then resumes exactly one durable background save on release without adding history or requiring an explicit flush");

    await moveWorld(page, 80, 100);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.face !== null);
    await clickNative(page, "DoneVectorButton", "InspectorViewport");
    const [download] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await download.failure(), null);
    const file = resolve(output, mode + "-bucket-artwork-export.png");
    await download.saveAs(file);
    const png = PNG.sync.read(await readFile(file));
    assert.deepEqual([png.width, png.height], [240, 120]);
    for (const x of [40, 120, 200]) {
        const offset = (60 * png.width + x) * 4;
        assert.deepEqual([...png.data.subarray(offset, offset + 4)], [37, 99, 235, 255]);
    }
    assert.deepEqual(await documentState(page), painted);
    check("The downloaded FerroUI PNG includes the painted faces with exact color and dimensions while excluding transient bucket hover adorners");

    const crossingGraph = {
        vertices: Object.fromEntries([[0, 0], [100, 0], [100, 100], [0, 100]]
            .map(([x, y], index) => [index + 1, { position: { x, y }, mode: "corner" }])),
        edges: {
            5: { start: 1, end: 2, curve: { type: "line" }, stroke: true },
            6: { start: 2, end: 3, curve: { type: "line" }, stroke: true },
            7: { start: 3, end: 4, curve: { type: "line" }, stroke: true },
            8: { start: 4, end: 1, curve: { type: "line" }, stroke: true },
            9: { start: 1, end: 3, curve: { type: "cubic", control1: { x: 20, y: 60 }, control2: { x: 80, y: 40 } }, stroke: true },
            10: { start: 2, end: 4, curve: { type: "cubic", control1: { x: 80, y: 60 }, control2: { x: 20, y: 40 } }, stroke: true },
        },
        regions: {}, fill_rule: "even_odd", next_id: 11,
    };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "vector", name: "Crossing cubic bucket faces", x: 40, y: 40, width: 100, height: 100,
        corner_radius: 0, corner_radii: null, fills: [blue], strokes: [],
        path: { segments: [], fill_rule: "even_odd" }, vector_network: crossingGraph,
    })], "Checked cubic bucket preparation"));
    await clickNative(page, "SelectTool");
    await page.keyboard.press("Escape");
    const crossingSelection = await state(page);
    await page.keyboard.press("Tab");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.selection.join() === "2" && view.chrome.CanvasHost.focusWithin;
    });
    await waitForNativeFrame(page, crossingSelection.renderedFrames, "Crossing bucket keyboard selection");
    await waitForNativeScene(page, crossingSelection.sceneFrames, "Crossing bucket keyboard selection");
    const crossingVector = await state(page);
    await clickNative(page, "EditVectorButton", "InspectorViewport");
    await page.waitForFunction(() => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.vectorNode === 2 && view.chrome.CanvasHost.focusWithin && view.chrome.NetworkBucketButton.visible;
    });
    await waitForNativeFrame(page, crossingVector.renderedFrames, "Crossing bucket vector entry");
    await waitForNativeScene(page, crossingVector.sceneFrames, "Crossing bucket vector entry");
    await page.keyboard.press("Shift+B");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).vectorTool === "Bucket");
    const unsplit = await documentState(page), crossingEntry = await state(page);
    await moveWorld(page, 90, 50);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.face !== null);
    assert.deepEqual(await documentState(page), unsplit, "Prepared cubic hover must not split the authored network or reserve IDs");
    assert.equal((await state(page)).documentRevision, crossingEntry.documentRevision);
    assert.equal((await state(page)).historyLength, crossingEntry.historyLength);
    const crossingDrag = async (cancel) => {
        await moveWorld(page, 90, 50);
        await page.mouse.down();
        try {
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
            await moveWorld(page, 90, 130);
            await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.visited === 2);
            const preview = (await documentState(page)).nodes[2].vector_network;
            assert.equal(Object.keys(preview.vertices).length, 5);
            assert.equal(Object.keys(preview.edges).length, 8);
            assert.equal(Object.keys(preview.regions).length, 2);
            assert.equal((await state(page)).historyLength, crossingEntry.historyLength);
            if (cancel) await page.keyboard.press("Escape");
        } finally { await page.mouse.up(); }
        await page.waitForFunction(() => !JSON.parse(globalThis.ferroDesign.ferroDesignState()).networkBucket.active);
        await frames(page);
    };
    await crossingDrag(true);
    assert.deepEqual(await documentState(page), unsplit, "Escape rolls back cubic subdivision, paint, provisional IDs and allocator together");
    assert.equal((await state(page)).historyLength, crossingEntry.historyLength);
    await crossingDrag(false);
    const splitPaint = await documentState(page), splitGraph = splitPaint.nodes[2].vector_network;
    assert.equal((await state(page)).historyLength, crossingEntry.historyLength + 1);
    assert.equal(splitGraph.next_id, 16);
    for (const [id, vertex] of Object.entries(crossingGraph.vertices)) assert.deepEqual(splitGraph.vertices[id], vertex);
    for (const id of [5, 6, 7, 8]) assert.deepEqual(splitGraph.edges[id], crossingGraph.edges[id]);
    assert.equal(splitGraph.edges[9].curve.type, "cubic");
    assert.equal(splitGraph.edges[10].curve.type, "cubic");
    for (const region of Object.values(splitGraph.regions)) assert.deepEqual(region.fills, [blue]);
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), unsplit);
    await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), splitPaint);
    check("Native cubic bucket hover prepares intersections without mutation; a captured two-face stroke applies subdivision once, Escape restores unsplit IDs and allocator, and exact undo/redo treats geometry plus paint as one edit");

    await clickNative(page, "DoneVectorButton", "InspectorViewport");
    const [crossingDownload] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport"),
    ]);
    assert.equal(await crossingDownload.failure(), null);
    const crossingFile = resolve(output, mode + "-bucket-split-cubics-export.png");
    await crossingDownload.saveAs(crossingFile);
    const crossingPng = PNG.sync.read(await readFile(crossingFile));
    assert.deepEqual([crossingPng.width, crossingPng.height], [100, 100]);
    for (const [x, y] of [[50, 10], [50, 90]]) {
        const offset = (y * crossingPng.width + x) * 4;
        assert.deepEqual([...crossingPng.data.subarray(offset, offset + 4)], [37, 99, 235, 255]);
    }
    for (const [x, y] of [[10, 50], [90, 50]]) {
        assert.equal(crossingPng.data[(y * crossingPng.width + x) * 4 + 3], 0,
            "Unpainted faces stay transparent after cubic intersection preparation");
    }
    assert.deepEqual(await documentState(page), splitPaint);
    check("The native PNG preserves exact cubic face fills and transparent unpainted neighbors after automatic subdivision, without exporting the hover preview");
}
