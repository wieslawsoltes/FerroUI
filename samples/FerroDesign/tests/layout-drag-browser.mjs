import assert from "node:assert/strict";

// Drag native canvas children through the real pointer capture path. Preview
// geometry is observable; it must never become an intermediate document edit.
export async function layoutDragRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, frames, captureNativeScreenshot } = helpers;
    const sizing = (extra = {}) => ({
        width: "fixed", height: "fixed", min_width: 0, max_width: null,
        min_height: 0, max_height: null, absolute: false, grow: 1, ...extra,
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
    const begin = async (from, to) => {
        const view = await state(page), start = screen(view, from), end = screen(view, to);
        await page.mouse.move(start.x, start.y);
        await page.mouse.down();
        await page.mouse.move(end.x, end.y, { steps: 12 });
        await waitDrag(true);
        return { view, start, end, drag: (await state(page)).layoutDrag };
    };
    const waitOrder = (order) => page.waitForFunction((order) =>
        JSON.stringify(JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].children) === JSON.stringify(order),
    order, { timeout: 10000 });

    const cases = [
        { name: "row", axis: "row", excluded: true },
        { name: "column", axis: "column" },
        { name: "reversed-row", axis: "row", reverse: true },
        { name: "wrapped-row", axis: "row", wrap: true },
        { name: "grid", axis: "grid" },
        { name: "affine-row", axis: "row", affine: { a: 1.1, b: 0.2, c: 0.25, d: 0.85, e: 0, f: 0 } },
    ];
    const fixture = (variant) => {
        const column = variant.axis === "column", wrapped = variant.wrap || variant.axis === "grid";
        const slot = (index) => column ? { x: 10, y: 10 + index * 58 }
            : wrapped ? { x: 10 + index % 2 * 74, y: 10 + Math.floor(index / 2) * 58 }
            : { x: 10 + index * 74, y: 10 };
        const transform = variant.affine ?? { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        const world = (point) => ({
            x: 40 + transform.a * point.x + transform.c * point.y + transform.e,
            y: 40 + transform.b * point.x + transform.d * point.y + transform.f,
        });
        const order = variant.excluded ? [3, 7, 4, 8, 5, 6] : [3, 4, 5, 6];
        const nodes = [fixtureNode(reference, 2, {
            kind: "frame", name: variant.name + " layout", x: 40, y: 40,
            width: column ? 140 : wrapped ? 158 : 340, height: column ? 270 : 150,
            children: order, fills: [], corner_radius: 0, affine_transform: variant.affine ?? null,
            layout: {
                axis: variant.axis, columns: 2, gap: 10, cross_gap: 10,
                padding: { top: 10, right: 10, bottom: 10, left: 10 },
                main_align: "start", cross_align: "start", width: "fixed", height: "fixed",
                reverse: variant.reverse ?? false, wrap: variant.wrap ?? false,
            },
        })];
        for (let index = 0; index < 4; index++) {
            const position = slot(variant.reverse ? 3 - index : index);
            nodes.push(fixtureNode(reference, index + 3, {
                parent: 2, name: String.fromCharCode(65 + index), ...position,
                width: 64, height: 48, corner_radius: 4, layout_child: sizing(),
                fills: [{ paint: { type: "solid", r: 40 + index * 45, g: 110, b: 210 - index * 30, a: 255 }, opacity: 1, visible: true }],
            }));
        }
        if (variant.excluded) {
            nodes.push(fixtureNode(reference, 7, { parent: 2, name: "Absolute slot", x: 10, y: 110, width: 24, height: 20,
                layout_child: sizing({ absolute: true }) }));
            nodes.push(fixtureNode(reference, 8, { parent: 2, name: "Hidden slot", x: 100, y: 110, width: 24, height: 20,
                visible: false, layout_child: sizing() }));
        }
        const first = slot(variant.reverse ? 3 : 0), last = slot(variant.reverse ? 0 : 3);
        const source = world({ x: first.x + 32, y: first.y + 24 });
        const destination = world(column ? { x: last.x + 32, y: last.y + 53 }
            : { x: last.x + (variant.reverse ? -5 : 69), y: last.y + 24 });
        const marker = column
            ? { start: world({ x: last.x, y: last.y + 48 }), end: world({ x: last.x + 64, y: last.y + 48 }) }
            : { start: world({ x: last.x + (variant.reverse ? 0 : 64), y: last.y }),
                end: world({ x: last.x + (variant.reverse ? 0 : 64), y: last.y + 48 }) };
        return { document: isolatedDocument(reference, nodes, "Native " + variant.name + " drag"),
            source, destination, marker, slot, world,
            order: variant.excluded ? [4, 7, 5, 8, 6, 3] : [4, 5, 6, 3] };
    };

    for (const variant of cases) {
        const example = fixture(variant);
        await importFixture(page, example.document);
        await clickWorld(page, example.source.x, example.source.y);
        assert.deepEqual((await state(page)).selection, [3], variant.name + " must select the actual child, not its frame");
        const before = await documentState(page), beforeState = await state(page);
        const gesture = await begin(example.source, example.destination);
        assert.equal(gesture.drag.child, 3);
        assert.equal(gesture.drag.parent, 2);
        assert.equal(gesture.drag.target.index, variant.reverse ? 0 : 3);
        for (const endpoint of ["start", "end"]) for (const axis of ["x", "y"]) {
            close(gesture.drag.target.marker[endpoint][axis], example.marker[endpoint][axis],
                variant.name + " world insertion marker " + endpoint + "." + axis);
        }
        assert.deepEqual(await documentState(page), before, "Pointer-frequency " + variant.name + " preview must not write node positions or order");
        assert.equal((await state(page)).historyLength, beforeState.historyLength);
        assert.equal((await state(page)).canRedo, beforeState.canRedo);
        if (variant.name === "row") {
            await captureNativeScreenshot(mode + "-native-layout-drag-preview", {
                controls: ["CanvasHost"], afterFrame: gesture.view.renderedFrames,
            });
        }
        await page.mouse.up();
        await waitDrag(false);
        await waitOrder(example.order);
        const after = await documentState(page), afterState = await state(page);
        const flow = example.order.filter((id) => id >= 3 && id <= 6);
        if (variant.reverse) flow.reverse();
        for (let index = 0; index < flow.length; index++) {
            const expected = example.slot(index), node = after.nodes[flow[index]];
            close(node.x, expected.x, variant.name + " reflow x");
            close(node.y, expected.y, variant.name + " reflow y");
            assert.deepEqual([node.width, node.height], [64, 48]);
        }
        for (const id of variant.excluded ? [7, 8] : []) {
            assert.deepEqual(after.nodes[id], before.nodes[id], "Excluded children retain geometry, authored data, and fixed sibling slots");
        }
        assert.equal(after.nodes[2].layout.reverse, variant.reverse ?? false);
        assert.deepEqual(after.nodes[2].affine_transform, before.nodes[2].affine_transform);
        await page.keyboard.press("Control+z");
        await frames(page);
        assert.deepEqual(await documentState(page), before, variant.name + " drop must be one undo including all reflowed siblings");
        assert.equal((await state(page)).historyLength, afterState.historyLength - 1);
        await page.keyboard.press("Control+Shift+z");
        await frames(page);
        assert.deepEqual(await documentState(page), after, variant.name + " redo must restore exact order and layout");
        assert.equal((await state(page)).historyLength, afterState.historyLength);
    }
    check("Native child drags reorder row, column, reversed, wrapped, grid and affine layouts using exact world insertion markers; deferred previews preserve the document and each drop is one undo/redo");

    const example = fixture(cases[0]);
    await importFixture(page, example.document);
    await clickWorld(page, example.source.x, example.source.y);
    const before = await documentState(page), beforeState = await state(page);
    let gesture = await begin(example.source, example.destination);
    await page.mouse.move(gesture.start.x, gesture.start.y, { steps: 8 });
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).layoutDrag?.target?.index === 0);
    await page.mouse.up();
    await waitDrag(false);
    assert.deepEqual(await documentState(page), before, "Returning to the original child rectangle must be a document no-op");
    assert.equal((await state(page)).historyLength, beforeState.historyLength);
    assert.equal((await state(page)).canRedo, beforeState.canRedo);

    gesture = await begin(example.source, example.destination);
    await page.keyboard.press("Escape");
    await waitDrag(false);
    await page.mouse.move(gesture.end.x - 20, gesture.end.y + 15, { steps: 4 });
    await page.mouse.up();
    assert.deepEqual(await documentState(page), before, "Escape and late pointer events must discard the retained drag without any restoration write");
    assert.equal((await state(page)).historyLength, beforeState.historyLength);
    assert.equal((await state(page)).canRedo, beforeState.canRedo);
    check("Native drag-return and Escape cancel immutable layout previews without document/revision/history changes; late pointer motion and release cannot commit a canceled drag");

    await begin(example.source, example.destination);
    await page.mouse.up();
    await waitOrder(example.order);
    const committed = await documentState(page);
    const committedDepth = (await state(page)).historyLength;
    const finalPosition = example.slot(3);
    const newSource = example.world({ x: finalPosition.x + 32, y: finalPosition.y + 24 });
    const firstSlot = example.slot(0);
    const newTarget = example.world({ x: firstSlot.x - 5, y: firstSlot.y + 24 });
    gesture = await begin(newSource, newTarget);
    await page.keyboard.press("Control+z");
    await waitDrag(false);
    await page.mouse.move(gesture.end.x + 20, gesture.end.y + 10, { steps: 4 });
    await page.mouse.up();
    assert.deepEqual(await documentState(page), committed, "The first Undo while captured owns and cancels only the current preview");
    assert.equal((await state(page)).historyLength, committedDepth, "Canceling an uncommitted preview cannot consume document history");
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), before, "The next Undo restores the previous complete drop");
    assert.equal((await state(page)).historyLength, committedDepth - 1);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), committed, "Canceled pointer release must preserve the previous drop's redo entry");
    check("Native Undo during a held layout drag cancels only its preview; a second Undo restores the previous drop, and late pointer release preserves exact redo");
}
