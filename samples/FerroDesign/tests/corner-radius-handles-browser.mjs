import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { startInputObservation, stopInputObservation } from "./input-observations.mjs";

// Only initial isolated fixtures enter through document import. Every edit uses
// native pointer input; cornerRadius supplies read-only handle geometry.
export async function cornerRadiusHandleRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickNative, clickWorld, moveWorld, frames, waitForNativeFrame, waitForNativeScene,
        captureNativeScreenshot } = helpers;
    const near = (actual, expected, label) => assert.ok(Math.abs(actual - expected) < 1e-6,
        `${label}: ${actual} != ${expected}`);
    const wait = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("view", "document", "args", "return (" + source + ")(view,document,args)")(view, document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const published = async (before, label) => {
        await waitForNativeFrame(page, before.renderedFrames, label);
        await waitForNativeScene(page, before.sceneFrames, label);
    };
    const storage = () => page.evaluate(() => globalThis.ferroDesignStorage.status());
    const durable = () => page.evaluate(async () => JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text));
    const world = (node, x, y) => {
        const matrix = node.affine_transform || { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        const dx = (matrix.a * x + matrix.c * y + matrix.e - node.width / 2) * (node.flip_x ? -1 : 1);
        const dy = (matrix.b * x + matrix.d * y + matrix.f - node.height / 2) * (node.flip_y ? -1 : 1);
        const angle = node.rotation * Math.PI / 180;
        return { x: node.x + node.width / 2 + dx * Math.cos(angle) - dy * Math.sin(angle),
            y: node.y + node.height / 2 + dx * Math.sin(angle) + dy * Math.cos(angle) };
    };
    const load = async fixture => {
        const previous = await state(page);
        await importFixture(page, fixture);
        await waitForNativeScene(page, previous.sceneFrames, "Corner fixture import");
        await clickNative(page, "SelectTool");
        const selected = await state(page);
        if (selected.selection.length) {
            await page.keyboard.press("Escape");
            await wait(view => view.selection.length === 0);
            await published(selected, "Corner fixture selection clear");
        }
        const node = fixture.nodes[2], center = world(node, node.width / 2, node.height / 2);
        await clickWorld(page, center.x, center.y);
        await wait(view => view.selection.join() === "2" && view.cornerRadius.node === 2 &&
            view.cornerRadius.handles.length > 0 && view.chrome.CanvasHost.focusWithin);
    };
    const handle = async corner => {
        const view = await state(page);
        assert.equal(view.cornerRadius.node, 2);
        const result = view.cornerRadius.handles.find(item => item.corner === corner);
        assert.ok(result, "The selected primitive exposes the requested native radius handle");
        for (const point of [result.world, result.worldPerRadius]) {
            assert.ok(Number.isFinite(point.x) && Number.isFinite(point.y));
        }
        assert.ok(Math.hypot(result.worldPerRadius.x, result.worldPerRadius.y) > 0);
        return result;
    };
    const destination = (item, delta) => ({ x: item.world.x + item.worldPerRadius.x * delta,
        y: item.world.y + item.worldPerRadius.y * delta });
    const assertRadiusMutation = (before, after, expectedRadius, corner = null) => {
        const expected = structuredClone(before), node = expected.nodes[2], actual = after.nodes[2];
        if (corner === null) {
            near(actual.corner_radius, expectedRadius, "Uniform fractional radius");
            node.corner_radius = actual.corner_radius;
            node.corner_radii = null;
        } else {
            near(actual.corner_radii[corner], expectedRadius, "Independent fractional radius");
            node.corner_radii = node.corner_radii ? [...node.corner_radii] : Array(4).fill(node.corner_radius);
            node.corner_radii[corner] = actual.corner_radii[corner];
        }
        delete node.variable_bindings.corner_radius;
        node.overridden_properties = [...new Set([...node.overridden_properties, "corner_radius"])].sort();
        assert.deepEqual(after, expected,
            "Radius input preserves every other document field, including primitive, transforms, paints, bindings and allocator");
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        let view = await state(page);
        await page.keyboard.press("Control+z");
        await published(view, label + " undo");
        assert.deepEqual(await documentState(page), before, label + " exact undo");
        view = await state(page);
        await page.keyboard.press("Control+Shift+z");
        await published(view, label + " redo");
        assert.deepEqual(await documentState(page), after, label + " exact redo");
        assert.deepEqual((await state(page)).selection, [2]);
    };
    const press = async (corner, individual = false) => {
        const item = await handle(corner);
        await moveWorld(page, item.world.x, item.world.y);
        await wait((view, _document, corner) => view.cornerRadius.hover === corner, corner);
        const before = await documentState(page), view = await state(page);
        await page.mouse.down();
        await wait((view, _document, args) => view.cornerRadius.active &&
            view.cornerRadius.corner === args.corner && view.cornerRadius.individual === args.individual,
        { corner, individual });
        const pressed = await state(page);
        assert.deepEqual(await documentState(page), before, "Pressing a displaced radius marker cannot change its bound value");
        assert.equal(pressed.historyLength, view.historyLength);
        assert.equal(pressed.canRedo, view.canRedo);
        return item;
    };
    const move = async (item, delta, individual = false) => {
        const point = destination(item, delta), expected = item.authoredRadius + delta;
        await moveWorld(page, point.x, point.y);
        await wait((_view, document, args) => Math.abs((args.individual
            ? document.nodes[2].corner_radii?.[args.corner] : document.nodes[2].corner_radius) - args.expected) < 1e-6,
        { corner: item.corner, individual, expected });
        return expected;
    };
    const release = async label => {
        const before = await state(page);
        await page.mouse.up();
        await wait(view => !view.cornerRadius.active);
        await published(before, label);
    };
    const drag = async (corner, delta, individual = false) => {
        if (individual) await page.keyboard.down("Alt");
        let held = false;
        try {
            held = true;
            const item = await press(corner, individual);
            const expected = await move(item, delta, individual);
            await release("Native corner radius release");
            held = false;
            return expected;
        } finally {
            if (held) await page.mouse.up();
            if (individual) await page.keyboard.up("Alt");
        }
    };
    const blue = { paint: { type: "solid", r: 37, g: 99, b: 235, a: 255 }, opacity: 1, visible: true };
    const primitive = properties => fixtureNode(reference, 2, {
        kind: "rectangle", x: 80, y: 80, width: 220, height: 160, rotation: 11,
        affine_transform: { a: 1, b: 0.08, c: 0.1, d: 1, e: 0, f: 0 },
        fills: [blue], strokes: [], corner_radius: 8, corner_radii: null, corner_smoothing: 0.6,
        polygon_sides: 5, star_inner_radius: 0.45, ...properties,
    });
    const rectangle = isolatedDocument(reference, [primitive({ name: "Bound native corner handles",
        variable_bindings: { corner_radius: 900 } })], "Native corner radius gestures");
    rectangle.variable_mode = "Default"; rectangle.next_id = 901;
    rectangle.variables = { 900: { id: 900, name: "Radius", collection: "Shapes",
        values: { Default: { type: "number", value: 8 } } } };
    await load(rectangle);
    const original = await documentState(page), initial = await state(page);
    let item = await handle(0);
    await moveWorld(page, item.world.x, item.world.y);
    await wait(view => view.cornerRadius.hover === 0 && !view.cornerRadius.active);
    await published(initial, "Native radius hover");
    let view = await state(page);
    assert.equal(view.documentRevision, initial.documentRevision);
    assert.equal(view.historyLength, initial.historyLength);
    assert.equal(view.canRedo, initial.canRedo);
    assert.deepEqual(await documentState(page), original);
    await captureNativeScreenshot(mode + "-corner-radius-hover", { controls: ["CanvasHost"] });
    check("Native corner hover paints a public FerroUI handle while preserving every document field, bound radius, revision and history");

    let expectedRadius = await drag(0, 10.5);
    const uniform = await documentState(page);
    near(expectedRadius, 18.5, "Uniform drag target");
    assertRadiusMutation(original, uniform, 18.5);
    await history(original, uniform, initial.historyLength, "Uniform corner handle");
    check("A captured fractional corner drag preserves the editable rectangle, smoothing, affine transform and artwork; it detaches only the radius binding and commits one exactly reversible edit");

    // Keep a real redo entry throughout both cancellation cases; a boolean flag
    // alone would not prove that the exact authored document remains redoable.
    view = await state(page); await page.keyboard.press("Control+z");
    await published(view, "Prepare corner cancellation redo");
    assert.deepEqual(await documentState(page), original);
    const canceledBaseline = await state(page);
    assert.equal(canceledBaseline.canRedo, true);
    let held = false;
    try {
        held = true; item = await press(0);
        await move(item, 11.25);
        assert.equal((await state(page)).historyLength, canceledBaseline.historyLength);
        const beforeEscape = await state(page);
        await page.keyboard.press("Escape");
        await wait(view => !view.cornerRadius.active);
        await published(beforeEscape, "Cancel native corner drag");
        const late = destination(item, 14.5);
        await moveWorld(page, late.x, late.y);
        await page.mouse.up(); held = false;
    } finally { if (held) await page.mouse.up(); }
    await frames(page);
    assert.deepEqual(await documentState(page), original);
    assert.deepEqual((await state(page)).selection, [2], "Escape cancels the captured drag before clearing selection");
    assert.equal((await state(page)).historyLength, canceledBaseline.historyLength);
    assert.equal((await state(page)).canRedo, true);
    try {
        held = true; await press(0);
        await release("Unchanged radius press"); held = false;
    } finally { if (held) await page.mouse.up(); }
    assert.deepEqual(await documentState(page), original, "An unchanged click preserves the bound radius and metadata");
    assert.equal((await state(page)).historyLength, canceledBaseline.historyLength);
    assert.equal((await state(page)).canRedo, true);
    view = await state(page); await page.keyboard.press("Control+Shift+z");
    await published(view, "Redo after corner cancellation");
    assert.deepEqual(await documentState(page), uniform);
    check("Escape restores the complete bound document, selection and redo; late pointer events and an unchanged radius press cannot detach the binding or consume the previous edit");

    view = await state(page); await page.keyboard.press("Control+z");
    await published(view, "Prepare touch cancellation redo");
    assert.deepEqual(await documentState(page), original);
    item = await handle(0);
    const touchView = await state(page), end = destination(item, 6.5);
    const css = point => ({ x: touchView.chrome.CanvasHost.x + touchView.pan.x + point.x * touchView.zoom,
        y: touchView.chrome.CanvasHost.y + touchView.pan.y + point.y * touchView.zoom });
    const touchStart = css(item.world), touchEnd = css(end), viewport = page.viewportSize();
    for (const point of [touchStart, touchEnd]) {
        assert.ok(point.x > 0 && point.x < viewport.width && point.y > 0 && point.y < viewport.height,
            "CDP touch coordinates use CSS viewport pixels, without multiplying by device scale");
    }
    const client = await page.context().newCDPSession(page);
    let touchHeld = false, observing = false, observation;
    try {
        await startInputObservation(page, ["pointerdown", "pointermove", "pointercancel"]); observing = true;
        touchHeld = true;
        await client.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ ...touchStart, id: 31, force: 1 }] });
        await wait(view => view.cornerRadius.active && view.cornerRadius.corner === 0);
        assert.deepEqual(await documentState(page), original);
        await client.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ ...touchEnd, id: 31, force: 1 }] });
        await wait((_view, document) => Math.abs(document.nodes[2].corner_radius - 14.5) < 1e-6);
        const beforeCancel = await state(page);
        await client.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] }); touchHeld = false;
        await wait(view => !view.cornerRadius.active);
        await published(beforeCancel, "Native touch capture cancellation");
    } finally {
        try {
            if (touchHeld) await client.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
        } finally {
            try {
                if (observing) {
                    observation = await stopInputObservation(page);
                    await writeFile(resolve(output, mode + "-corner-radius-touch-input.json"), JSON.stringify(observation, null, 2));
                }
            } finally { await client.detach(); }
        }
    }
    for (const type of ["pointerdown", "pointermove", "pointercancel"]) {
        assert.ok(observation.events.some(event => event.type === type && event.phase === "capture" &&
            event.trusted && event.pointerType === "touch"), "The browser must dispatch a trusted native " + type);
    }
    assert.deepEqual(await documentState(page), original);
    view = await state(page);
    assert.equal(view.historyLength, canceledBaseline.historyLength);
    assert.equal(view.canRedo, true); assert.deepEqual(view.selection, [2]);
    await page.keyboard.press("Control+Shift+z");
    await published(view, "Redo after native touch cancellation");
    assert.deepEqual(await documentState(page), uniform);
    check("Real Chromium touch cancellation reaches the production pointer bridge and rolls back the captured radius edit, including its variable binding, while preserving exact redo and selection");

    const individualDepth = (await state(page)).historyLength;
    expectedRadius = await drag(1, 6.25, true);
    const individual = await documentState(page);
    near(expectedRadius, 24.75, "Alt corner target");
    assertRadiusMutation(uniform, individual, 24.75, 1);
    assert.equal(individual.nodes[2].corner_radius, uniform.nodes[2].corner_radius);
    await captureNativeScreenshot(mode + "-corner-radius-independent", { controls: ["CanvasHost"] });
    await history(uniform, individual, individualDepth, "Alt rectangle corner");
    check("Alt at pointer press changes only the chosen rectangle corner; its other three radii, uniform value, transform and smoothing remain exact through one-step undo/redo");

    // A clean baseline proves pagehide itself opens the native draft commit
    // gate; no explicit post-pagehide flush may conceal a missing host hook.
    const saved = await page.evaluate(async () => {
        let timeout;
        try {
            return await Promise.race([globalThis.ferroDesignStorage.flush(), new Promise((_, reject) => {
                timeout = setTimeout(() => reject(new Error("Corner baseline storage flush exceeded 30 seconds")), 30000);
            })]);
        } finally { clearTimeout(timeout); }
    });
    assert.equal(saved, true);
    const savedStatus = await storage(), savedNative = await state(page);
    assert.equal(savedStatus.dirty, false); assert.equal(savedStatus.draftDirty, false);
    assert.equal(savedNative.serializationPending, false);
    assert.deepEqual(await durable(), individual);
    let lifecycle;
    try {
        held = true; item = await press(0);
        await move(item, 10.75);
        lifecycle = await documentState(page);
        assertRadiusMutation(individual, lifecycle, 29.25);
        assert.equal((await state(page)).historyLength, savedNative.historyLength);
        assert.equal((await storage()).draftDirty, true);
        assert.equal((await state(page)).saveSerializations, savedNative.saveSerializations);
        assert.deepEqual(await durable(), individual, "A held preview cannot replace the last durable snapshot");
        await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState()), saved = globalThis.ferroDesignStorage.status();
            return !view.cornerRadius.active && !view.serializationPending && !saved.dirty && saved.phase === "saved";
        }, undefined, { timeout: 30000 });
        assert.deepEqual(await durable(), lifecycle);
        assert.equal((await state(page)).historyLength, savedNative.historyLength + 1);
        assert.equal((await state(page)).saveSerializations, savedNative.saveSerializations + 1);
        assert.equal((await storage()).revision, savedStatus.revision + 1);
        await page.mouse.up(); held = false;
    } finally { if (held) await page.mouse.up(); }
    await frames(page);
    assert.deepEqual(await documentState(page), lifecycle);
    assert.deepEqual(await durable(), lifecycle);
    assert.equal((await state(page)).historyLength, savedNative.historyLength + 1);
    assert.equal((await state(page)).saveSerializations, savedNative.saveSerializations + 1);
    assert.equal((await storage()).revision, savedStatus.revision + 1);
    assert.equal((await storage()).draftDirty, false);
    await history(individual, lifecycle, savedNative.historyLength, "Pagehide radius gesture");
    check("A held radius drag marks the clean host draft; real pagehide commits one history entry, serializes and saves one exact IndexedDB revision, and late release cannot duplicate either operation");

    for (const [kind, radius, delta] of [["polygon", 4, 8.75], ["star", 6.5, 5.25]]) {
        const shape = primitive({ kind, name: "Affine " + kind + " radius handle", x: 180, y: 100,
            width: 210, height: 180, rotation: 23, flip_x: true, corner_radius: radius, corner_smoothing: 0,
            affine_transform: { a: 1.1, b: 0.2, c: 0.25, d: 0.9, e: 0, f: 0 } });
        await load(isolatedDocument(reference, [shape], "Affine " + kind + " radius gesture"));
        const before = await documentState(page), depth = (await state(page)).historyLength;
        await drag(0, delta);
        const after = await documentState(page);
        assertRadiusMutation(before, after, radius + delta);
        assert.equal(after.nodes[2].kind, kind);
        assert.equal(after.nodes[2].corner_radii, null);
        await captureNativeScreenshot(mode + "-corner-radius-affine-" + kind, { controls: ["CanvasHost"] });
        await history(before, after, depth, "Affine " + kind + " radius");
    }
    assert.equal((await state(page)).invalidNodes, 0);
    assert.equal((await state(page)).unsupportedEffects, 0);
    check("Native polygon and star radius drags preserve editable point geometry, center rotation, reflection and shear; fractional local radii and complete documents survive exact single-step history");
}
