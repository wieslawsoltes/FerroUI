import assert from "node:assert/strict";

// Fixture import supplies initial conditions only. Selection, preferences,
// history and gesture cancellation all use the actual native input surface.
export async function selectionRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, frames, waitForNativeFrame, waitForNativeScene, captureNativeScreenshot } = helpers;
    const fill = (r, g, b, a = 255) => ({ paint: { type: "solid", r, g, b, a }, opacity: 1, visible: true });
    const rectangle = (id, properties = {}) => fixtureNode(reference, id, {
        kind: "rectangle", x: 40, y: 40, width: 200, height: 200,
        corner_radius: 0, corner_radii: null, blend_mode: "normal", fills: [fill(60, 120, 220)], ...properties,
    });
    const selected = async (expected) => page.waitForFunction((expected) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return JSON.stringify(view.selection) === JSON.stringify(expected);
    }, expected, { timeout: 10000 });
    const press = async (key, expected) => {
        const before = await state(page);
        await page.keyboard.press(key);
        await selected(expected);
        if (JSON.stringify(before.selection) !== JSON.stringify(expected)) {
            await waitForNativeFrame(page, before.renderedFrames, "Keyboard selection " + key);
        }
    };
    const click = async (point, modifiers, expected, options) => {
        for (const modifier of modifiers) await page.keyboard.down(modifier);
        try { await clickWorld(page, point.x, point.y, options); }
        finally { for (const modifier of [...modifiers].reverse()) await page.keyboard.up(modifier); }
        await selected(expected);
    };
    const action = async (search, command) => {
        await clickNative(page, "ActionsButton");
        await clickNative(page, "ActionSearch", "ModalViewport");
        await page.keyboard.press("Control+a");
        await page.keyboard.insertText(search);
        // TextChanged filtering is dispatched after TextBox.Text changes. A
        // previously arranged row can therefore still have its old bounds.
        // Observe the actual native filtered result before deriving a click.
        await page.waitForFunction(({ search, command }) => {
            const chrome = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome;
            const visible = Object.entries(chrome)
                .filter(([name, bounds]) => name.startsWith("Action-") && bounds.visible);
            return chrome.ActionSearch?.text === search && visible.length === 1 &&
                visible[0][0] === "Action-" + command &&
                ["ModalPanel", "ModalContent", "ModalViewport"].every(name =>
                    chrome[name]?.measureValid && chrome[name]?.arrangeValid);
        }, { search, command });
        // Layout properties can already describe the filtered, recentered
        // modal while native hit testing still uses the previous scene. There
        // can be one older coalesced completion pending; require two completed
        // scenes after the final layout before deriving the click coordinates.
        // The focused native search caret continues to publish scenes.
        const filtered = await state(page);
        await waitForNativeScene(page, filtered.sceneFrames + 1, "Filtered action " + command);
        await clickNative(page, "Action-" + command, "ModalViewport");
        await page.waitForFunction(() =>
            JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ActionSearch?.visible !== true,
        null, { timeout: 10000 });
    };
    const snapshot = async () => {
        const view = await state(page);
        return { document: await documentState(page), history: view.historyLength, redo: view.canRedo };
    };
    const unchanged = async (before, label) => {
        const view = await state(page);
        assert.deepEqual(await documentState(page), before.document, label + " preserves every document field");
        assert.equal(view.historyLength, before.history, label + " preserves undo depth");
        assert.equal(view.canRedo, before.redo, label + " preserves redo");
    };

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Back overlap", fills: [fill(255, 180, 50)] }),
        rectangle(3, { name: "Middle overlap", fills: [fill(40, 180, 100)] }),
        rectangle(4, { name: "Front overlap" }),
        rectangle(5, { name: "Hidden overlap", visible: false }),
        rectangle(6, { name: "Locked overlap", locked: true, opacity: 0.15 }),
        rectangle(7, { name: "Ellipse geometry miss", kind: "ellipse", opacity: 0.3 }),
    ], "Ordered native selection"));
    assert.equal((await state(page)).selectGroups, false, "Existing direct picking remains the default");
    const overlap = { x: 65, y: 65 };
    await click(overlap, [], [4]);
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Control+z");
    await frames(page);
    const beforeCycle = await snapshot();
    assert.equal(beforeCycle.redo, true);
    await click(overlap, ["Alt"], [3]);
    await click(overlap, ["Alt"], [2]);
    await click(overlap, ["Alt"], [4]);
    await click(overlap, ["Alt", "Shift"], [2]);
    await press("Alt+]", [3]);
    await press("Alt+[", [2]);
    await unchanged(beforeCycle, "Overlapping-layer pointer and keyboard cycling");
    check("Native Alt-click and bracket keys cycle exact overlapping geometry in painter order, reverse and wrap, skip locked/hidden/shape misses, and preserve document/undo/redo");

    await action("Select layer under pointer", "select-layer");
    const menu = (await state(page)).chrome;
    assert.ok(menu.SelectLayer6?.visible, "The explicit layer menu includes a locked visible hit");
    assert.ok(menu.SelectLayer4?.visible && menu.SelectLayer3?.visible && menu.SelectLayer2?.visible);
    assert.equal(menu.SelectLayer5, undefined, "Hidden layers are absent from the native list");
    assert.equal(menu.SelectLayer7, undefined, "The native list uses exact ellipse geometry");
    await clickNative(page, "SelectLayer6", "ModalViewport");
    await selected([6]);
    await unchanged(beforeCycle, "Locked-layer inspection from Select Layer");
    await click(overlap, ["Alt"], [4]);
    check("The native Select Layer list exposes locked layers for inspection, excludes hidden and missed shapes, and changes selection without writing history");

    const hierarchy = [
        rectangle(2, { kind: "frame", name: "Hierarchy frame", width: 300, height: 180, fills: [], children: [3, 8] }),
        rectangle(3, { kind: "group", parent: 2, name: "Nested group", x: 20, y: 20, width: 160, height: 110,
            blend_mode: "pass_through", fills: [], children: [4, 5, 6, 7] }),
        rectangle(4, { parent: 3, name: "Back child", x: 10, y: 10, width: 50, height: 50 }),
        rectangle(5, { parent: 3, name: "Front child", x: 90, y: 10, width: 50, height: 50 }),
        rectangle(6, { parent: 3, name: "Hidden child", x: 10, y: 10, width: 50, height: 50, visible: false }),
        rectangle(7, { parent: 3, name: "Locked child", x: 90, y: 10, width: 50, height: 50, locked: true, opacity: 0.2 }),
        rectangle(8, { parent: 2, name: "Frame sibling", x: 220, y: 50, width: 50, height: 50 }),
    ];
    await importFixture(page, isolatedDocument(reference, hierarchy, "Native hierarchy navigation"));
    await click({ x: 48, y: 48 }, [], [2]);
    const beforeNavigation = await snapshot();
    await press("Enter", [8, 3]);
    await press("Tab", [8]);
    await press("Tab", [3]);
    await press("Enter", [5, 4]);
    await press("Tab", [5]);
    await press("Tab", [4]);
    await press("Shift+Tab", [5]);
    await press("Shift+Enter", [3]);
    await press("Shift+Enter", [2]);
    await press("Shift+Enter", [2]);
    await press("Escape", []);
    await press("Tab", [2]);
    await unchanged(beforeNavigation, "Nested keyboard navigation");
    check("Native Enter/Shift+Enter navigate children and parents; Tab/Shift+Tab narrow multi-selection and wrap through eligible siblings, with no document or history mutation");

    await action("Select groups", "toggle-select-groups");
    assert.equal((await state(page)).selectGroups, true, "The native command changes the explicit preference");
    await clickNative(page, "SelectTool");
    await press("Escape", []);
    const child = { x: 95, y: 95 };
    await click(child, [], [2]);
    // A distinct preceding pointer position resets the native double-click
    // count, including consecutive nested descent sequences.
    await click({ x: 48, y: 48 }, [], [2]);
    await click(child, [], [3], { clickCount: 2, delay: 80 });
    await click({ x: 205, y: 155 }, [], [3]);
    await click(child, [], [4], { clickCount: 2, delay: 80 });
    await press("Escape", []);
    await click(child, ["Control"], [4]);
    await press("Escape", []);
    await click({ x: 175, y: 95 }, ["Meta"], [5]);
    await click(child, ["Control", "Shift"], [5, 4]);
    await captureNativeScreenshot(mode + "-native-selection-groups", { controls: ["CanvasHost", "LayersViewport"] });
    await action("Select groups", "toggle-select-groups");
    assert.equal((await state(page)).selectGroups, false);
    await unchanged(beforeNavigation, "Group selection preference and nested deep picking");
    check("The native Select groups toggle enables one-level double-click descent; Ctrl/Command always deep-select and Shift adds nested layers while the preference and selection remain outside undo history");

    const loop = (points) => [
        { type: "move_to", point: { x: points[0][0], y: points[0][1] } },
        ...points.slice(1).map(([x, y]) => ({ type: "line_to", point: { x, y } })),
        { type: "close" },
    ];
    for (const kind of ["alpha", "luminance", "vector"]) {
        const gradientColor = (bright) => kind === "alpha"
            ? { r: 255, g: 255, b: 255, a: bright ? 255 : 0 }
            : { r: bright ? 255 : 0, g: bright ? 255 : 0, b: bright ? 255 : 0, a: 255 };
        const mask = rectangle(10, { name: kind + " selection mask", width: 220, height: 180,
            locked: true, is_mask: true, mask_type: kind,
            fills: [{ opacity: 1, visible: true, paint: { type: "linear_gradient",
                start: { x: 0, y: 0 }, end: { x: 1, y: 0 }, stops: [
                    { offset: 0, color: gradientColor(false) },
                    { offset: 0.35, color: gradientColor(false) },
                    { offset: 0.65, color: gradientColor(true) },
                    { offset: 1, color: gradientColor(true) },
                ] } }],
        });
        if (kind === "vector") {
            mask.kind = "vector";
            mask.fills = [fill(255, 255, 255)];
            mask.path = { fill_rule: "even_odd", segments: [
                ...loop([[0, 0], [220, 0], [220, 180], [0, 180]]),
                ...loop([[80, 40], [140, 40], [140, 140], [80, 140]]),
            ] };
        }
        await importFixture(page, isolatedDocument(reference, [
            rectangle(2, { name: "Unmasked backdrop", width: 220, height: 180, fills: [fill(240, 180, 50)] }),
            mask,
            rectangle(11, { name: "Masked target", width: 220, height: 180 }),
            rectangle(12, { name: "Hidden target", width: 220, height: 180, visible: false }),
            rectangle(13, { name: "Locked target", width: 220, height: 180, locked: true, opacity: 0.1 }),
        ], "Native " + kind + " mask picking"));
        const visible = { x: 235, y: 110 };
        const empty = kind === "vector" ? { x: 150, y: 120 } : { x: 65, y: 110 };
        await click(visible, [], [11]);
        const beforeMask = await snapshot();
        await click(empty, ["Alt"], [2]);
        await click(visible, ["Alt"], [11]);
        await click(visible, ["Alt"], [2]);
        await click(visible, ["Alt"], [11]);
        await unchanged(beforeMask, kind + " mask picking");
        await captureNativeScreenshot(mode + "-native-" + kind + "-mask-selection", { controls: ["CanvasHost"] });
        check("Native " + kind + " mask picking and overlap cycling skip transparent pixels/holes, retain visible target geometry, and preserve document/history");
    }

    const affine = { a: 1, b: 0.2, c: 0.3, d: 1, e: 0, f: 0 };
    const world = (x, y) => ({ x: 60 + x + 0.3 * y, y: 60 + 0.2 * x + y });
    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Clip backdrop", width: 300, height: 220 }),
        rectangle(3, { kind: "frame", name: "Affine rounded clip", x: 60, y: 60, width: 160, height: 140,
            fills: [], corner_radius: 40, clip_content: true, affine_transform: affine, children: [4] }),
        rectangle(4, { parent: 3, name: "Clipped child", x: 0, y: 0, width: 160, height: 140 }),
    ], "Native affine clipping selection"));
    await click(world(50, 50), [], [4]);
    const beforeClip = await snapshot();
    // The selected child's zero-radius marker is inset near local (3, 3).
    // Alt at that real control correctly starts independent radius editing.
    // Probe a different point in the same rounded cutout, explicitly clear of
    // both radius and resize targets, so this assertion exercises layer picking.
    const emptyLocal = { x: 3, y: 20 }, emptyWorld = world(emptyLocal.x, emptyLocal.y);
    assert.ok(emptyLocal.x > 0 && emptyLocal.y > 0 && emptyLocal.x < 40 && emptyLocal.y < 40);
    assert.ok(Math.hypot(emptyLocal.x - 40, emptyLocal.y - 40) > 40,
        "The probe is inside the frame box but outside its exact rounded corner");
    const clipView = await state(page), emptyScreen = {
        x: clipView.pan.x + emptyWorld.x * clipView.zoom,
        y: clipView.pan.y + emptyWorld.y * clipView.zoom,
    };
    assert.equal(clipView.cornerRadius.node, 4); assert.equal(clipView.cornerRadius.handles.length, 4);
    for (const handle of clipView.cornerRadius.handles) {
        assert.ok(Math.hypot(handle.screen.x - emptyScreen.x, handle.screen.y - emptyScreen.y) > 16,
            "The rounded-cutout probe is clear of every native radius hit target");
        assert.ok(Math.hypot(handle.vertexWorld.x - emptyWorld.x, handle.vertexWorld.y - emptyWorld.y) * clipView.zoom > 16,
            "The rounded-cutout probe is clear of every native resize hit target");
    }
    await click(emptyWorld, ["Alt"], [2]);
    await unchanged(beforeClip, "Affine rounded clip picking");
    check("Native picking applies the exact rounded ancestor clip in its affine coordinate system instead of accepting the clip's world bounding box");

    await importFixture(page, isolatedDocument(reference, [
        rectangle(2, { name: "Held drag", x: 40, y: 40, width: 100, height: 100 }),
        rectangle(3, { name: "Keyboard destination", x: 300, y: 40, width: 100, height: 100 }),
    ], "Selection cancels held native move"));
    await click({ x: 90, y: 90 }, [], [2]);
    const beforeHeld = await snapshot(), view = await state(page);
    const screen = (x, y) => ({ x: view.chrome.CanvasHost.x + view.pan.x + x * view.zoom,
        y: view.chrome.CanvasHost.y + view.pan.y + y * view.zoom });
    const start = screen(90, 90), end = screen(110, 100);
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(end.x, end.y, { steps: 8 });
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].x !== 40);
    await press("Tab", [3]);
    await page.mouse.move(end.x + 20, end.y + 10, { steps: 3 });
    await page.mouse.up();
    await selected([3]);
    await unchanged(beforeHeld, "Selection navigation during a held native move");
    check("Tab during a held native move restores its preview before navigating siblings; late pointer motion/release cannot move the new selection or create history");
}
