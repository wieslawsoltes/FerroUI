import assert from "node:assert/strict";

// Import and explicit flush share one JavaScript task so the debounce cannot
// save before preview starts. Overflow enables clipping only in the live preview;
// the durable snapshot must retain the authored frame's clip_content: false.
export async function compactAutosaveRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, state, frames, flushStorage } = helpers;
    const root = 2;
    const fixture = isolatedDocument(reference, [
        fixtureNode(reference, root, {
            kind: "frame", name: "Authored overflow — café 🖌️", x: 0, y: 0,
            width: 320, height: 180, children: [3], corner_radius: 0,
            prototype_overflow: "vertical", clip_content: false,
        }),
        fixtureNode(reference, 3, {
            parent: root, name: "Below the frame\nUnicode survives compact storage ✓",
            x: 20, y: 260, width: 120, height: 60,
        }),
    ], "Compact authored autosave");
    fixture.pages[0].prototype_start = root;

    const captured = await page.evaluate((json) => {
        const api = globalThis.ferroDesign;
        const view = () => JSON.parse(api.ferroDesignState());
        if (!api.ferroDesignImportJson(json)) throw new Error("Compact autosave fixture import failed.");
        const pretty = api.ferroDesignExportJson();
        const authored = JSON.parse(pretty);
        const before = view();
        const presented = api.ferroDesignCommand("present");
        const presenting = view();
        const previewExport = api.ferroDesignExportJson();
        const committed = api.ferroDesignCommand("commit-edit");
        return { pretty, authored, before, presented, presenting, previewExport, committed, after: view() };
    }, JSON.stringify(fixture));

    assert.equal(captured.authored.nodes[root].clip_content, false);
    assert.equal(captured.authored.nodes[root].prototype_overflow, "vertical");
    assert.equal(captured.authored.pages[0].prototype_start, root);
    assert.equal(captured.before.serializationPending, true, "Import must schedule the snapshot before presentation starts");
    assert.equal(captured.presented, true);
    assert.equal(captured.presenting.presentation, root);
    assert.equal(captured.presenting.serializationPending, true, "Preview must begin while the authored snapshot is still pending");
    assert.equal(captured.presenting.saveSerializations, captured.before.saveSerializations,
        "No debounce serialization may run between import and preview in the same JavaScript task");
    assert.equal(captured.previewExport, captured.pretty, "Portable export must continue to read the authored snapshot during preview");
    assert.equal(captured.committed, true);
    assert.equal(captured.after.presentation, root, "Lifecycle commit must save without exiting presentation");
    assert.equal(captured.after.serializationPending, false);
    assert.equal(captured.after.saveSerializations, captured.before.saveSerializations + 1,
        "The explicit lifecycle flush must serialize exactly once");

    const persisted = await flushStorage(page);
    assert.equal(persisted.saved, true);
    assert.equal(persisted.backend, "indexedDB");
    assert.equal(persisted.dirty, false);
    assert.equal(persisted.serializationPending, false);
    const snapshot = await page.evaluate(async () => {
        const stored = await globalThis.ferroDesignStorage.backend.load();
        if (typeof stored?.text !== "string") throw new Error("No committed compact IndexedDB snapshot exists.");
        return { text: stored.text, revision: stored.revision };
    });
    assert.equal(snapshot.revision, persisted.revision, "The read transaction must return the committed storage revision");
    assert.deepEqual(JSON.parse(snapshot.text), captured.authored,
        "Durable autosave must preserve authored overflow and every field, including escaped newlines and Unicode");
    assert.ok(!snapshot.text.includes("\n"), "The durable snapshot must use compact JSON");
    assert.ok(captured.pretty.includes("\n"), "Portable document export must retain its established pretty format");
    assert.ok(Buffer.byteLength(snapshot.text) < Buffer.byteLength(captured.pretty),
        "Compact autosave must occupy fewer bytes than the equivalent portable export");

    await page.waitForFunction(({ root, afterFrame }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.presentation === root && view.renderedFrames > afterFrame;
    }, { root, afterFrame: captured.after.renderedFrames }, { timeout: 10000 });
    await frames(page);
    const rendered = await state(page);
    assert.equal(rendered.invalidNodes, 0, mode + " must render the saved preview without invalid nodes");
    assert.ok(rendered.visibleNodes > 0, mode + " must produce visible preview content after the flush");
    assert.equal(rendered.serializationPending, false);
    assert.equal(rendered.saveSerializations, captured.before.saveSerializations + 1,
        "Waiting for durable storage and rendering must not serialize a second time");

    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === null,
        undefined, { timeout: 10000 });
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson()), captured.pretty,
        "Stopping presentation must restore the exact authored document and its pretty export");
    const stopped = await state(page);
    assert.equal(stopped.serializationPending, false, "Restoring the authored snapshot must not schedule a new save");
    assert.equal(stopped.saveSerializations, captured.before.saveSerializations + 1,
        "Stopping presentation must preserve the exactly-once lifecycle serialization count");
    check("Compact IndexedDB autosave flushes once during preview, preserves the authored overflow snapshot and Unicode, uses fewer bytes, and restores exact portable export on Escape");
}
