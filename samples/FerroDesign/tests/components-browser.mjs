import assert from "node:assert/strict";

// Fixture import supplies authored source trees. Every mutation asserted below
// comes from real FerroUI controls, pointer selection, and native key handling.
export async function componentAuthoringRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state, clickWorld, clickNative, chooseNative, frames, captureNativeScreenshot } = helpers;
    const click = (name) => clickNative(page, name, "ModalViewport");
    const open = async () => {
        const selection = (await state(page)).selection;
        await clickNative(page, "EditComponentsButton", "InspectorViewport");
        await page.waitForFunction(() => {
            const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
            return chrome.ModalBackdrop?.visible === true && chrome.ComponentDone?.visible === true;
        }, undefined, { timeout: 10000 });
        assert.deepEqual((await state(page)).selection, selection, "Opening component properties preserves the layer selection");
    };
    const done = () => clickNative(page, "ComponentDone");
    const field = async (name, value, commit = true) => {
        await click(name);
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(value);
        if (commit) await page.keyboard.press("Enter");
        await frames(page);
    };
    const next = (name, count = 1) => chooseNative(page, name, { offset: count, scrollContainer: "ModalViewport" });
    const waitDocument = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const doc = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("doc", "args", "return (" + source + ")(doc, args)")(doc, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const value = (type, value) => ({ type, value });
    const definitions = (label, icon, text) => ({
        icon: { name: "Icon", default_value: value("instance_swap", 20), targets: [icon] },
        show: { name: "Show label", default_value: value("boolean", true), targets: [label] },
        title: { name: "Title", default_value: value("text", text), targets: [label] },
    });
    const label = (id, parent, text, component_source = null, properties = {}) => fixtureNode(reference, id, {
        parent, kind: "text", name: "Label", x: 12, y: 14, width: 140, height: 30,
        text, text_runs: [], font_family: "Inter", font_size: 18, font_weight: 400,
        text_auto_resize: "fixed", component_source, ...properties,
    });
    const mark = (id, parent, text, source = null) => label(id, parent, text, source, {
        name: "Mark", x: 2, y: 2, width: 30, height: 30, font_size: 18,
    });
    const icon = (id, parent, child, source, properties = {}) => fixtureNode(reference, id, {
        parent, kind: "instance", name: "Icon", x: 160, y: 14, width: 32, height: 32,
        children: [child], component_id: 20, component_source: source, ...properties,
    });
    const nodes = [
        fixtureNode(reference, 2, { kind: "frame", name: "Button set", x: 40, y: 40, width: 470, height: 140,
            children: [3, 6], metadata: { component_set: "true" }, fills: [] }),
        fixtureNode(reference, 3, { parent: 2, kind: "component", name: "Default", x: 16, y: 20, width: 200, height: 100,
            children: [4, 5], variant_properties: { State: "Default" }, component_property_definitions: definitions(4, 5, "Default title") }),
        label(4, 3, "Default title"), icon(5, 3, 24, 20), mark(24, 5, "○", 21),
        fixtureNode(reference, 6, { parent: 2, kind: "component", name: "Hover", x: 248, y: 20, width: 200, height: 100,
            children: [7, 8], variant_properties: { State: "Hover" }, component_property_definitions: definitions(7, 8, "Hover title") }),
        label(7, 6, "Hover title"), icon(8, 6, 25, 20), mark(25, 8, "○", 21),
        fixtureNode(reference, 10, { kind: "instance", name: "Button instance", x: 40, y: 240, width: 200, height: 100,
            children: [11, 12], component_id: 3, component_source: 3, variant_properties: { State: "Default" } }),
        label(11, 10, "Manual layer override", 4, { overridden_properties: ["text", "visible"], visible: false }),
        icon(12, 10, 13, 5, { component_id: 22, overridden_properties: ["component_id"] }), mark(13, 12, "□", 23),
        fixtureNode(reference, 20, { kind: "component", name: "Circle icon", x: 540, y: 40, width: 32, height: 32, children: [21] }),
        mark(21, 20, "○"),
        fixtureNode(reference, 22, { kind: "component", name: "Square icon", x: 600, y: 40, width: 32, height: 32, children: [23] }),
        mark(23, 22, "□"),
    ];
    await importFixture(page, isolatedDocument(reference, nodes, "Native component variants"));
    await clickWorld(page, 110, 315);
    assert.deepEqual((await state(page)).selection, [10]);
    const beforeVariant = await documentState(page);
    await open();
    await next("ComponentVariant");
    await waitDocument((doc) => doc.nodes[10].component_id === 6);
    const hover = await documentState(page);
    assert.deepEqual(hover.nodes[10].children, [11, 12]);
    assert.deepEqual(hover.nodes[12].children, [13]);
    assert.equal(hover.nodes[11].text, "Manual layer override");
    assert.equal(hover.nodes[11].component_source, 7);
    assert.equal(hover.nodes[12].component_source, 8);
    assert.equal(hover.nodes[10].x, 40);
    assert.equal(hover.nodes[10].y, 240);
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeVariant, "One native undo restores the complete previous variant and materialized identities");
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), hover);
    check("Native variant selection preserves descendant IDs, text overrides and placement with exact one-step undo and redo");

    await open();
    // Append to the displayed value without replacing it. This proves the
    // native TextBox contains the effective direct layer override, not a default.
    await click("ComponentValue2");
    await page.keyboard.press("Control+End");
    await page.keyboard.insertText(" · edited");
    await page.keyboard.press("Enter");
    await waitDocument((doc) => doc.nodes[11].text === "Manual layer override · edited");
    // The effective direct visibility is false and the direct swap is Square.
    // Native toggle/previous-item input must therefore produce true / Circle.
    await click("ComponentValue1");
    await waitDocument((doc) => doc.nodes[11].visible === true && doc.nodes[10].component_property_values.show.value === true);
    await click("ComponentValue0");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Enter");
    await waitDocument((doc) => doc.nodes[12].component_id === 20 && doc.nodes[10].component_property_values.icon.value === 20);
    await done();
    for (let index = 0; index < 3; index++) await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), hover);
    await open();
    await click("ComponentReset2");
    await waitDocument((doc) => doc.nodes[11].text === "Hover title" && !doc.nodes[11].overridden_properties.includes("text"));
    await click("ComponentReset1");
    await waitDocument((doc) => doc.nodes[11].visible === true && !doc.nodes[11].overridden_properties.includes("visible"));
    await click("ComponentReset0");
    await waitDocument((doc) => doc.nodes[12].component_id === 20 && !doc.nodes[12].overridden_properties.includes("component_id"));
    const directReset = await documentState(page);
    assert.deepEqual(directReset.nodes[10].children, [11, 12]);
    assert.deepEqual(directReset.nodes[12].children, [13]);
    await done();
    for (let index = 0; index < 3; index++) await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), hover, "Resetting direct text, visibility and swap overrides remains independently reversible");
    for (let index = 0; index < 3; index++) await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), directReset);
    check("Native exposed fields display effective direct text, visibility and component swaps; editing and Reset clear only the corresponding direct override with exact history");

    await open();
    const unicode = "東京 · café 👩🏽‍💻";
    await field("ComponentValue2", unicode);
    await waitDocument((doc, text) => doc.nodes[11].text === text && doc.nodes[10].component_property_values.title.value === text, unicode);
    await click("ComponentValue1");
    await waitDocument((doc) => doc.nodes[11].visible === false && doc.nodes[10].component_property_values.show.value === false);
    await next("ComponentValue0");
    await waitDocument((doc) => doc.nodes[12].component_id === 22);
    const overridden = await documentState(page);
    assert.deepEqual(overridden.nodes[10].component_property_values, {
        icon: value("instance_swap", 22), show: value("boolean", false), title: value("text", unicode),
    });
    assert.equal(overridden.nodes[7].text, "Hover title", "Instance text does not change the main component");
    assert.equal(overridden.nodes[13].text, "□");
    assert.equal(overridden.nodes[12].component_source, 8, "A nested swap retains its identity in the outer main component");
    assert.deepEqual(overridden.nodes[12].children, [13]);
    await captureNativeScreenshot(mode + "-component-instance-properties", {
        controls: ["ModalContent", "ComponentValue0", "ComponentDone"],
    });
    await click("ComponentReset2");
    await waitDocument((doc) => doc.nodes[11].text === "Hover title");
    await click("ComponentReset1");
    await waitDocument((doc) => doc.nodes[11].visible === true);
    const beforeSwapReset = await documentState(page);
    await click("ComponentReset0");
    await waitDocument((doc) => doc.nodes[12].component_id === 20);
    const reset = await documentState(page);
    assert.deepEqual(reset.nodes[10].component_property_values ?? {}, {});
    assert.equal(reset.nodes[13].text, "○");
    assert.deepEqual(reset.nodes[12].children, [13]);
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeSwapReset, "Resetting an instance-swap property and its subtree is one transaction");
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), reset);
    check("Native text, boolean and instance-swap controls apply typed Unicode overrides, preserve nested IDs and restore defaults atomically");

    await open();
    await clickNative(page, "ComponentEditMaster");
    await field("ComponentDefault2", "Updated default Ω");
    await waitDocument((doc) => doc.nodes[6].component_property_definitions.title.default_value.value === "Updated default Ω"
        && doc.nodes[7].text === "Updated default Ω" && doc.nodes[11].text === "Updated default Ω");
    await field("ComponentPropertyName2", "Button label");
    await waitDocument((doc) => doc.nodes[6].component_property_definitions.title.name === "Button label");
    let authored = await documentState(page);
    assert.equal(authored.nodes[6].component_property_definitions.title.default_value.value, "Updated default Ω", "Renaming uses the latest definition and must not restore an old default");
    assert.equal(authored.nodes[4].text, "Default title");
    await next("ComponentNewPropertyType");
    await field("ComponentNewPropertyName", "Show icon", false);
    await next("ComponentNewPropertyTarget", 2);
    const beforeDefinition = await documentState(page);
    await click("ComponentAddProperty");
    await waitDocument((doc) => doc.nodes[6].component_property_definitions.property1?.name === "Show icon");
    authored = await documentState(page);
    assert.deepEqual(authored.nodes[6].component_property_definitions.property1, {
        name: "Show icon", default_value: value("boolean", true), targets: [8],
    });
    await captureNativeScreenshot(mode + "-component-main-properties", {
        controls: ["ModalContent", "ComponentAddProperty", "ComponentDone"],
    });
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeDefinition);
    await page.keyboard.press("Control+Shift+z");
    await frames(page);
    assert.deepEqual(await documentState(page), authored);
    check("Native main-component fields update instance defaults, rename a stable property without reverting its value, and create an exposed visibility binding with one-step history");

    const first = fixtureNode(reference, 2, { kind: "component", name: "Primary", x: 40, y: 40, width: 180, height: 100, children: [3],
        component_property_definitions: { title: { name: "Title", default_value: value("text", "Primary"), targets: [3] } } });
    const second = fixtureNode(reference, 4, { kind: "component", name: "Secondary", x: 300, y: 40, width: 180, height: 100, children: [5] });
    await importFixture(page, isolatedDocument(reference, [first, label(3, 2, "Primary"), second, label(5, 4, "Secondary")], "Native component set creation"));
    await clickWorld(page, 110, 115);
    await page.keyboard.down("Shift");
    await clickWorld(page, 370, 115);
    await page.keyboard.up("Shift");
    assert.deepEqual(new Set((await state(page)).selection), new Set([2, 4]));
    await open();
    await field("ComponentSetName", "Browser button variants", false);
    const beforeSet = await documentState(page);
    await click("ComponentCombine");
    await waitDocument((doc) => Object.values(doc.nodes).some((node) => node.metadata.component_set === "true"));
    const combined = await documentState(page);
    const set = Object.values(combined.nodes).find((node) => node.metadata.component_set === "true");
    assert.equal(set.name, "Browser button variants");
    assert.deepEqual(set.children, [2, 4]);
    for (const id of [2, 4]) {
        assert.equal(set.x + combined.nodes[id].x, beforeSet.nodes[id].x);
        assert.equal(set.y + combined.nodes[id].y, beforeSet.nodes[id].y);
    }
    await clickNative(page, "ComponentAddVariant");
    await waitDocument((doc, id) => doc.nodes[id].children.length === 3, set.id);
    const copied = await documentState(page);
    const addedId = copied.nodes[set.id].children.find((id) => id !== 2 && id !== 4);
    const added = copied.nodes[addedId];
    assert.equal(added.kind, "component");
    assert.notDeepEqual(added.variant_properties, copied.nodes[2].variant_properties);
    assert.equal(added.component_property_definitions.title.targets[0], added.children[0]);
    assert.notEqual(added.children[0], 3);
    await captureNativeScreenshot(mode + "-component-set-authoring", {
        controls: ["ModalContent", "ComponentVariant", "ComponentAddVariant", "ComponentDone"],
    });
    await done();
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), combined);
    await page.keyboard.press("Control+z");
    await frames(page);
    assert.deepEqual(await documentState(page), beforeSet);
    check("Native multi-selection combines stable source IDs into a component set and adds a variant with remapped bindings while preserving original placement and transaction boundaries");
}
