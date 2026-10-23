import assert from "node:assert/strict";

// Exercise the real bounded history through native keyboard input. Feature
// scenarios are isolated by reload, so this case deliberately crosses the
// production retention boundary instead of making other cases depend on it.
export async function boundedHistoryRegressions(page, reference, check, helpers) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, waitForNativeFrame } = helpers;
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        name: "Bounded native history", x: 40, y: 40, width: 80, height: 60, corner_radius: 0,
    })], "Native history retention boundary"));
    await clickWorld(page, 80, 70);
    assert.deepEqual((await state(page)).selection, [2]);
    const initial = await documentState(page);
    for (let index = 0; index < 101; index++) await page.keyboard.press("ArrowRight");
    await page.waitForFunction(x => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].x === x,
        initial.nodes[2].x + 101);
    const before = await documentState(page), beforeState = await state(page);
    assert.equal(beforeState.historyLength, 100, "Production history retains at most 100 committed edits");

    await page.keyboard.press("ArrowRight");
    await waitForNativeFrame(page, beforeState.renderedFrames, "Edit beyond history retention boundary");
    const after = await documentState(page);
    assert.equal(after.nodes[2].x, before.nodes[2].x + 1, "The edit beyond the cap is applied");
    assert.equal((await state(page)).historyLength, 100, "The oldest edit is evicted without growing the cap");

    let frame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+z");
    await waitForNativeFrame(page, frame, "Bounded history undo");
    assert.deepEqual(await documentState(page), before, "One undo restores every field of the latest edit at the cap");
    assert.equal((await state(page)).historyLength, 99);
    frame = (await state(page)).renderedFrames;
    await page.keyboard.press("Control+Shift+z");
    await waitForNativeFrame(page, frame, "Bounded history redo");
    assert.deepEqual(await documentState(page), after, "One redo restores every field after eviction");
    assert.equal((await state(page)).historyLength, 100);
    assert.equal((await state(page)).canRedo, false);
    check("Native keyboard editing crosses the 100-entry production history limit, evicts the oldest edit and retains exact one-step undo/redo for the newest transaction");
}
