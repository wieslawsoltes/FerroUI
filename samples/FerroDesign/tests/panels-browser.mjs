import assert from "node:assert/strict";

export async function floatingPanelRegressions(page, reference, check, helpers, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const inside = (box, width, height) => {
        assert.ok(box.visible && box.width > 0 && box.height > 0);
        assert.ok(box.x >= 15 && box.y >= 15 && box.x + box.width <= width - 15
            && box.y + box.height <= height - 15, "Native palette must remain reachable: " + JSON.stringify(box));
    };
    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { name: "Palette source", x: 40, y: 40, width: 160, height: 140, effects: [] }),
        fixtureNode(reference, 3, { name: "Palette destination", x: 600, y: 40, width: 160, height: 140, effects: [] }),
    ], "Native floating palettes"));
    await clickWorld(page, 100, 100);
    await clickNative(page, "EffectButton", "InspectorViewport");
    const before = await state(page), document = await documentState(page);
    assert.equal(before.floatingPanel, true);
    inside(before.chrome.ModalPanel, 1600, 1000);
    const header = before.chrome.ModalHeader;
    // Drag the real native header to the left side. The control retains input
    // capture even as the pointer crosses the canvas and navigation rail.
    const start = { x: header.x + 50, y: header.y + header.height / 2 };
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(start.x - before.chrome.ModalPanel.x + 16, start.y + 120, { steps: 8 });
    await page.mouse.up();
    await frames(page);
    const moved = await state(page);
    inside(moved.chrome.ModalPanel, 1600, 1000);
    assert.ok(Math.abs(moved.chrome.ModalPanel.x - 16) <= 2, "Dragging changes native panel position");
    assert.equal(moved.historyLength, before.historyLength);
    assert.deepEqual(await documentState(page), document, "Moving chrome cannot edit design geometry or history");
    check("Native authoring palettes drag through a retained FerroUI transform, clamp to the window, and leave document geometry/history untouched");

    await clickWorld(page, 680, 100);
    const selected = await state(page);
    assert.deepEqual(selected.selection, [3], "Canvas input remains active outside the palette");
    assert.equal(selected.chrome.ModalBackdrop.visible, true);
    assert.ok(Math.abs(selected.chrome.ModalPanel.x - moved.chrome.ModalPanel.x) <= 2,
        "Selection refresh keeps the user's palette position");
    await clickNative(page, "EffectAdd", "ModalActions");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[3].effects.length === 1);
    const edited = await documentState(page);
    assert.equal(edited.nodes[2].effects.length, 0, "A refreshed palette cannot edit its former layer");
    await captureNativeScreenshot(mode + "-floating-native-palette", { controls: ["ModalPanel", "ModalHeader", "EffectType0"] });
    check("A modeless native palette keeps canvas input available, follows the new selection, retains its position, and applies properties only to that selection");

    await page.setViewportSize({ width: 980, height: 650 });
    await page.waitForFunction(() => {
        const canvas = document.querySelector("canvas.ferroui-canvas");
        const panel = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalPanel;
        return canvas?.width === 980 && canvas.height === 650 && panel.arrangeValid
            && panel.x >= 15 && panel.y >= 15 && panel.x + panel.width <= 965 && panel.y + panel.height <= 635;
    });
    inside((await state(page)).chrome.ModalPanel, 980, 650);
    await clickNative(page, "CloseModalButton");
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, false);
    await page.mouse.move(700, 550);
    assert.deepEqual(await documentState(page), edited, "Closed palettes release their gesture without a late document edit");
    await page.setViewportSize({ width: 1600, height: 1000 });
    await page.waitForFunction(() => document.querySelector("canvas.ferroui-canvas")?.width === 1600);
    await frames(page);
    check("Native palettes remain fully reachable after shrinking to the supported desktop minimum, and closing releases pointer ownership");
}
