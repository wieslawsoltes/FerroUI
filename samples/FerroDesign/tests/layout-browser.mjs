import assert from "node:assert/strict";

// The main gate supplies the real browser/app and imports the fixture. Every
// placement, sizing and reorder assertion below uses native controls or keys.
export async function layoutAuthoringRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, chooseNative, frames, captureNativeScreenshot } = helpers;
    const click = (name) => clickNative(page, name, "ModalViewport");
    const open = () => clickNative(page, "EditLayoutButton", "InspectorViewport");
    const done = () => clickNative(page, "LayoutDone");
    const field = async (name, value) => {
        await click(name);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await frames(page);
    };
    const next = (name, count = 1) => chooseNative(page, name, { offset: count, scrollContainer: "ModalViewport" });
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const waitError = () => page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.LayoutError?.visible === true);
    const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-6, actual + " != " + expected);
    const sizing = (values = {}) => ({
        width: "fill", height: "fixed", min_width: 0, max_width: null,
        min_height: 0, max_height: null, absolute: false, grow: 1, ...values,
    });
    const placement = (row, column, row_span = 1, column_span = 1) => ({ row, column, row_span, column_span });
    const grid = {
        axis: "grid", columns: 3, gap: 10, cross_gap: 5,
        padding: { top: 10, right: 10, bottom: 10, left: 10 },
        main_align: "start", cross_align: "start", width: "fixed", height: "hug", reverse: false, wrap: false,
    };
    const nodes = [
        fixtureNode(reference, 2, { kind: "frame", name: "Placement grid", x: 40, y: 40, width: 340, height: 60,
            children: [3, 4, 5, 6, 7], layout: grid, fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { parent: 2, name: "A", x: 10, y: 10, width: 100, height: 30, layout_child: sizing() }),
        fixtureNode(reference, 4, { parent: 2, name: "Pinned B", x: 120, y: 10, width: 100, height: 40,
            layout_child: sizing({ grid: placement(0, 1) }) }),
        fixtureNode(reference, 5, { parent: 2, name: "C", x: 230, y: 10, width: 100, height: 25, layout_child: sizing() }),
        fixtureNode(reference, 6, { parent: 2, name: "Hidden", x: 270, y: 18, width: 20, height: 30, visible: false,
            layout_child: sizing({ grid: placement(0, 1) }) }),
        fixtureNode(reference, 7, { parent: 2, name: "Absolute", x: 280, y: 18, width: 20, height: 30,
            layout_child: sizing({ absolute: true, grid: placement(0, 1) }) }),
    ];
    await importFixture(page, isolatedDocument(reference, nodes, "Native grid placement"));
    // Padding selects the frame, whose dialog initially targets its first child.
    await clickWorld(page, 44, 44);
    assert.deepEqual((await state(page)).selection, [2]);
    const initial = await documentState(page);
    const initialHistory = (await state(page)).historyLength;
    await open();
    await field("LayoutGridRow", "2");
    await field("LayoutGridColumn", "1");
    await field("LayoutGridRowSpan", "2");
    await field("LayoutGridColumnSpan", "2");
    await next("LayoutHeightMode", 2);
    await field("LayoutMinHeight", "90");
    await clickNative(page, "LayoutApply");
    await waitDocument((doc) => doc.nodes[3].layout_child.grid?.row_span === 2 && doc.nodes[3].height === 90);
    const authored = await documentState(page);
    const authoredHistory = (await state(page)).historyLength;
    assert.ok(authoredHistory >= initialHistory && authoredHistory <= initialHistory + 1);
    assert.deepEqual(authored.nodes[3].layout_child.grid, placement(1, 0, 2, 2), "Native one-based cells translate to zero-based stored placement");
    close(authored.nodes[3].x, 10);
    close(authored.nodes[3].y, 55);
    close(authored.nodes[3].width, 210);
    close(authored.nodes[3].height, 90);
    close(authored.nodes[2].height, 155);
    close(authored.nodes[4].x, 120);
    close(authored.nodes[5].x, 10);
    assert.deepEqual(authored.nodes[2].children, [3, 4, 5, 6, 7]);
    assert.deepEqual(authored.nodes[6], initial.nodes[6], "Hidden children reserve no grid cells and keep their geometry");
    assert.deepEqual(authored.nodes[7], initial.nodes[7], "Absolute children reserve no grid cells and keep their geometry");
    await captureNativeScreenshot(mode + "-grid-placement-spans", {
        controls: ["ModalContent", "LayoutGridRow", "LayoutApply"],
    });
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), initial, "One undo restores placement, child sizing, siblings and the Hug parent");
    assert.equal((await state(page)).historyLength, authoredHistory - 1);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), authored);
    check("Native one-based grid row, column and spans size a Fill child across tracks and gaps, update its Hug parent, exclude hidden/absolute layers and restore one-step history");

    await open();
    await field("LayoutGridRow", "1");
    await field("LayoutGridColumn", "2");
    await field("LayoutGridRowSpan", "1");
    await field("LayoutGridColumnSpan", "1");
    await clickNative(page, "LayoutApply");
    await waitError();
    assert.deepEqual(await documentState(page), authored, "A collision with a pinned sibling is rejected without partial changes");
    assert.equal((await state(page)).historyLength, authoredHistory);
    await field("LayoutGridColumnSpan", "0");
    await clickNative(page, "LayoutApply");
    await waitError();
    assert.deepEqual(await documentState(page), authored, "A zero span never reaches document mutation");
    assert.equal((await state(page)).historyLength, authoredHistory);
    await click("LayoutClearPlacement");
    await waitDocument((doc) => doc.nodes[3].layout_child.grid === undefined);
    const reset = await documentState(page);
    const resetHistory = (await state(page)).historyLength;
    assert.equal(reset.nodes[3].layout_child.min_height, 90, "Reset placement retains authored sizing");
    close(reset.nodes[3].x, 10);
    close(reset.nodes[3].width, 100);
    close(reset.nodes[2].height, 110);
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), authored, "Rejected drafts add no history before the placement reset");
    assert.equal((await state(page)).historyLength, resetHistory - 1);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reset);
    check("Native grid controls reject overlapping cells and zero spans atomically; Reset to Auto removes placement metadata while retaining sizing and exact undo");

    await open();
    await next("LayoutReorderTarget");
    await click("LayoutMoveAfter");
    await waitDocument((doc) => doc.nodes[2].children.join(",") === "4,5,3,6,7");
    const reordered = await documentState(page);
    const reorderedHistory = (await state(page)).historyLength;
    close(reordered.nodes[3].x, 230);
    close(reordered.nodes[5].x, 10);
    close(reordered.nodes[4].x, 120);
    assert.deepEqual(reordered.nodes[4].layout_child.grid, placement(0, 1));
    assert.deepEqual(reordered.nodes[6], reset.nodes[6]);
    assert.deepEqual(reordered.nodes[7], reset.nodes[7]);
    await captureNativeScreenshot(mode + "-layout-native-reorder", {
        controls: ["ModalContent", "LayoutReorderTarget", "LayoutMoveAfter"],
    });
    // Repeating the same relation must not create an extra history boundary.
    await next("LayoutReorderTarget");
    await click("LayoutMoveAfter");
    assert.equal((await state(page)).historyLength, reorderedHistory, "Repeated native reorder leaves the captured committed depth unchanged");
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reset);
    assert.equal((await state(page)).historyLength, reorderedHistory - 1);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reordered);
    check("Native Move after reorders automatic grid children while pinned cells and excluded layers retain placement; repeated no-op reorder adds no history");

    const reverseNodes = [
        fixtureNode(reference, 2, { kind: "frame", name: "Reversed row", x: 40, y: 40, width: 200, height: 60,
            children: [3, 4, 5], layout: { ...grid, axis: "row", reverse: true, height: "fixed" }, fills: [], corner_radius: 0 }),
        ...[[3, "A", 70], [4, "B", 40], [5, "C", 10]].map(([id, name, x]) => fixtureNode(reference, id, {
            parent: 2, name, x, y: 10, width: 20, height: 20, layout_child: sizing({ width: "fixed" }),
        })),
    ];
    await importFixture(page, isolatedDocument(reference, reverseNodes, "Native reversed flow order"));
    await clickWorld(page, 44, 44);
    const reverseInitial = await documentState(page);
    const reverseInitialHistory = (await state(page)).historyLength;
    await open();
    // The dialog starts at C, first in reversed flow. Each change rebuilds the
    // native editor, so focus its fresh child chooser for the next actual key.
    await next("LayoutChildChoice");
    await next("LayoutChildChoice");
    assert.equal((await state(page)).historyLength, reverseInitialHistory, "Browsing layout children changes no document history");
    await click("LayoutMoveBefore");
    await waitDocument((doc) => doc.nodes[2].children.join(",") === "4,5,3");
    const reverseReordered = await documentState(page);
    const reverseReorderedHistory = (await state(page)).historyLength;
    close(reverseReordered.nodes[3].x, 10);
    close(reverseReordered.nodes[5].x, 40);
    close(reverseReordered.nodes[4].x, 70);
    assert.equal(reverseReordered.nodes[2].layout.reverse, true);
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reverseInitial);
    assert.equal((await state(page)).historyLength, reverseReorderedHistory - 1);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reverseReordered);
    check("Native Move before follows reversed visual flow while preserving stored child identities, reverse settings and one-step undo/redo");
}
