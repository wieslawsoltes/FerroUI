import assert from "node:assert/strict";
import { PNG } from "pngjs";

// Pointer coordinates and controls are supplied by the actual FerroUI tree.
export async function vectorNetworkRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative,
        frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const command = async (name) => {
        assert.equal(await page.evaluate((name) => globalThis.ferroDesign.ferroDesignCommand(name), name), true);
        await frames(page);
    };
    const selectNetworkRoot = async () => {
        await page.keyboard.press("Escape");
        const before = await state(page);
        await page.keyboard.press("Tab");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.selection.join() === "2" && view.chrome.CanvasHost.focusWithin;
        }, null, { timeout: 10000 });
        // Keyboard selection changes the inspector before its hit-test scene
        // is necessarily published, just as pointer selection does.
        await waitForNativeFrame(page, before.renderedFrames, "Network fixture keyboard selection");
        await waitForNativeScene(page, before.sceneFrames, "Network fixture keyboard selection");
    };
    const enterNetwork = async () => {
        const before = await state(page);
        await command("edit-vector");
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
            return view.vectorNode === 2 && view.chrome.CanvasHost.focusWithin &&
                view.chrome.NetworkDetectFacesButton.visible;
        }, null, { timeout: 10000 });
        await waitForNativeFrame(page, before.renderedFrames, "Network fixture vector entry");
        await waitForNativeScene(page, before.sceneFrames, "Network fixture vector entry");
    };
    const node = async () => (await documentState(page)).nodes[2];
    const screen = (view, x, y) => ({ x: view.chrome.CanvasHost.x + view.pan.x + x * view.zoom, y: view.chrome.CanvasHost.y + view.pan.y + y * view.zoom });
    const drag = async (from, to) => {
        const view = await state(page), a = screen(view, ...from), b = screen(view, ...to);
        await page.mouse.move(a.x, a.y);
        await page.mouse.down();
        await page.mouse.move(b.x, b.y, { steps: 16 });
        await page.mouse.up();
        await frames(page);
    };
    const fixture = fixtureNode(reference, 2, {
        kind: "vector", name: "Branching vector network", x: 40, y: 40, width: 200, height: 160,
        corner_radius: 0,
        fills: [{ paint: { type: "solid", r: 239, g: 68, b: 68, a: 255 }, opacity: 1, visible: true }],
        strokes: [{ paint: { type: "solid", r: 24, g: 24, b: 24, a: 255 }, width: 3, opacity: 1, visible: true,
            align: "center", cap: "round", join: "round", dash: [] }],
        path: { fill_rule: "non_zero", segments: [
            { type: "move_to", point: { x: 0, y: 0 } },
            { type: "line_to", point: { x: 200, y: 0 } },
            { type: "line_to", point: { x: 100, y: 160 } },
            { type: "close" },
        ] },
    });
    await importFixture(page, isolatedDocument(reference, [fixture], "Vector graph authoring"));
    // Leave room for a branch outside the original fitted bounds.
    for (let i = 0; i < 3; i++) await command("zoom-out");
    await clickWorld(page, 140, 100);
    await command("vector-network");
    const converted = await node();
    assert.equal((await state(page)).vectorNode, 2);
    assert.equal(converted.path.segments.length, 0);
    assert.equal(Object.keys(converted.vector_network.vertices).length, 3);
    assert.equal(Object.keys(converted.vector_network.edges).length, 3);
    assert.equal(Object.keys(converted.vector_network.regions).length, 1);
    const image = PNG.sync.read(await captureNativeScreenshot(mode + "-vector-network-fill", {
        controls: ["CanvasHost", "DoneVectorButton"],
    })), p = screen(await state(page), 140, 100);
    let red = 0;
    for (let y = Math.round(p.y) - 3; y <= Math.round(p.y) + 3; y++) {
        for (let x = Math.round(p.x) - 3; x <= Math.round(p.x) + 3; x++) {
            const offset = (y * image.width + x) * 4;
            if (image.data[offset] > 200 && image.data[offset + 1] < 100 && image.data[offset + 2] < 100) red++;
        }
    }
    assert.ok(red >= 35, "Converted network fill must be drawn by the real renderer");
    await command("undo");
    assert.equal((await node()).vector_network, undefined);
    await command("redo");
    assert.deepEqual(await node(), converted);
    // Undo exits editing when the legacy path returns; explicitly re-enter.
    await command("edit-vector");
    await clickNative(page, "NetworkPenButton", "InspectorViewport");
    await clickWorld(page, 40, 40);
    await clickWorld(page, 0, 160);
    await frames(page);
    const branched = await node();
    const originalVertexIds = new Set(Object.keys(converted.vector_network.vertices));
    const branch = Object.keys(branched.vector_network.vertices).find((id) => !originalVertexIds.has(id));
    assert.ok(branch);
    assert.equal(Object.keys(branched.vector_network.edges).length, 4);
    assert.deepEqual(branched.vector_network.regions, converted.vector_network.regions, "Adding a branch must preserve the explicit fill boundary");
    const branchEdge = Object.entries(branched.vector_network.edges).find(([, edge]) => String(edge.start) === branch || String(edge.end) === branch)?.[0];
    assert.ok(branchEdge);
    await clickNative(page, "VectorSelectButton", "InspectorViewport");
    await drag([0, 160], [-40, 200]);
    const moved = await node(), vertex = moved.vector_network.vertices[branch].position;
    assert.ok(Math.abs(moved.x + vertex.x + 40) < 0.7 && Math.abs(moved.y + vertex.y - 200) < 0.7);
    assert.deepEqual(Object.keys(moved.vector_network.vertices), Object.keys(branched.vector_network.vertices));
    assert.deepEqual(Object.keys(moved.vector_network.edges), Object.keys(branched.vector_network.edges));
    await command("undo");
    assert.deepEqual(await node(), branched, "A multi-event network drag must undo as one exact document edit");
    await command("redo");
    assert.deepEqual(await node(), moved);
    await clickWorld(page, 0, 120, { clickCount: 2 });
    await frames(page);
    const split = await node();
    assert.equal(Object.keys(split.vector_network.vertices).length, 5);
    assert.equal(Object.keys(split.vector_network.edges).length, 5);
    assert.deepEqual(split.vector_network.regions, moved.vector_network.regions);
    await command("undo");
    assert.deepEqual(await node(), moved);
    check("Native graph conversion, branching, stable-ID vertex dragging and edge subdivision preserve regions and exact one-step history");

    await clickNative(page, "VectorBendButton", "InspectorViewport");
    await drag([0, 120], [24, 132]);
    const bent = await node(), graph = bent.vector_network, edge = graph.edges[branchEdge];
    assert.equal(edge.curve.type, "cubic");
    const a = graph.vertices[edge.start].position, b = graph.vertices[edge.end].position;
    const c = edge.curve.control1, d = edge.curve.control2;
    const midpoint = { x: (a.x + 3 * c.x + 3 * d.x + b.x) / 8 + bent.x, y: (a.y + 3 * c.y + 3 * d.y + b.y) / 8 + bent.y };
    assert.ok(Math.abs(midpoint.x - 24) < 1 && Math.abs(midpoint.y - 132) < 1, "The curve must pass through the native bend target");
    await clickNative(page, "VectorEraseButton", "InspectorViewport");
    await clickWorld(page, midpoint.x, midpoint.y);
    await frames(page);
    const erased = await node();
    assert.equal(erased.vector_network.edges[branchEdge], undefined);
    assert.deepEqual(erased.vector_network.regions, bent.vector_network.regions);
    await command("undo");
    assert.deepEqual(await node(), bent);
    await command("redo");
    assert.deepEqual(await node(), erased);
    await captureNativeScreenshot(mode + "-vector-network", {
        controls: ["CanvasHost", "VectorEraseButton"],
    });
    const json = await documentState(page);
    await importFixture(page, json);
    assert.deepEqual(await documentState(page), json, "Stable graph IDs and boundaries must survive a real browser save/import round-trip");
    check("Native network Bend and Erase edit authoritative cubic edges, retain unrelated fill regions and serialize stable topology");

    const graphFixture = {
        vertices: {
            1: { position: { x: 0, y: 0 }, mode: "corner" },
            2: { position: { x: 200, y: 0 }, mode: "corner" },
            3: { position: { x: 200, y: 160 }, mode: "corner" },
            4: { position: { x: 0, y: 160 }, mode: "corner" },
            5: { position: { x: 260, y: 160 }, mode: "corner" },
        },
        edges: {
            6: { start: 1, end: 2, curve: { type: "line" }, stroke: true },
            7: { start: 2, end: 3, curve: { type: "line" }, stroke: true },
            8: { start: 3, end: 4, curve: { type: "line" }, stroke: true },
            9: { start: 4, end: 1, curve: { type: "line" }, stroke: true },
            10: { start: 1, end: 3, curve: { type: "line" }, stroke: true },
            11: { start: 3, end: 5, curve: { type: "line" }, stroke: true },
        },
        regions: {}, fill_rule: "even_odd", next_id: 12,
    };
    const faceNode = fixtureNode(reference, 2, {
        ...fixture, name: "Automatic closed faces", width: 260,
        path: { segments: [], fill_rule: "even_odd" }, vector_network: graphFixture,
    });
    await importFixture(page, isolatedDocument(reference, [faceNode], "Automatic vector faces"));
    await selectNetworkRoot();
    await enterNetwork();
    const unfilled = await documentState(page), faceDepth = (await state(page)).historyLength;
    await clickNative(page, "NetworkDetectFacesButton", "InspectorViewport");
    await frames(page);
    const filled = await documentState(page), filledGraph = filled.nodes[2].vector_network;
    assert.equal(Object.keys(filledGraph.regions).length, 2);
    assert.equal(filledGraph.next_id, 14);
    assert.deepEqual(filledGraph.vertices, graphFixture.vertices);
    assert.deepEqual(filledGraph.edges, graphFixture.edges);
    assert.equal((await state(page)).historyLength, faceDepth + 1);
    assert.ok(Object.values(filledGraph.regions).every((region) =>
        region.contours.flat().every((reference) => reference.edge !== 11)),
    "Open branches must stay stroked without entering a face boundary");
    const faceImage = PNG.sync.read(await captureNativeScreenshot(mode + "-automatic-vector-faces", {
        controls: ["CanvasHost", "NetworkDetectFacesButton"],
    }));
    const faceView = await state(page);
    for (const [x, y] of [[90, 150], [190, 90]]) {
        const pixel = screen(faceView, x, y), offset = (Math.round(pixel.y) * faceImage.width + Math.round(pixel.x)) * 4;
        assert.ok(faceImage.data[offset] > 200 && faceImage.data[offset + 1] < 100 && faceImage.data[offset + 2] < 100,
            "Both automatically discovered faces must paint with the native layer fill");
    }
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("network-fill-faces")), false);
    assert.deepEqual(await documentState(page), filled, "Repeated discovery is a serialized no-op");
    assert.equal((await state(page)).historyLength, faceDepth + 1);
    await command("undo");
    assert.deepEqual(await documentState(page), unfilled);
    await command("redo");
    assert.deepEqual(await documentState(page), filled);
    check("Native Fill all closed regions discovers planar faces, excludes open branches, paints every face, preserves topology IDs and commits one exact idempotent undo/redo edit");

    const crossing = structuredClone(faceNode);
    crossing.vector_network.edges[12] = { start: 2, end: 4, curve: { type: "line" }, stroke: true };
    crossing.vector_network.next_id = 13;
    await importFixture(page, isolatedDocument(reference, [crossing], "Automatic network intersection splitting"));
    await selectNetworkRoot();
    await enterNetwork();
    const beforeCrossing = await documentState(page), crossingDepth = (await state(page)).historyLength;
    await clickNative(page, "NetworkDetectFacesButton", "InspectorViewport");
    await page.waitForFunction(() => {
        const graph = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].vector_network;
        return Object.keys(graph.regions).length === 4;
    }, null, { timeout: 10000 });
    const splitCrossing = await documentState(page), splitGraph = splitCrossing.nodes[2].vector_network;
    assert.equal(Object.keys(splitGraph.vertices).length, 6);
    assert.equal(Object.keys(splitGraph.edges).length, 9);
    assert.equal(splitGraph.next_id, 20);
    for (const [id, vertex] of Object.entries(graphFixture.vertices)) assert.deepEqual(splitGraph.vertices[id], vertex);
    for (const id of [6, 7, 8, 9, 11]) assert.deepEqual(splitGraph.edges[id], graphFixture.edges[id]);
    assert.deepEqual(splitGraph.vertices[13].position, { x: 100, y: 80 });
    assert.equal(splitGraph.edges[10].start, 1);
    assert.equal(splitGraph.edges[12].start, 2);
    assert.equal(splitGraph.edges[10].end, 13);
    assert.equal(splitGraph.edges[12].end, 13);
    assert.equal((await state(page)).historyLength, crossingDepth + 1);
    const crossingImage = PNG.sync.read(await captureNativeScreenshot(mode + "-split-vector-intersections", {
        controls: ["CanvasHost", "NetworkDetectFacesButton"],
    }));
    const crossingView = await state(page);
    for (const [x, y] of [[140, 60], [220, 120], [140, 180], [60, 120]]) {
        const pixel = screen(crossingView, x, y), offset = (Math.round(pixel.y) * crossingImage.width + Math.round(pixel.x)) * 4;
        assert.ok(crossingImage.data[offset] > 200 && crossingImage.data[offset + 1] < 100 && crossingImage.data[offset + 2] < 100,
            "Every newly split bounded face must paint through the native renderer");
    }
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("network-fill-faces")), false);
    assert.deepEqual(await documentState(page), splitCrossing);
    assert.equal((await state(page)).historyLength, crossingDepth + 1);
    await command("undo");
    assert.deepEqual(await documentState(page), beforeCrossing, "One undo restores unsplit geometry and its exact allocator");
    await command("redo");
    assert.deepEqual(await documentState(page), splitCrossing);
    check("Native Fill all closed regions splits transverse intersections, preserves original first-edge and branch identities, paints four new faces, and keeps subdivision plus fill in one idempotent undo/redo edit");

    const overlapping = structuredClone(faceNode);
    overlapping.vector_network.edges[12] = { start: 1, end: 3, curve: { type: "line" }, stroke: true };
    overlapping.vector_network.next_id = 13;
    await importFixture(page, isolatedDocument(reference, [overlapping], "Ambiguous overlapping network edges"));
    await selectNetworkRoot();
    await page.keyboard.press("ArrowRight"); await frames(page);
    const redoDocument = await documentState(page);
    await command("undo");
    const beforeError = await documentState(page), beforeErrorView = await state(page);
    assert.equal(beforeErrorView.canRedo, true);
    await enterNetwork();
    const rejection = "Edges 10 and 12 cross, overlap, touch without a shared vertex, or cannot be separated reliably. Split and weld their intersections before finding faces.";
    const beforeClick = await state(page);
    assert.equal(beforeClick.chrome.Toast?.visible === true &&
        beforeClick.chrome.ToastText?.text === rejection, false,
    "The expected intersection rejection must not already be visible before face discovery");
    await clickNative(page, "NetworkDetectFacesButton", "InspectorViewport");
    await page.waitForFunction((rejection) => {
        const { chrome } = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return chrome.Toast?.visible === true && chrome.ToastText?.text === rejection;
    }, rejection, { timeout: 10000 });
    assert.deepEqual(await documentState(page), beforeError, "Ambiguous overlapping edges must fail before changing graph or allocator");
    assert.equal((await state(page)).historyLength, beforeErrorView.historyLength);
    assert.equal((await state(page)).canRedo, true);
    await command("redo");
    assert.deepEqual(await documentState(page), redoDocument);
    check("Native automatic intersection preparation rejects ambiguous overlapping edges with a fresh exact error, preserves the entire graph and allocator, and retains the preceding redo transaction");
}
