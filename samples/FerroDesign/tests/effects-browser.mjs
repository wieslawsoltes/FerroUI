import assert from "node:assert/strict";

// Author the real FerroUI effect stack with native pointer and keyboard input.
// The core renderer/pixel regressions exercise the same serialized effects.
export async function effectAuthoringRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, captureNativeScreenshot } = helpers;
    const click = (name) => clickNative(page, name, "ModalViewport");
    const open = () => clickNative(page, "EffectButton", "InspectorViewport");
    const done = () => clickNative(page, "EffectDone");
    const field = async (name, value, commit = true) => {
        await click(name);
        assert.equal((await state(page)).chrome[name].focused, true, name + " must own native text focus");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        if (commit) await page.keyboard.press("Enter");
        await frames(page);
    };
    const waitEffects = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const effects = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].effects;
        return new Function("effects", "args", "return (" + source + ")(effects, args)")(effects, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const exactHistory = async (before, after, label) => {
        const committedDepth = (await state(page)).historyLength;
        await done();
        await page.keyboard.press("Control+z");
        await frames(page);
        assert.deepEqual(await documentState(page), before, label + " must undo as one complete document transaction");
        assert.equal((await state(page)).historyLength, committedDepth - 1);
        await page.keyboard.press("Control+Shift+z");
        await frames(page);
        assert.deepEqual(await documentState(page), after, label + " must redo every effect property exactly");
        assert.equal((await state(page)).historyLength, committedDepth);
    };

    const rectangle = fixtureNode(reference, 2, {
        name: "Native effect layer", x: 40, y: 40, width: 220, height: 160,
        corner_radius: 16, effects: [],
    });
    await importFixture(page, isolatedDocument(reference, [rectangle], "Native effect authoring"));
    await clickWorld(page, 130, 110);
    const initial = await documentState(page);
    await clickNative(page, "AddEffectButton", "InspectorViewport");
    await waitEffects((effects) => effects.length === 1 && effects[0].kind === "drop_shadow");
    const added = await documentState(page);
    await exactHistory(initial, added, "Inspector Add effect");
    check("Native inspector Add effect creates a typed drop shadow and restores the exact document with one undo/redo");

    await open();
    await field("EffectRadius0", "12.5");
    for (const [key, expected] of [["ArrowUp", 13.5], ["Shift+ArrowDown", 3.5], ["Alt+ArrowUp", 3.6]]) {
        const beforeStep = await documentState(page);
        await click("EffectRadius0");
        assert.equal((await state(page)).chrome.EffectRadius0.focused, true,
            "Native numeric stepping must start with TextBox keyboard focus");
        await page.keyboard.press(key);
        await waitEffects((effects, expected) => Math.abs(effects[0].radius - expected) < 1e-9, expected);
        const stepped = await documentState(page);
        await exactHistory(beforeStep, stepped, "Effect radius " + key);
        await open();
    }
    await field("EffectRadius0", "12.5");
    await field("EffectSpread0", "3");
    await field("EffectOffsetX0", "-4");
    await field("EffectOffsetY0", "8.25");
    await field("EffectColor0", "#3B82F6");
    const beforeOpacity = await documentState(page);
    await field("EffectOpacity0", "37.5");
    await waitEffects((effects) => effects[0].color.a === 96);
    const authored = await documentState(page);
    assert.equal(authored.nodes[2].effects[0].radius, 12.5);
    assert.equal(authored.nodes[2].effects[0].spread, 3);
    assert.deepEqual(authored.nodes[2].effects[0].offset, { x: -4, y: 8.25 });
    assert.deepEqual(authored.nodes[2].effects[0].color, { r: 59, g: 130, b: 246, a: 96 });
    assert.equal(authored.nodes[2].effects[0].visible, true);
    await exactHistory(beforeOpacity, authored, "Effect opacity field");
    check("Native effect fields author blur/spread, signed offsets, RGB and alpha; Up/Shift/Alt step by 1/10/0.1 ahead of TextBox caret handling, with exact one-step history");

    await open();
    const invalidDepth = (await state(page)).historyLength;
    for (const [name, value] of [
        ["EffectRadius0", "-1"], ["EffectRadius0", "NaN"],
        ["EffectOffsetX0", "Infinity"], ["EffectOpacity0", "101"], ["EffectColor0", "#ZZ0000"],
    ]) {
        await field(name, value);
        assert.deepEqual(await documentState(page), authored, name + " must reject " + value + " without any partial edit");
        assert.equal((await state(page)).historyLength, invalidDepth, "Rejected effect input adds no history");
        await page.keyboard.press("Escape");
        await frames(page);
    }
    await field("EffectRadius0", "72", false);
    assert.deepEqual(await documentState(page), authored, "An uncommitted native field draft is not an effect edit");
    await page.keyboard.press("Escape");
    await click("EffectOffsetY0");
    assert.deepEqual(await documentState(page), authored, "Escape restores the committed field so its later blur cannot apply the canceled draft");
    assert.equal((await state(page)).historyLength, invalidDepth);
    await done();
    check("Native effect fields reject negative radius, nonfinite numbers, invalid alpha and malformed color atomically; Escape cancels drafts without a deferred blur edit");

    await open();
    await click("EffectVisible0");
    await waitEffects((effects) => effects[0].visible === false);
    const hidden = await documentState(page);
    await exactHistory(authored, hidden, "Effect visibility");
    await open();
    await click("EffectVisible0");
    await waitEffects((effects) => effects[0].visible === true);
    await chooseNative(page, "EffectAddType", { index: 2 });
    await clickNative(page, "EffectAdd");
    await waitEffects((effects) => effects.length === 2 && effects[1].kind === "layer_blur");
    await field("EffectRadius1", "6.5");
    const beforeReorder = await documentState(page);
    await click("EffectUp1");
    await waitEffects((effects) => effects[0].kind === "layer_blur" && effects[1].kind === "drop_shadow");
    const reordered = await documentState(page);
    assert.deepEqual(reordered.nodes[2].effects, [...beforeReorder.nodes[2].effects].reverse(),
        "Native Up reorders complete effects, retaining every numeric/color property");
    await captureNativeScreenshot(mode + "-native-effect-stack", {
        controls: ["ModalContent", "EffectType0", "EffectRadius0", "EffectDone"],
    });
    await exactHistory(beforeReorder, reordered, "Effect order");
    check("Native visibility and effect-type controls preserve typed stacks; Up reorders complete effects with exact one-step history and a rendered native dialog");

    await open();
    await click("EffectDuplicate0");
    await waitEffects((effects) => effects.length === 3);
    const duplicated = await documentState(page);
    assert.deepEqual(duplicated.nodes[2].effects.slice(0, 2), [reordered.nodes[2].effects[0], reordered.nodes[2].effects[0]],
        "Native Duplicate inserts an independent complete copy next to its source");
    await field("EffectRadius1", "10");
    await waitEffects((effects) => effects[1].radius === 10);
    const beforeRemove = await documentState(page);
    assert.equal(beforeRemove.nodes[2].effects[0].radius, 6.5, "Editing the copied effect cannot mutate the source");
    await click("EffectRemove1");
    await waitEffects((effects) => effects.length === 2);
    const removed = await documentState(page);
    assert.deepEqual(removed.nodes[2].effects, reordered.nodes[2].effects);
    await exactHistory(beforeRemove, removed, "Effect removal");
    check("Native Duplicate creates an independent typed effect; Remove retains remaining order and exactly restores the removed effect on undo");
}
