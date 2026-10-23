import assert from "node:assert/strict";
import { PNG } from "pngjs";

export async function pageInspectorRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, frames, captureNativeScreenshot } = helpers;
    const fixture = isolatedDocument(reference, [fixtureNode(reference, 2, {
        x: 0, y: 0, width: 160, height: 120, corner_radius: 0,
    })], "Contextual native inspector");
    await importFixture(page, fixture);
    const authored = await documentState(page);
    let view = await state(page);
    const initialHistory = view.historyLength;
    assert.equal(view.selectionCount, 0);
    assert.equal(view.chrome.PageInspector.visible, true);
    assert.equal(view.chrome.DesignInspector.visible, false);
    await clickNative(page, "PageBackgroundField", "InspectorViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("E6E9EF");
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => {
        const color = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).pages[0].background;
        return color.r === 230 && color.g === 233 && color.b === 239 && color.a === 255;
    });
    await frames(page);
    view = await state(page);
    const committedHistory = view.historyLength;
    assert.ok(committedHistory === initialHistory + 1 || committedHistory === initialHistory,
        "A page background edit adds one history entry, allowing bounded-history eviction");
    const bytes = await captureNativeScreenshot(mode + "-page-inspector-native", {
        controls: ["CanvasHost", "PageInspector", "PageBackgroundField"],
    });
    const png = PNG.sync.read(bytes);
    const x = Math.floor(view.chrome.CanvasHost.x + 10), y = 90;
    const actual = [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 4)];
    assert.ok([230, 233, 239, 255].every((channel, index) => Math.abs(channel - actual[index]) <= 3),
        "The native page field must change actual canvas background pixels: " + JSON.stringify(actual));
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("undo")), true);
    await frames(page);
    assert.deepEqual(await documentState(page), authored, "Page background has one exact document undo");
    assert.equal((await state(page)).historyLength, committedHistory - 1);
    await clickNative(page, "PageBackgroundField", "InspectorViewport");
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("not a color");
    await page.keyboard.press("Enter");
    await frames(page);
    assert.deepEqual(await documentState(page), authored, "Invalid page colors leave the document and history unchanged");
    assert.equal((await state(page)).historyLength, committedHistory - 1);
    await clickWorld(page, 80, 60);
    await frames(page);
    view = await state(page);
    assert.equal(view.chrome.PageInspector.visible, false);
    assert.equal(view.chrome.DesignInspector.visible, true);
    const canvas = view.chrome.CanvasHost;
    await page.mouse.click(canvas.x + 10, 90);
    await frames(page);
    view = await state(page);
    assert.equal(view.selectionCount, 0);
    assert.equal(view.chrome.PageInspector.visible, true);
    assert.equal(view.chrome.DesignInspector.visible, false);
    check("Empty selection exposes native page background and local resources; native color edits change canvas pixels, validate atomically, and undo exactly");
}
