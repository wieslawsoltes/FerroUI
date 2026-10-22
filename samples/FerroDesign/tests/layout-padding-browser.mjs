import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

// Initial fixtures are the only supplied document mutations. Every authored
// side, link toggle, save and discard below uses real native pointer/key input.
export async function layoutPaddingRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, flushStorage, waitForNativeScene, captureNativeScreenshot } = helpers;
    const click = name => clickNative(page, name, "ModalViewport");
    const storage = () => page.evaluate(() => globalThis.ferroDesignStorage.status());
    const durable = () => page.evaluate(async () => JSON.parse((await globalThis.ferroDesignStorage.backend.load()).text));
    const pagehide = () => page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
    const wait = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return new Function("view", "args", "return (" + source + ")(view, args)")(view, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const waitPadding = async expected => {
        await page.waitForFunction(expected => {
            const padding = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].layout.padding;
            return Object.keys(expected).every(side => padding[side] === expected[side]);
        }, expected, { timeout: 10000 });
        assert.deepEqual((await documentState(page)).nodes[2].layout.padding, expected);
    };
    const open = async () => {
        const before = await state(page);
        await clickNative(page, "EditLayoutButton", "InspectorViewport");
        await wait(view => view.chrome.LayoutPaddingTop?.visible && view.chrome.LayoutPaddingLeft?.visible);
        await waitForNativeScene(page, before.sceneFrames, "Open native padding dialog");
    };
    const done = async () => {
        const before = await state(page);
        await clickNative(page, "LayoutDone", "ModalActions");
        await wait(view => !view.chrome.ModalBackdrop.visible && view.chrome.CanvasHost.focusWithin);
        await waitForNativeScene(page, before.sceneFrames, "Close native padding dialog");
    };
    const draft = async (side, value) => {
        const name = "LayoutPadding" + side;
        await click(name);
        assert.equal((await state(page)).chrome[name].focused, true);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        await frames(page);
        assert.equal((await state(page)).chrome[name].text, value);
    };
    const edit = async (side, value) => {
        await draft(side, value);
        const before = await state(page);
        await page.keyboard.press("Enter");
        await frames(page);
        const after = await state(page);
        if (after.documentRevision !== before.documentRevision ||
            after.chrome.LayoutError.visible !== before.chrome.LayoutError.visible) {
            await waitForNativeScene(page, before.sceneFrames, "Commit or validate native padding");
        }
    };
    const link = async checked => {
        assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, !checked);
        await click("LayoutPaddingLinked");
        await wait((view, checked) => view.chrome.LayoutPaddingLinked.checked === checked, checked);
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " adds one transaction");
        await page.keyboard.press("Control+z"); await frames(page);
        assert.deepEqual(await documentState(page), before, label + " undoes every document field");
        assert.equal((await state(page)).historyLength, depth);
        assert.equal((await state(page)).canRedo, true);
        await page.keyboard.press("Control+Shift+z"); await frames(page);
        assert.deepEqual(await documentState(page), after, label + " redoes every document field");
        assert.equal((await state(page)).historyLength, depth + 1);
    };
    const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-7, actual + " != " + expected);
    const initialPadding = { top: 10.125, right: 20.5625, bottom: 30.9375, left: 40.375 };
    const layout = {
        axis: "row", gap: 8, cross_gap: 12, padding: initialPadding,
        main_align: "start", cross_align: "start", width: "fixed", height: "fixed",
        columns: 3, reverse: false, wrap: false,
    };
    const sizing = { width: "fixed", height: "fixed", min_width: 0, min_height: 0,
        max_width: null, max_height: null, absolute: false, grow: 1 };
    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Independent padding", x: 40, y: 40, width: 300, height: 160,
            children: [3, 4], layout, fills: [], corner_radius: 0 }),
        fixtureNode(reference, 3, { parent: 2, name: "A", x: 40.375, y: 10.125, width: 40, height: 20, layout_child: sizing }),
        fixtureNode(reference, 4, { parent: 2, name: "B", x: 88.375, y: 10.125, width: 50, height: 30, layout_child: sizing }),
    ], "Native four-side layout padding"));
    await clickWorld(page, 44, 44);
    assert.deepEqual((await state(page)).selection, [2]);
    assert.equal((await state(page)).chrome.PaddingField.text, "", "The uniform inspector does not misreport asymmetric padding");
    const initial = await documentState(page), initialDepth = (await state(page)).historyLength;
    await open();
    assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, false);
    const fieldsBefore = Object.fromEntries(["Top", "Right", "Bottom", "Left"].map(side => [side, null]));
    for (const side of Object.keys(fieldsBefore)) fieldsBefore[side] = (await state(page)).chrome["LayoutPadding" + side].text;
    await link(true); await link(false);
    for (const [side, text] of Object.entries(fieldsBefore)) assert.equal((await state(page)).chrome["LayoutPadding" + side].text, text);
    assert.deepEqual(await documentState(page), initial, "Opening/linking/unlinking does not normalize asymmetric values");
    assert.equal((await state(page)).historyLength, initialDepth);
    await edit("Top", "18.5");
    await waitPadding({ ...initialPadding, top: 18.5 });
    const independent = await documentState(page);
    assert.deepEqual(independent.nodes[2].layout, { ...initial.nodes[2].layout, padding: { ...initialPadding, top: 18.5 } });
    close(independent.nodes[3].x, 40.375); close(independent.nodes[3].y, 18.5);
    close(independent.nodes[4].x, 88.375); close(independent.nodes[4].y, 18.5);
    assert.equal((await state(page)).chrome.LayoutPaddingTop.text, "18.50");
    await captureNativeScreenshot(mode + "-layout-independent-padding", { controls: ["LayoutPaddingTop", "LayoutPaddingLinked", "LayoutApplyPadding"] });
    await done(); await history(initial, independent, initialDepth, "Independent fractional padding");
    check("Native four-side fields open and toggle linking without mutation; one fractional side reflows children, preserves untouched numeric precision and unrelated layout settings, and has exact one-step undo/redo");

    await open(); await link(true);
    assert.deepEqual(await documentState(page), independent);
    const linkedDepth = (await state(page)).historyLength;
    await edit("Right", "25.75");
    await waitPadding({ top: 25.75, right: 25.75, bottom: 25.75, left: 25.75 });
    const linked = await documentState(page);
    assert.equal((await state(page)).historyLength, linkedDepth + 1);
    for (const side of ["Top", "Right", "Bottom", "Left"]) assert.equal((await state(page)).chrome["LayoutPadding" + side].text, "25.75");
    await done();
    await page.keyboard.press("Control+z"); await frames(page);
    assert.deepEqual(await documentState(page), independent);
    assert.equal((await state(page)).canRedo, true);
    await open(); await link(true); await link(false);
    await edit("Top", "18.50");
    await click("LayoutApplyPadding");
    assert.deepEqual(await documentState(page), independent);
    assert.equal((await state(page)).historyLength, linkedDepth);
    assert.equal((await state(page)).canRedo, true, "A real pending redo survives link toggles and equal submission");
    await done(); await page.keyboard.press("Control+Shift+z"); await frames(page);
    assert.deepEqual(await documentState(page), linked);
    check("Explicit linked editing applies one side to all four in one transaction; opening, relinking and equal-value Apply preserve a real redo entry and the complete authored document");

    await open(); await link(false);
    const invalidDepth = (await state(page)).historyLength;
    assert.equal((await state(page)).chrome.LayoutError.visible, false);
    await edit("Top", "NaN");
    await wait(view => view.chrome.LayoutError.visible && view.chrome.LayoutPaddingTop.focused);
    await edit("Right", "44.5");
    await wait(view => view.chrome.LayoutError.visible && view.chrome.LayoutPaddingRight.focused);
    assert.deepEqual(await documentState(page), linked, "A valid sibling draft cannot partially apply while another side is invalid");
    assert.equal((await state(page)).historyLength, invalidDepth);
    await click("LayoutPaddingLinked");
    assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, false, "Link changes cannot discard an invalid group");
    assert.equal((await state(page)).chrome.LayoutPaddingTop.text, "NaN");
    assert.equal((await state(page)).chrome.LayoutPaddingRight.text, "44.5");
    await click("LayoutPaddingRight"); await page.keyboard.press("Escape"); await frames(page);
    assert.equal((await state(page)).chrome.LayoutPaddingRight.focused, true);
    assert.deepEqual((await state(page)).selection, [2]);
    assert.equal((await state(page)).chrome.LayoutError.visible, false);
    for (const side of ["Top", "Right", "Bottom", "Left"]) assert.equal((await state(page)).chrome["LayoutPadding" + side].text, "25.75");
    assert.deepEqual(await documentState(page), linked);
    assert.equal((await state(page)).historyLength, invalidDepth);
    await edit("Top", "-1"); await edit("Right", "44.5"); await edit("Top", "31.25");
    await waitPadding({ top: 31.25, right: 44.5, bottom: 25.75, left: 25.75 });
    const repaired = await documentState(page);
    assert.equal((await state(page)).historyLength, invalidDepth + 1);
    await done(); await history(linked, repaired, invalidDepth, "Atomic repaired padding group");
    check("All four native padding drafts validate together: nonfinite/negative input blocks partial edits and relinking; Escape restores the complete group without losing focus or selection, and repairing two sides commits once");

    const emptyLayout = { ...layout, padding: { top: 16, right: 16, bottom: 16, left: 16 } };
    await importFixture(page, isolatedDocument(reference, [fixtureNode(reference, 2, {
        kind: "frame", name: "Empty layout frame", x: 40, y: 40, width: 240, height: 160,
        children: [], layout: emptyLayout, fills: [], corner_radius: 0,
    })], "Empty frame padding lifecycle"));
    await clickWorld(page, 44, 44); await open();
    assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, true);
    assert.equal((await state(page)).chrome.LayoutChildChoice?.visible === true, false);
    await link(false);
    assert.equal((await flushStorage(page)).saved, true);
    const beforeLifecycle = await documentState(page), saved = await state(page), savedStorage = await storage();
    assert.equal(savedStorage.dirty, false); assert.equal(savedStorage.draftDirty, false);
    await draft("Left", "29.125");
    assert.deepEqual(await documentState(page), beforeLifecycle);
    assert.equal((await storage()).draftDirty, true);
    await pagehide();
    await page.waitForFunction(() => {
        const native = JSON.parse(globalThis.ferroDesign.ferroDesignState()), storage = globalThis.ferroDesignStorage.status();
        return !native.serializationPending && !storage.dirty && !storage.draftDirty && storage.phase === "saved";
    }, undefined, { timeout: 10000 });
    await waitPadding({ top: 16, right: 16, bottom: 16, left: 29.125 });
    const lifecycle = await documentState(page);
    assert.deepEqual(await durable(), lifecycle);
    assert.equal((await state(page)).historyLength, saved.historyLength + 1);
    assert.equal((await state(page)).saveSerializations, saved.saveSerializations + 1);
    assert.equal((await storage()).revision, savedStorage.revision + 1);
    await pagehide(); await frames(page);
    assert.deepEqual(await documentState(page), lifecycle);
    assert.equal((await state(page)).historyLength, saved.historyLength + 1);
    assert.equal((await state(page)).saveSerializations, saved.saveSerializations + 1);
    await captureNativeScreenshot(mode + "-empty-layout-padding", { controls: ["LayoutPaddingLeft", "LayoutSave", "LayoutDone"] });
    await done(); await history(beforeLifecycle, lifecycle, saved.historyLength, "Pagehide empty-frame padding");
    check("Empty auto-layout frames expose every padding side before child-only controls; from a clean baseline, real pagehide commits a focused fractional draft into exactly one history step, serialization and IndexedDB revision");

    await open();
    assert.equal((await state(page)).chrome.LayoutPaddingLinked.checked, false);
    assert.equal((await flushStorage(page)).saved, true);
    const beforeIme = await documentState(page), imeDepth = (await state(page)).historyLength;
    await draft("Bottom", "27.125"); await page.keyboard.press("Control+a");
    const ime = page.locator(".ferroui-input-element");
    assert.equal(await ime.count(), 1);
    try {
        await ime.evaluate(input => {
            if (document.activeElement !== input) throw new Error("Padding field must own the production IME bridge");
            input.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "" }));
            input.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data: "未確定" }));
        });
        assert.equal((await state(page)).chrome.LayoutPaddingBottom.composing, true);
        assert.deepEqual(await documentState(page), beforeIme);
        await pagehide();
        assert.equal((await flushStorage(page)).saved, true);
    } finally {
        if (await ime.count()) {
            await ime.evaluate(input => {
                if (document.activeElement === input) input.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "" }));
            });
        }
    }
    await waitPadding({ ...beforeIme.nodes[2].layout.padding, bottom: 27.125 });
    const confirmed = await documentState(page);
    assert.deepEqual(await durable(), confirmed);
    assert.equal((await state(page)).historyLength, imeDepth + 1);
    assert.equal((await state(page)).chrome.LayoutPaddingBottom.composing, false);
    await done(); await history(beforeIme, confirmed, imeDepth, "Confirmed IME padding draft");
    check("Pagehide during production native padding composition saves the last confirmed number once, preserves every other side exactly, and never stores temporary selection deletion or IME preedit");

    await open();
    const beforeSave = await documentState(page), saveDepth = (await state(page)).historyLength;
    await draft("Right", "32.5");
    assert.deepEqual(await documentState(page), beforeSave);
    const [download] = await Promise.all([
        page.waitForEvent("download", { timeout: 60000 }),
        clickNative(page, "LayoutSave", "ModalActions"),
    ]);
    assert.equal(await download.failure(), null);
    await wait(view => !view.chrome.ModalBackdrop.visible);
    await waitPadding({ ...beforeSave.nodes[2].layout.padding, right: 32.5 });
    const portable = await documentState(page);
    const path = resolve(output, mode + "-layout-padding-save.ferrodesign");
    await download.saveAs(path);
    assert.deepEqual(JSON.parse(await readFile(path, "utf8")), portable);
    assert.equal((await state(page)).historyLength, saveDepth + 1);
    assert.equal((await flushStorage(page)).saved, true);
    assert.deepEqual(await durable(), portable);
    await history(beforeSave, portable, saveDepth, "Native Save padding draft");
    check("The real native Save design button commits the focused padding draft once before downloading complete editable JSON; durable storage, undo and redo preserve every side and unrelated field");
}
