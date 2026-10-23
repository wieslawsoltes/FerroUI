import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";
import { readInputObservation, startInputObservation, stopInputObservation } from "./input-observations.mjs";
import { angularInsertionFromPointerInput, angularPaintFromPointerInput } from "./gradient-input.mjs";
import { captureWithinDeadline } from "./isolated-scenario.mjs";

// Fixtures are imported once. All subsequent edits use native controls and
// trusted browser input; gradientEdit supplies read-only handle coordinates.
export async function gradientHandleRegressions(page, reference, check, helpers, output, mode) {
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
    const entryInputs = [];
    const entryState = (view, name) => ({
        renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames,
        documentRevision: view.documentRevision, historyLength: view.historyLength,
        canUndo: view.canUndo, canRedo: view.canRedo, selection: view.selection,
        gradientEdit: view.gradientEdit,
        chrome: Object.fromEntries([name, "ModalViewport", "ModalBackdrop", "CanvasHost"]
            .map(key => [key, view.chrome[key] ?? null])),
        focusedControls: Object.keys(view.chrome).filter(key => view.chrome[key].focused),
    });
    const enterGradient = async (fill, before) => {
        const name = "PaintCanvasGradient" + fill, bounds = before.chrome[name];
        const input = { name, fill, before: { native: entryState(before, name) }, stage: "before" };
        const documentBefore = await documentState(page);
        entryInputs.push(input);
        let entryError;
        try {
            assert.equal(typeof bounds?.pointerOver, "boolean", "Gradient entry requires native pointerOver diagnostics from the current runtime");
            assert.equal(typeof bounds?.pressed, "boolean", "Gradient entry requires native Button pressed diagnostics from the current runtime");
            assert.equal(bounds.pressed, false, "The gradient entry button starts released");
            input.requested = { x: bounds.x + bounds.width / 2, y: bounds.y + bounds.height / 2 };
            await startInputObservation(page,
                ["pointermove", "pointerdown", "pointerup", "pointercancel", "mousedown", "mouseup", "click"], 64);
            input.before.observation = await readInputObservation(page);
            input.stage = "hover";
            // A completed scene can belong to an earlier layout batch. Native
            // hover establishes that the compositor now hits this actual button.
            await page.mouse.move(input.requested.x, input.requested.y);
            await wait((view, _document, target) => {
                const button = view.chrome[target.name];
                return button?.visible && button.enabled && button.measureValid && button.arrangeValid &&
                    button.pointerOver && !button.pressed &&
                    ["x", "y", "width", "height"].every(key => button[key] === target.bounds[key]);
            }, { name, bounds });
            input.hover = { native: entryState(await state(page), name), observation: await readInputObservation(page) };
            input.stage = "down";
            let pressError;
            try {
                await page.mouse.down();
                await wait((view, _document, name) => view.chrome[name]?.pressed === true, name);
            } catch (error) {
                pressError = error;
                input.pressError = error?.stack || String(error);
            } finally {
                // Capture the held state on both success and failure, before
                // the release changes the native pressed/capture diagnostics.
                input.down = {};
                try {
                    await captureWithinDeadline(async () => {
                        input.down.observation = await readInputObservation(page);
                        input.down.native = entryState(await state(page), name);
                        if (!pressError) {
                            assert.equal(input.down.native.chrome[name].pressed, true, "The intended native button owns the held press");
                            assert.equal(input.down.native.chrome.ModalBackdrop.visible, true, "The release-mode button stays in its modal while held");
                            assert.equal(input.down.native.gradientEdit.node, null, "Holding the button cannot activate gradient editing before release");
                        }
                    }, 5000);
                } catch (error) {
                    input.down.captureError = error?.stack || String(error);
                    pressError ??= error;
                }
                // Release exactly once even if native press readiness fails.
                // Preserve the press failure if releasing also fails.
                try { await page.mouse.up(); }
                catch (error) {
                    input.releaseError = error?.stack || String(error);
                    pressError ??= error;
                }
            }
            if (pressError) throw pressError;
            input.stage = "entry";
            await wait((view, _document, fill) => view.gradientEdit.node === 2 && view.gradientEdit.fill === fill &&
                !view.gradientEdit.active && !view.chrome.ModalBackdrop.visible && view.chrome.CanvasHost.focusWithin, fill);
            await published(before, "Enter native canvas gradient editor");
            const after = await state(page);
            assert.deepEqual(await documentState(page), documentBefore, "Entering native gradient editing cannot change the document");
            for (const key of ["documentRevision", "historyLength", "canUndo", "canRedo"]) {
                assert.equal(after[key], before[key], "Gradient entry preserves " + key);
            }
            assert.deepEqual(after.selection, before.selection, "Gradient entry preserves the selected layer");
            input.stage = "complete";
        } catch (error) {
            entryError = error;
            input.entryError = error?.stack || String(error);
        } finally {
            try {
                await captureWithinDeadline(async () => {
                    const observation = await stopInputObservation(page);
                    input.after = { observation };
                    input.after.native = entryState(await state(page), name);
                    if (!entryError) {
                        assert.equal(observation.droppedEvents, 0, "Gradient entry input evidence must be complete");
                        const events = observation.events.filter(event => event.phase === "capture");
                        const downs = events.filter(event => event.type === "pointerdown");
                        const ups = events.filter(event => event.type === "pointerup");
                        const moves = events.filter(event => event.type === "pointermove");
                        assert.equal(downs.length, 1, "Gradient entry has exactly one actual pointer press");
                        assert.equal(ups.length, 1, "Gradient entry has exactly one actual pointer release");
                        assert.equal(events.filter(event => event.type === "pointercancel").length, 0);
                        assert.ok(moves.length > 0, "Native hover follows an actual delivered pointer move");
                        const down = downs[0], up = ups[0], move = moves.at(-1);
                        assert.ok([move, down, up].every(event => event.trusted && event.pointerType === "mouse"));
                        assert.equal(down.button, 0); assert.equal(down.buttons, 1);
                        assert.equal(up.button, 0); assert.equal(up.buttons, 0);
                        assert.equal(down.pointerId, up.pointerId);
                        assert.ok(events.indexOf(move) < events.indexOf(down) && events.indexOf(down) < events.indexOf(up));
                        assert.ok(down.x >= bounds.x && down.x <= bounds.x + bounds.width &&
                            down.y >= bounds.y && down.y <= bounds.y + bounds.height,
                        "The delivered press remains inside the measured native button");
                        assert.equal(down.x, move.x); assert.equal(down.y, move.y);
                        assert.equal(up.x, down.x); assert.equal(up.y, down.y);
                    }
                }, 5000);
            } catch (error) {
                input.captureError = error?.stack || String(error);
                entryError ??= error;
            }
            try {
                await captureWithinDeadline(() => writeFile(resolve(output, mode + "-gradient-entry-input.json"),
                    JSON.stringify(entryInputs, null, 2)), 5000);
            } catch (error) {
                input.writeError = error?.stack || String(error);
                entryError ??= error;
            }
        }
        if (entryError) throw entryError;
    };
    const angularInputs = [];
    const saveAngularInputs = () => writeFile(resolve(output, mode + "-angular-gradient-input.json"), JSON.stringify(angularInputs, null, 2));
    const observeAngularPointer = async (type, action, count = 1) => {
        const input = { type };
        await startInputObservation(page, [type], 32);
        try { input.requested = await action(); }
        finally {
            input.observation = await stopInputObservation(page);
            angularInputs.push(input); await saveAngularInputs();
        }
        assert.equal(input.observation.droppedEvents, 0, "Angular input evidence must be complete");
        const events = input.observation.events.filter(event => event.type === type && event.phase === "capture");
        assert.equal(events.length, count, "Each requested angular pointer action supplies its actual positions");
        assert.ok(events.every(event => event.trusted && event.pointerType === "mouse" &&
            Number.isFinite(event.offsetX) && Number.isFinite(event.offsetY)));
        input.event = events.at(-1);
        return input;
    };
    const angularPress = (view, baseline, input, count = 1) => {
        const press = view.pointerPresses.entries.at(-1);
        assert.equal(press.serial, baseline.pointerPresses.serial + count);
        assert.equal(press.buttons, 1);
        near(press.world.x, (input.event.x - view.chrome.CanvasHost.x - press.pan.x) / press.zoom,
            "Observed angular press world X");
        near(press.world.y, (input.event.y - view.chrome.CanvasHost.y - press.pan.y) / press.zoom,
            "Observed angular press world Y");
        return press;
    };
    const world = (node, x, y) => {
        const matrix = node.affine_transform || { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        const dx = (matrix.a * x + matrix.c * y + matrix.e - node.width / 2) * (node.flip_x ? -1 : 1);
        const dy = (matrix.b * x + matrix.d * y + matrix.f - node.height / 2) * (node.flip_y ? -1 : 1);
        const angle = node.rotation * Math.PI / 180;
        return { x: node.x + node.width / 2 + dx * Math.cos(angle) - dy * Math.sin(angle),
            y: node.y + node.height / 2 + dx * Math.sin(angle) + dy * Math.cos(angle) };
    };
    const localDelta = (node, x, y) => {
        const zero = world(node, 0, 0), moved = world(node, x, y);
        return { x: moved.x - zero.x, y: moved.y - zero.y };
    };
    const sameTarget = (actual, expected) => actual?.kind === expected.kind && actual.stop === expected.stop;
    let fillIndex = 1;
    const paint = document => structuredClone(document.nodes[2].fills[fillIndex].paint);
    const assertPaint = (actual, expected) => {
        const normalized = structuredClone(expected);
        for (const endpoint of ["start", "end", "center"].filter(key => key in expected)) for (const coordinate of ["x", "y"]) {
            near(actual[endpoint][coordinate], expected[endpoint][coordinate], endpoint + "." + coordinate);
            normalized[endpoint][coordinate] = actual[endpoint][coordinate];
        }
        for (const scalar of ["radius", "angle"].filter(key => key in expected)) {
            near(actual[scalar], expected[scalar], scalar);
            normalized[scalar] = actual[scalar];
        }
        assert.equal(actual.stops.length, expected.stops.length, "Stop count");
        for (let index = 0; index < expected.stops.length; index++) {
            near(actual.stops[index].offset, expected.stops[index].offset, "Stop " + index + " offset");
            normalized.stops[index].offset = actual.stops[index].offset;
        }
        assert.deepEqual(actual, normalized, "Stop source order, colors and every non-coordinate paint field remain exact");
    };
    const assertChange = (before, after, expectedPaint) => {
        assertPaint(after.nodes[2].fills[fillIndex].paint, expectedPaint);
        const expected = structuredClone(before), node = expected.nodes[2];
        node.fills[fillIndex].paint = structuredClone(after.nodes[2].fills[fillIndex].paint);
        node.fill_style = null;
        delete node.variable_bindings.fill;
        node.overridden_properties = [...new Set([...node.overridden_properties, "fills"])].sort();
        assert.deepEqual(after, expected,
            "Only the chosen fill paint and its explicit layer-wide style/override metadata may change");
    };
    const waitPaint = expected => wait((_view, document, args) => {
        const actual = document.nodes[2].fills[args.fill].paint;
        return actual.type === args.paint.type &&
            ["start", "end", "center"].filter(key => key in args.paint).every(endpoint => ["x", "y"].every(coordinate =>
                Math.abs(actual[endpoint][coordinate] - args.paint[endpoint][coordinate]) < 1e-6)) &&
            ["radius", "angle"].filter(key => key in args.paint).every(key => Math.abs(actual[key] - args.paint[key]) < 1e-6) &&
            actual.stops.length === args.paint.stops.length && actual.stops.every((stop, index) =>
                Math.abs(stop.offset - args.paint.stops[index].offset) < 1e-6 &&
                JSON.stringify(stop.color) === JSON.stringify(args.paint.stops[index].color));
    }, { fill: fillIndex, paint: expected });
    const revealPaintCard = async name => {
        // Fill 1 follows a complete card and can start below the modal clip.
        // Publish each real wheel scroll before using the rearranged hit target;
        // there is still only one eventual button click and no action retry.
        await wait((view, _document, name) => {
            const bounds = view.chrome[name];
            return bounds?.visible && bounds.enabled && bounds.measureValid && bounds.arrangeValid &&
                bounds.width > 0 && bounds.height > 0;
        }, name);
        for (let scroll = 0; scroll < 16; scroll++) {
            const view = await state(page), bounds = view.chrome[name], clip = view.chrome.ModalViewport;
            assert.ok(bounds && clip, "Native paint card and modal viewport must exist");
            const x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
            if (x >= clip.x && x <= clip.x + clip.width && y >= clip.y + 3 && y <= clip.y + clip.height - 3) return;
            const delta = y - clip.y - clip.height / 2;
            await page.mouse.move(clip.x + clip.width / 2, clip.y + clip.height / 2);
            await page.mouse.wheel(0, Math.sign(delta) * Math.min(240, Math.max(80, Math.abs(delta))));
            await waitForNativeScene(page, view.sceneFrames, "Publish native paint-card scroll");
        }
        assert.fail("Could not reveal native gradient entry " + name);
    };
    const load = async (fixture, fill) => {
        fillIndex = fill;
        const previous = await state(page);
        await importFixture(page, fixture);
        await waitForNativeScene(page, previous.sceneFrames, "Gradient fixture import");
        await clickNative(page, "SelectTool");
        const selected = await state(page);
        if (selected.selection.length) {
            await page.keyboard.press("Escape");
            await wait(view => view.selection.length === 0);
            await published(selected, "Clear gradient fixture selection");
        }
        const node = fixture.nodes[2], center = world(node, node.width / 2, node.height / 2);
        await clickWorld(page, center.x, center.y);
        await wait(view => view.selection.join() === "2" && view.chrome.CanvasHost.focusWithin);
        const beforeDialog = await state(page);
        await clickNative(page, "EditPaintsButton", "InspectorViewport");
        await wait((view, _document, fill) => view.chrome.ModalBackdrop.visible &&
            view.chrome["PaintCanvasGradient" + fill]?.visible, fill);
        await waitForNativeScene(page, beforeDialog.sceneFrames, "Open native gradient paint card");
        await revealPaintCard("PaintCanvasGradient" + fill);
        const beforeEntry = await state(page);
        await enterGradient(fill, beforeEntry);
    };
    const handle = async (kind, stop = null) => {
        const view = await state(page);
        assert.equal(view.gradientEdit.node, 2);
        assert.equal(view.gradientEdit.fill, fillIndex);
        const item = view.gradientEdit.handles.find(item => item.kind === kind && item.stop === stop);
        assert.ok(item, "The native gradient editor exposes " + kind + " " + stop);
        for (const point of [item.local, item.world, item.screen, item.markerWorld, item.markerScreen]) {
            assert.ok(Number.isFinite(point.x) && Number.isFinite(point.y), "Native handle coordinates are finite");
        }
        return item;
    };
    const axis = async offset => {
        const { axis } = (await state(page)).gradientEdit;
        assert.ok(axis && Math.hypot(axis.end.x - axis.start.x, axis.end.y - axis.start.y) > 0);
        return { kind: "axis", stop: null, markerWorld: {
            x: axis.start.x + (axis.end.x - axis.start.x) * offset,
            y: axis.start.y + (axis.end.y - axis.start.y) * offset,
        } };
    };
    const stopDelta = async offset => {
        const { axis } = (await state(page)).gradientEdit;
        return { x: (axis.end.x - axis.start.x) * offset, y: (axis.end.y - axis.start.y) * offset };
    };
    const press = async item => {
        const requested = await moveWorld(page, item.markerWorld.x, item.markerWorld.y);
        await wait((view, _document, item) => view.gradientEdit.hover?.kind === item.kind &&
            view.gradientEdit.hover.stop === item.stop, { kind: item.kind, stop: item.stop });
        const before = await documentState(page), baseline = await state(page);
        const angular = paint(before).type === "angular_gradient" && ["end", "stop"].includes(item.kind);
        const input = angular ? await observeAngularPointer("pointerdown", () => page.mouse.down()) : null;
        if (!angular) await page.mouse.down();
        await wait((view, _document, item) => view.gradientEdit.active &&
            view.gradientEdit.selected?.kind === item.kind && view.gradientEdit.selected.stop === item.stop,
        { kind: item.kind, stop: item.stop });
        const pressed = await state(page);
        assert.deepEqual(await documentState(page), before, "Displaced gradient markers cannot jump the paint on press");
        assert.equal(pressed.documentRevision, baseline.documentRevision);
        assert.equal(pressed.historyLength, baseline.historyLength);
        assert.equal(pressed.canRedo, baseline.canRedo);
        item.angularInput = angular ? { node: before.nodes[2], paint: paint(before), part: { kind: item.kind, stop: item.stop },
            press: angularPress(pressed, baseline, input), down: input.event, moves: [],
            requestedDown: { offsetX: requested.x, offsetY: requested.y }, requestedMoves: [] } : null;
        return { before, baseline };
    };
    const move = async (item, delta, expected) => {
        if (item.angularInput) {
            const input = await observeAngularPointer("pointermove", () =>
                moveWorld(page, item.markerWorld.x + delta.x, item.markerWorld.y + delta.y));
            const original = item.angularInput, view = await state(page);
            assert.equal(view.zoom, original.press.zoom); assert.deepEqual(view.pan, original.press.pan);
            assert.equal(input.event.pointerId, original.down.pointerId);
            // Captured events can retarget from the host overlay to its parent
            // container; the actual coordinate origin must remain identical.
            near(input.event.offsetX - input.event.x, original.down.offsetX - original.down.x, "Angular pointer X origin");
            near(input.event.offsetY - input.event.y, original.down.offsetY - original.down.y, "Angular pointer Y origin");
            original.moves.push(input.event);
            original.requestedMoves.push({ offsetX: input.requested.x, offsetY: input.requested.y });
            assertPaint(angularPaintFromPointerInput({ ...original, zoom: view.zoom,
                down: original.requestedDown, moves: original.requestedMoves }), expected);
            expected = angularPaintFromPointerInput({ ...original, zoom: view.zoom });
            input.expectation = { original: structuredClone(original), paint: expected };
            await saveAngularInputs();
        } else {
            await moveWorld(page, item.markerWorld.x + delta.x, item.markerWorld.y + delta.y);
        }
        await waitPaint(expected);
        assert.equal((await state(page)).gradientEdit.active, true);
        return expected;
    };
    const release = async label => {
        const before = await state(page);
        await page.mouse.up();
        await wait(view => !view.gradientEdit.active);
        await published(before, label);
    };
    const drag = async (item, delta, expected) => {
        let held = false;
        try {
            held = true;
            const { baseline } = await press(item);
            expected = await move(item, delta, expected);
            assert.equal((await state(page)).historyLength, baseline.historyLength, "Held previews do not append history");
            await release("Commit native gradient drag"); held = false;
            return expected;
        } finally { if (held) await page.mouse.up(); }
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
        const restored = await state(page);
        assert.deepEqual(restored.selection, [2]);
        assert.equal(restored.gradientEdit.node, 2);
        assert.equal(restored.gradientEdit.fill, fillIndex);
    };
    const red = { r: 255, g: 0, b: 0, a: 255 }, blue = { r: 0, g: 0, b: 255, a: 255 };
    const fills = [
        { paint: { type: "solid", r: 20, g: 30, b: 40, a: 32 }, opacity: 0.25, visible: true },
        { paint: { type: "linear_gradient", start: { x: 0.125, y: 0.5 }, end: { x: 0.875, y: 0.5 }, stops: [
            { offset: 0, color: red }, { offset: 0.5, color: { r: 0, g: 255, b: 0, a: 192 } },
            { offset: 0.5, color: { ...blue, a: 128 } }, { offset: 1, color: { r: 255, g: 255, b: 255, a: 255 } },
        ] }, opacity: 0.85, visible: true },
        { paint: { type: "solid", r: 248, g: 250, b: 252, a: 255 }, opacity: 1, visible: true },
    ];
    const fixture = isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "rectangle", name: "Affine gradient with a shared fill stack", x: 240, y: 130,
            width: 240, height: 160, rotation: 17, flip_x: true,
            affine_transform: { a: 1.1, b: 0.15, c: 0.2, d: 0.9, e: 3, f: -5 },
            fills: structuredClone(fills), corner_radius: 6, corner_radii: null, corner_smoothing: 0,
            fill_style: 900, variable_bindings: { corner_radius: 901 } }),
        fixtureNode(reference, 3, { kind: "ellipse", name: "Unchanged linked sibling", x: 590, y: 160,
            width: 100, height: 80, fills: structuredClone(fills), fill_style: 900,
            corner_radius: 0, corner_radii: null }),
    ], "Native affine gradient handles");
    fixture.styles = { 900: { id: 900, name: "Shared gradient stack", description: "Preserve the definition and sibling",
        value: { type: "fill", fills: structuredClone(fills) } } };
    fixture.variable_mode = "Default";
    fixture.variables = { 901: { id: 901, name: "Independent radius", collection: "Shape",
        values: { Default: { type: "number", value: 6 } } } };
    fixture.next_id = 902;
    await load(fixture, 1);
    const original = await documentState(page), initial = await state(page);
    const firstTie = await handle("stop", 1), secondTie = await handle("stop", 2);
    near(firstTie.world.x, secondTie.world.x, "Tied stop world x");
    near(firstTie.world.y, secondTie.world.y, "Tied stop world y");
    assert.ok(Math.hypot(firstTie.markerScreen.x - secondTie.markerScreen.x,
        firstTie.markerScreen.y - secondTie.markerScreen.y) > 14, "Tied stops have separate native hit targets");
    const expectedEnd = world(original.nodes[2], 0.875 * 240, 0.5 * 160), endHandle = await handle("end");
    near(endHandle.world.x, expectedEnd.x, "Reflected/sheared endpoint world x");
    near(endHandle.world.y, expectedEnd.y, "Reflected/sheared endpoint world y");
    await moveWorld(page, firstTie.markerWorld.x, firstTie.markerWorld.y);
    await wait(view => view.gradientEdit.hover?.kind === "stop" && view.gradientEdit.hover.stop === 1);
    await published(initial, "Native gradient hover");
    let view = await state(page);
    assert.equal(view.documentRevision, initial.documentRevision);
    assert.equal(view.historyLength, initial.historyLength);
    assert.deepEqual(await documentState(page), original);
    await captureNativeScreenshot(mode + "-gradient-handles-tied-affine", { controls: ["CanvasHost"] });
    check("Native paint-card entry and immutable hover expose distinct tied-stop markers for one affine reflected gradient fill while preserving the shared style, radius binding, sibling and complete document");

    let expected = paint(original);
    expected.end = { x: 0.9375, y: 0.4375 };
    await drag(await handle("end"), localDelta(original.nodes[2], 15, -10), expected);
    const endpoint = await documentState(page);
    assertChange(original, endpoint, expected);
    await history(original, endpoint, initial.historyLength, "Affine gradient endpoint");
    check("A fractional endpoint drag uses the inverse full layer transform, changes only the chosen paint, detaches its shared fill style, preserves unrelated bindings and commits one exactly reversible edit");

    view = await state(page); await page.keyboard.press("Control+z");
    await published(view, "Prepare gradient cancellation redo");
    assert.deepEqual(await documentState(page), original);
    const cancellation = await state(page);
    assert.equal(cancellation.canRedo, true);
    let held = false, item = await handle("stop", 1);
    try {
        held = true; await press(item);
        expected = paint(original); expected.stops[1].offset = 0.625;
        const delta = await stopDelta(0.125);
        await move(item, delta, expected);
        assertChange(original, await documentState(page), expected);
        const beforeEscape = await state(page);
        await page.keyboard.press("Escape");
        await wait(view => !view.gradientEdit.active && view.gradientEdit.node === 2);
        await published(beforeEscape, "Cancel captured gradient stop");
        await moveWorld(page, item.markerWorld.x + delta.x * 1.2, item.markerWorld.y + delta.y * 1.2);
        await page.mouse.up(); held = false;
    } finally { if (held) await page.mouse.up(); }
    assert.deepEqual(await documentState(page), original);
    assert.equal((await state(page)).historyLength, cancellation.historyLength);
    assert.equal((await state(page)).canRedo, true);
    // A genuine round trip must also restore metadata, not merely coordinates.
    item = await handle("end");
    try {
        held = true; await press(item);
        expected = paint(original); expected.end.x += 0.0625;
        await move(item, localDelta(original.nodes[2], 15, 0), expected);
        await move(item, { x: 0, y: 0 }, paint(original));
        await release("Return gradient endpoint to original value"); held = false;
    } finally { if (held) await page.mouse.up(); }
    try {
        held = true; await press(await handle("stop", 2));
        await release("Unchanged gradient marker press"); held = false;
    } finally { if (held) await page.mouse.up(); }
    assert.deepEqual(await documentState(page), original);
    assert.equal((await state(page)).historyLength, cancellation.historyLength);
    assert.equal((await state(page)).canRedo, true);
    assert.deepEqual((await state(page)).selection, [2]);
    check("Escape, late pointer events, an exact drag-back and an unchanged tied-stop press restore every style/binding field and preserve a real redo entry without appending history");

    item = await handle("stop", 2);
    const touchView = await state(page), touchDelta = await stopDelta(-0.125);
    const css = point => ({ x: touchView.chrome.CanvasHost.x + touchView.pan.x + point.x * touchView.zoom,
        y: touchView.chrome.CanvasHost.y + touchView.pan.y + point.y * touchView.zoom });
    const touchStart = css(item.markerWorld), touchEnd = css({ x: item.markerWorld.x + touchDelta.x,
        y: item.markerWorld.y + touchDelta.y }), viewport = page.viewportSize();
    for (const point of [touchStart, touchEnd]) assert.ok(point.x > 0 && point.x < viewport.width &&
        point.y > 0 && point.y < viewport.height, "Touch uses in-viewport CSS pixels, without a device-scale multiplier");
    const client = await page.context().newCDPSession(page);
    let touchHeld = false, observing = false, observation;
    try {
        await startInputObservation(page, ["pointerdown", "pointermove", "pointercancel"]); observing = true;
        touchHeld = true;
        await client.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ ...touchStart, id: 37, force: 1 }] });
        await wait(view => view.gradientEdit.active && view.gradientEdit.selected?.kind === "stop" && view.gradientEdit.selected.stop === 2);
        assert.deepEqual(await documentState(page), original);
        await client.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ ...touchEnd, id: 37, force: 1 }] });
        expected = paint(original); expected.stops[2].offset = 0.375;
        await waitPaint(expected);
        assertChange(original, await documentState(page), expected);
        const beforeCancel = await state(page);
        await client.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] }); touchHeld = false;
        await wait(view => !view.gradientEdit.active);
        await published(beforeCancel, "Cancel gradient touch capture");
    } finally {
        try { if (touchHeld) await client.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] }); }
        finally {
            try {
                if (observing) {
                    observation = await stopInputObservation(page);
                    await writeFile(resolve(output, mode + "-gradient-handles-touch-input.json"), JSON.stringify(observation, null, 2));
                }
            } finally { await client.detach(); }
        }
    }
    for (const type of ["pointerdown", "pointermove", "pointercancel"]) assert.ok(observation.events.some(event =>
        event.type === type && event.phase === "capture" && event.trusted && event.pointerType === "touch"),
    "The production bridge must receive trusted touch " + type);
    assert.deepEqual(await documentState(page), original);
    view = await state(page);
    assert.equal(view.historyLength, cancellation.historyLength);
    assert.equal(view.canRedo, true); assert.deepEqual(view.selection, [2]);
    await page.keyboard.press("Control+Shift+z");
    await published(view, "Redo after canceled gradient gestures");
    assert.deepEqual(await documentState(page), endpoint);
    check("Real Chromium touch cancellation reaches the native gradient capture-loss path, restores the full shared-style snapshot and selection, and leaves the prior endpoint edit exactly redoable");

    let before = await documentState(page), depth = (await state(page)).historyLength;
    expected = paint(before); expected.stops[1].offset = 0.75;
    await drag(await handle("stop", 1), await stopDelta(0.25), expected);
    let after = await documentState(page);
    assertChange(before, after, expected);
    await history(before, after, depth, "Move one tied gradient stop past its neighbor");
    before = await documentState(page); depth = (await state(page)).historyLength;
    expected = paint(before); expected.stops[1].offset = 1;
    await drag(await handle("stop", 1), await stopDelta(0.375), expected);
    after = await documentState(page);
    assertChange(before, after, expected);
    await history(before, after, depth, "Clamp gradient stop to the axis end");
    check("Stop drags project onto the transformed world axis, clamp to 0–1, and retain authored stop indices and RGBA colors through crossings and ties with exact single-step undo/redo");

    before = await documentState(page); depth = (await state(page)).historyLength;
    expected = paint(before);
    for (const endpoint of ["start", "end"]) {
        expected[endpoint].x += 8 / before.nodes[2].width;
        expected[endpoint].y -= 6 / before.nodes[2].height;
    }
    await drag(await axis(0.25), localDelta(before.nodes[2], 8, -6), expected);
    after = await documentState(page);
    assertChange(before, after, expected);
    await history(before, after, depth, "Translate the complete gradient axis");
    await captureNativeScreenshot(mode + "-gradient-handles-translated", { controls: ["CanvasHost"] });
    check("Dragging a bare native gradient axis translates both relative endpoints together without changing stops, paint order, opacity, shape geometry, transforms or the linked sibling");

    const storage = () => page.evaluate(() => globalThis.ferroDesignStorage.status());
    const durable = () => page.evaluate(async () => JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text));
    const flushBaseline = async () => {
        const saved = await page.evaluate(async () => {
            let timeout;
            try {
                return await Promise.race([globalThis.ferroDesignStorage.flush(), new Promise((_, reject) => {
                    timeout = setTimeout(() => reject(new Error("Gradient baseline storage flush exceeded 30 seconds")), 30000);
                })]);
            } finally { clearTimeout(timeout); }
        });
        assert.equal(saved, true);
        const savedStatus = await storage(), native = await state(page);
        assert.equal(savedStatus.dirty, false); assert.equal(savedStatus.draftDirty, false);
        assert.equal(native.serializationPending, false);
        assert.deepEqual(await durable(), await documentState(page));
        return { savedStatus, native };
    };
    const saved = await flushBaseline();
    before = await documentState(page); item = await handle("end");
    let lifecycle;
    try {
        held = true; await press(item);
        expected = paint(before); expected.end.x += 10 / before.nodes[2].width;
        await move(item, localDelta(before.nodes[2], 10, 0), expected);
        lifecycle = await documentState(page); assertChange(before, lifecycle, expected);
        assert.equal((await state(page)).historyLength, saved.native.historyLength);
        assert.equal((await storage()).draftDirty, true);
        assert.equal((await state(page)).saveSerializations, saved.native.saveSerializations);
        assert.deepEqual(await durable(), before, "Held gradient previews do not replace the last durable snapshot");
        await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
        await page.waitForFunction(() => {
            const view = JSON.parse(globalThis.ferroDesign.ferroDesignState()), saved = globalThis.ferroDesignStorage.status();
            return !view.gradientEdit.active && !view.serializationPending && !saved.dirty && !saved.draftDirty && saved.phase === "saved";
        }, undefined, { timeout: 30000 });
        assert.deepEqual(await durable(), lifecycle);
        assert.equal((await state(page)).historyLength, saved.native.historyLength + 1);
        assert.equal((await state(page)).saveSerializations, saved.native.saveSerializations + 1);
        assert.equal((await storage()).revision, saved.savedStatus.revision + 1);
        await page.mouse.up(); held = false;
    } finally { if (held) await page.mouse.up(); }
    await frames(page);
    assert.deepEqual(await documentState(page), lifecycle);
    assert.deepEqual(await durable(), lifecycle);
    assert.equal((await state(page)).historyLength, saved.native.historyLength + 1);
    assert.equal((await state(page)).saveSerializations, saved.native.saveSerializations + 1);
    assert.equal((await storage()).revision, saved.savedStatus.revision + 1);
    await history(before, lifecycle, saved.native.historyLength, "Pagehide gradient gesture");
    check("From a clean IndexedDB baseline, a held gradient draft is committed by real pagehide alone into exactly one history entry, serialization and durable revision; late release cannot duplicate the commit");

    await flushBaseline();
    before = await documentState(page); depth = (await state(page)).historyLength; item = await handle("start");
    let portable;
    try {
        held = true; await press(item);
        expected = paint(before); expected.start.x += 6 / before.nodes[2].width; expected.start.y += 5 / before.nodes[2].height;
        await move(item, localDelta(before.nodes[2], 6, 5), expected);
        portable = await documentState(page); assertChange(before, portable, expected);
        const [download] = await Promise.all([page.waitForEvent("download", { timeout: 30000 }), page.keyboard.press("Control+s")]);
        assert.equal(await download.failure(), null);
        const path = resolve(output, mode + "-gradient-held-save.ferrodesign");
        await download.saveAs(path);
        assert.deepEqual(JSON.parse(await readFile(path, "utf8")), portable,
            "The actual Save shortcut serializes the committed held preview");
        await wait(view => !view.gradientEdit.active);
        assert.equal((await state(page)).historyLength, depth + 1);
        await page.mouse.up(); held = false;
    } finally { if (held) await page.mouse.up(); }
    await frames(page);
    assert.deepEqual(await documentState(page), portable);
    await history(before, portable, depth, "Save during gradient capture");
    check("Actual Ctrl+S finishes a held native gradient drag once before the portable download is serialized; the file contains every preview field and the late release preserves exact undo/redo");

    const simple = isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "rectangle", name: "Native gradient stop authoring", x: 80, y: 80, width: 240, height: 160,
        corner_radius: 0, corner_radii: null, corner_smoothing: 0,
        fills: [{ paint: { type: "linear_gradient", start: { x: 0, y: 0.5 }, end: { x: 1, y: 0.5 },
            stops: [{ offset: 0, color: red }, { offset: 1, color: blue }] }, opacity: 1, visible: true }],
    })], "Native gradient insertion and artwork");
    await load(simple, 0);
    before = await documentState(page); depth = (await state(page)).historyLength;
    const insertion = await axis(0.25);
    await clickWorld(page, insertion.markerWorld.x, insertion.markerWorld.y, { clickCount: 2, delay: 80 });
    await wait(view => view.gradientEdit.selected?.kind === "stop" && view.gradientEdit.selected.stop === 2);
    expected = paint(before); expected.stops.push({ offset: 0.25, color: { r: 191, g: 0, b: 64, a: 255 } });
    await waitPaint(expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Double-click native gradient insertion");
    // Re-select the newly authored stop through its actual marker after history,
    // then prove Tab traverses source indices rather than sorted offsets.
    try {
        held = true; await press(await handle("stop", 2));
        await release("Select inserted gradient stop"); held = false;
    } finally { if (held) await page.mouse.up(); }
    before = after; depth = (await state(page)).historyLength;
    expected = paint(before); expected.stops[2].offset = 0.251;
    await page.keyboard.press("Alt+ArrowRight");
    await waitPaint(expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Fine native gradient-stop keyboard nudge");
    const tabBaseline = await state(page);
    for (const target of [{ kind: "start", stop: null }, { kind: "end", stop: null },
        { kind: "stop", stop: 0 }, { kind: "stop", stop: 1 }, { kind: "stop", stop: 2 }]) {
        await page.keyboard.press("Tab");
        await wait((view, _document, target) => view.gradientEdit.selected?.kind === target.kind &&
            view.gradientEdit.selected.stop === target.stop && view.chrome.CanvasHost.focusWithin, target);
        assert.ok(sameTarget((await state(page)).gradientEdit.selected, target));
    }
    assert.deepEqual(await documentState(page), after);
    assert.equal((await state(page)).historyLength, tabBaseline.historyLength);
    assert.equal((await state(page)).documentRevision, tabBaseline.documentRevision);
    before = after; depth = (await state(page)).historyLength;
    await page.keyboard.press("Delete");
    await wait(view => view.gradientEdit.selected === null);
    expected = paint(before); expected.stops.splice(2, 1); await waitPaint(expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Delete one native gradient stop");
    check("A real axis double-click appends an interpolated stop, Alt+ArrowRight nudges its offset by 0.001, Tab visits authored source indices without editing, and Delete removes exactly that stop with exact history without deleting the layer");

    before = await documentState(page); depth = (await state(page)).historyLength;
    expected = paint(before); expected.start.x = 0.25;
    await drag(await handle("start"), { x: 60, y: 0 }, expected);
    const artwork = await documentState(page); assertChange(before, artwork, expected);
    view = await state(page); await page.keyboard.press("Control+z");
    await published(view, "Prepare two-stop rejection redo");
    assert.deepEqual(await documentState(page), before);
    try {
        held = true; await press(await handle("stop", 0));
        await release("Select protected minimum gradient stop"); held = false;
    } finally { if (held) await page.mouse.up(); }
    const rejected = await state(page), minimumMessage = "Keep at least two gradient stops.";
    assert.equal(rejected.canRedo, true);
    assert.ok(!(rejected.chrome.Toast?.visible && rejected.chrome.ToastText?.text?.includes(minimumMessage)),
        "The expected native rejection must not already be visible");
    await page.keyboard.press("Delete");
    await wait((view, _document, message) => view.chrome.Toast?.visible && view.chrome.ToastText?.text?.includes(message), minimumMessage);
    assert.deepEqual(await documentState(page), before);
    view = await state(page);
    assert.equal(view.historyLength, depth); assert.equal(view.canRedo, true);
    assert.ok(sameTarget(view.gradientEdit.selected, { kind: "stop", stop: 0 }));
    await page.keyboard.press("Control+Shift+z");
    await published(view, "Redo after rejected two-stop deletion");
    assert.deepEqual(await documentState(page), artwork);
    await page.keyboard.press("Enter");
    await wait(view => view.gradientEdit.node === null && view.chrome.CanvasHost.focusWithin);
    const [pngDownload] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport")]);
    assert.equal(await pngDownload.failure(), null);
    const pngPath = resolve(output, mode + "-gradient-handles-artwork.png");
    await pngDownload.saveAs(pngPath);
    const png = PNG.sync.read(await readFile(pngPath));
    assert.deepEqual([png.width, png.height], [240, 160]);
    const pixel = (x, y) => [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    const start = pixel(12, 80), midpoint = pixel(150, 80), end = pixel(236, 80);
    assert.ok(start[0] > 250 && start[1] === 0 && start[2] < 5 && start[3] === 255, "Native clamped red start: " + start);
    assert.ok(end[2] > 240 && end[1] === 0 && end[0] < 15 && end[3] === 255, "Native blue gradient end: " + end);
    assert.ok(Math.abs(midpoint[0] - midpoint[2]) < 4 && midpoint[1] === 0 && midpoint[3] === 255,
        "Native midpoint follows the authored endpoint movement: " + midpoint);
    assert.deepEqual(await documentState(page), artwork, "Leaving the editor and PNG export do not change artwork");
    assert.equal((await state(page)).invalidNodes, 0);
    assert.equal((await state(page)).unsupportedEffects, 0);
    check("Deleting below two stops produces a fresh native error while preserving exact redo; Enter exits the editor and native PNG pixels verify the edited gradient without adorners or unsupported rendering effects");

    // Native radial and angular paints reuse the same transaction/lifecycle
    // entry, but their geometric parameters and stop mappings remain distinct.
    const polarFixture = (kind, affine = true) => isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "rectangle", name: "Native " + kind + " gradient handles", x: affine ? 240 : 80, y: affine ? 130 : 80,
        width: 240, height: 160, rotation: affine ? 17 : 0, flip_x: affine,
        affine_transform: affine ? { a: 1.1, b: 0.15, c: 0.2, d: 0.9, e: 3, f: -5 } : null,
        corner_radius: 0, corner_radii: null, corner_smoothing: 0,
        fills: [{ paint: { type: kind + "_gradient", center: { x: 0.5, y: 0.5 },
            ...(kind === "radial" ? { radius: 0.5 } : { angle: 359 }),
            stops: [{ offset: 0, color: red }, { offset: 0.5, color: { r: 0, g: 255, b: 0, a: 192 } },
                { offset: 0.5, color: { ...blue, a: 128 } }, { offset: 1, color: blue }] }, opacity: 1, visible: true }],
    })], "Native " + kind + " authoring");
    await load(polarFixture("radial"), 0);
    assert.equal((await state(page)).gradientEdit.gradientKind, "radial");
    before = await documentState(page); depth = (await state(page)).historyLength;
    expected = paint(before); expected.radius = 0.625;
    await drag(await handle("end"), localDelta(before.nodes[2], 30, 0), expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Affine native radial radius");
    before = after; depth = (await state(page)).historyLength;
    expected = paint(before); expected.center.x += 12 / 240; expected.center.y -= 8 / 160;
    await drag(await handle("start"), localDelta(before.nodes[2], 12, -8), expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Affine native radial center");
    check("Native radial paint-card entry exposes the actual ellipse and center/radius controls; reflected and sheared pointer motion edits the shared relative radius and center with exact undo/redo and unchanged stops");

    view = await state(page); await page.keyboard.press("Control+z");
    await published(view, "Prepare radial redo preservation");
    before = await documentState(page); const radialRedo = await state(page);
    item = await handle("end");
    try {
        held = true; await press(item);
        expected = paint(before); expected.radius += 0.05;
        await move(item, localDelta(before.nodes[2], 12, 0), expected);
        await move(item, { x: 0, y: 0 }, paint(before));
        assert.deepEqual(await documentState(page), before);
        assert.equal((await state(page)).canRedo, true);
        await release("Radial exact drag-back"); held = false;
    } finally { if (held) await page.mouse.up(); }
    assert.deepEqual(await documentState(page), before);
    assert.equal((await state(page)).historyLength, radialRedo.historyLength);
    view = await state(page); await page.keyboard.press("Control+Shift+z");
    await published(view, "Execute radial redo after drag-back");
    assert.deepEqual(await documentState(page), after);
    await captureNativeScreenshot(mode + "-gradient-handles-radial", { controls: ["CanvasHost"] });
    check("Radial handle drag-back restores the exact document during capture, leaves redo executable, and paints a real native ellipse adorner without adding history");

    await load(polarFixture("angular"), 0);
    assert.equal((await state(page)).gradientEdit.gradientKind, "angular");
    const angularFirst = await handle("stop", 0), angularLast = await handle("stop", 3);
    near(angularFirst.world.x, angularLast.world.x, "Angular seam stop x");
    near(angularFirst.world.y, angularLast.world.y, "Angular seam stop y");
    assert.ok(Math.hypot(angularFirst.markerScreen.x - angularLast.markerScreen.x,
        angularFirst.markerScreen.y - angularLast.markerScreen.y) > 14, "Stops 0 and 1 stay separately selectable on the conic seam");
    const ring = (await state(page)).gradientEdit.ring;
    assert.ok(ring.length >= 33 && ring.length <= 513, "Native editing ring is bounded");
    before = await documentState(page); depth = (await state(page)).historyLength;
    const angularDelta = (node, angle, degrees, radius = Math.min(node.width, node.height) / 2) => {
        const a = angle * Math.PI / 180, b = (angle + degrees) * Math.PI / 180;
        return localDelta(node, (Math.sin(b) - Math.sin(a)) * radius, (-Math.cos(b) + Math.cos(a)) * radius);
    };
    item = await handle("end");
    try {
        held = true; await press(item);
        for (const degrees of [15, 30, 45]) {
            expected = paint(before); expected.angle += degrees;
            expected = await move(item, angularDelta(before.nodes[2], paint(before).angle, degrees), expected);
        }
        await release("Angular seam rotation"); held = false;
    } finally { if (held) await page.mouse.up(); }
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Native angular sweep rotation");
    check("Angular gradient handles preserve native local-pixel angles on a nonsquare affine box; a real sweep-handle drag crosses 360 degrees continuously while distinct seam stops retain their source order");

    before = after; depth = (await state(page)).historyLength;
    item = await handle("stop", 2);
    expected = paint(before); expected.stops[2].offset = 0.625;
    expected = await drag(item, angularDelta(before.nodes[2], paint(before).angle + 180, 45), expected);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Native conic color-stop rotation");
    check("Dragging a tied conic color stop uses the inverse layer transform and local sweep angle, preserving every other stop, RGBA color and authored gradient parameter with exact history");

    // A real native ring double-click inserts an interpolated stop; the spoke
    // is a center-translation target, not a fake linear stop axis.
    before = await documentState(page); depth = (await state(page)).historyLength;
    const angle = (paint(before).angle + 90) * Math.PI / 180;
    const angularInsertion = world(before.nodes[2], before.nodes[2].width * paint(before).center.x + Math.sin(angle) * 80,
        before.nodes[2].height * paint(before).center.y - Math.cos(angle) * 80);
    const beforeInsertion = await state(page);
    const insertionInput = await observeAngularPointer("pointerdown", () =>
        clickWorld(page, angularInsertion.x, angularInsertion.y, { clickCount: 2, delay: 80 }), 2);
    const insertionPress = angularPress(await state(page), beforeInsertion, insertionInput, 2);
    assert.equal(insertionPress.clickCount, 2, "The native second press performs the conic insertion");
    expected = angularInsertionFromPointerInput({ node: before.nodes[2], paint: paint(before), nativeWorld: insertionPress.world });
    insertionInput.expectation = { node: before.nodes[2], paint: paint(before), press: insertionPress, expected };
    await saveAngularInputs();
    await waitPaint(expected);
    await wait(view => view.gradientEdit.selected?.kind === "stop" && view.gradientEdit.selected.stop === 4 && !view.gradientEdit.active);
    after = await documentState(page); assertChange(before, after, expected);
    await history(before, after, depth, "Native angular ring insertion");
    await captureNativeScreenshot(mode + "-gradient-handles-angular", { controls: ["CanvasHost"] });
    check("Double-clicking the native conic ring appends a correctly interpolated RGBA stop without changing existing indices, and undo/redo preserves the complete angular paint");

    // An untransformed opaque radial fixture gives independent native artwork
    // samples at known normalized radii, with the adorner still active.
    const radialPixels = polarFixture("radial", false);
    radialPixels.nodes[2].fills[0].paint.stops = [{ offset: 0, color: red }, { offset: 1, color: blue }];
    await load(radialPixels, 0);
    before = await documentState(page); expected = paint(before); expected.radius = 0.75;
    await drag(await handle("end"), { x: 60, y: 0 }, expected);
    const radialArtwork = await documentState(page);
    const [radialDownload] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport")]);
    assert.equal(await radialDownload.failure(), null);
    const radialPath = resolve(output, mode + "-radial-gradient-handles-artwork.png");
    await radialDownload.saveAs(radialPath);
    const radialPng = PNG.sync.read(await readFile(radialPath));
    assert.deepEqual([radialPng.width, radialPng.height], [240, 160]);
    const radialPixel = (x, y) => [...radialPng.data.subarray((y * radialPng.width + x) * 4, (y * radialPng.width + x) * 4 + 4)];
    const radialCenter = radialPixel(120, 80), radialHorizontal = radialPixel(210, 80), radialVertical = radialPixel(120, 140);
    assert.ok(radialCenter[0] > 250 && radialCenter[2] < 5 && radialCenter[3] === 255, "Native radial center: " + radialCenter);
    for (const sample of [radialHorizontal, radialVertical]) {
        assert.ok(Math.abs(sample[0] - sample[2]) < 5 && sample[1] === 0 && sample[3] === 255,
            "Equal normalized X/Y radii produce the same native midpoint: " + sample);
    }
    assert.deepEqual(await documentState(page), radialArtwork);
    assert.equal((await state(page)).invalidNodes, 0);
    check("Native PNG pixels independently confirm the edited radial brush's equal relative radii on a nonsquare rectangle, with no exported handle adorners or changes from export");

    const angularPixels = polarFixture("angular", false);
    angularPixels.nodes[2].fills[0].paint.angle = 0;
    angularPixels.nodes[2].fills[0].paint.stops = [{ offset: 0, color: red }, { offset: 1, color: blue }];
    await load(angularPixels, 0);
    before = await documentState(page); expected = paint(before); expected.angle = 90;
    expected = await drag(await handle("end"), { x: 80, y: 80 }, expected);
    const angularArtwork = await documentState(page);
    assertChange(before, angularArtwork, expected);
    const [angularDownload] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "ExportPngButton", "InspectorViewport")]);
    assert.equal(await angularDownload.failure(), null);
    const angularPath = resolve(output, mode + "-angular-gradient-handles-artwork.png");
    await angularDownload.saveAs(angularPath);
    const angularPng = PNG.sync.read(await readFile(angularPath));
    assert.deepEqual([angularPng.width, angularPng.height], [240, 160]);
    const angularPixel = (x, y) => [...angularPng.data.subarray((y * angularPng.width + x) * 4, (y * angularPng.width + x) * 4 + 4)];
    const angularStart = angularPixel(200, 80), angularQuarter = angularPixel(120, 140), angularHalf = angularPixel(40, 80);
    assert.ok(angularStart[0] > 250 && angularStart[2] < 5 && angularStart[3] === 255,
        "Rotated native conic start faces right: " + angularStart);
    assert.ok(Math.abs(angularQuarter[0] - 191) < 4 && Math.abs(angularQuarter[2] - 64) < 4 && angularQuarter[1] === 0 && angularQuarter[3] === 255,
        "Native conic quarter-turn below center: " + angularQuarter);
    assert.ok(Math.abs(angularHalf[0] - angularHalf[2]) < 4 && angularHalf[1] === 0 && angularHalf[3] === 255,
        "Native conic half-turn left of center: " + angularHalf);
    assert.deepEqual(await documentState(page), angularArtwork);
    assert.equal((await state(page)).invalidNodes, 0);
    check("Native conic PNG samples prove a real canvas sweep-handle rotation changes the exported start ray and quarter/half-turn colors while preserving document state and excluding adorners");

}
