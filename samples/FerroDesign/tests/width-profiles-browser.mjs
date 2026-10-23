import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

// Fixtures establish geometry only. Every profile/point edit below uses actual
// native controls; the DOM canvas at the end independently decodes an SVG file.
export async function widthProfileRegressions(page, reference, check, helpers, output, mode) {
    const { fixtureNode, isolatedDocument, importFixture, documentState, state,
        clickWorld, clickNative, chooseNative, frames, waitForNativeFrame,
        waitForNativeScene, captureNativeScreenshot } = helpers;
    const wait = (predicate, args) => page.waitForFunction(({ source, args }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        const document = JSON.parse(globalThis.ferroDesign.ferroDesignExportJson());
        return new Function("view", "document", "args", "return (" + source + ")(view,document,args)")(view, document, args);
    }, { source: predicate.toString(), args }, { timeout: 10000 });
    const profile = (doc, index) => doc.nodes[3].strokes[index].width_profile ?? { type: "uniform" };
    const custom = points => ({ type: "custom", points: points.map(([position, width]) => ({ position, width })) });
    const unchanged = async (doc, depth, label) => {
        assert.deepEqual(await documentState(page), doc, label + " preserves the complete document");
        assert.equal((await state(page)).historyLength, depth, label + " preserves history");
    };
    const focusCanvas = async () => {
        await clickNative(page, "SelectTool");
        await wait(view => view.chrome.CanvasHost.focusWithin);
        assert.deepEqual((await state(page)).selection, [3]);
    };
    const history = async (before, after, depth, label) => {
        assert.equal((await state(page)).historyLength, depth + 1, label + " commits once");
        await focusCanvas();
        let view = await state(page); await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " undo");
        await waitForNativeScene(page, view.sceneFrames, label + " undo");
        await unchanged(before, depth, label + " exact undo");
        view = await state(page); await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, label + " redo");
        await waitForNativeScene(page, view.sceneFrames, label + " redo");
        await unchanged(after, depth + 1, label + " exact redo");
    };
    const editProfile = async (index, expectedProfile, action, label) => {
        const before = await documentState(page), view = await state(page);
        const expected = structuredClone(before), stroke = expected.nodes[3].strokes[index];
        if (expectedProfile.type === "uniform") delete stroke.width_profile;
        else stroke.width_profile = expectedProfile;
        if (expectedProfile.type !== "uniform" && stroke.cap === "arrow") stroke.cap = "butt";
        expected.nodes[3].overridden_properties = [...new Set([...expected.nodes[3].overridden_properties, "strokes"])].sort();
        await action();
        await wait((view, doc, args) => view.historyLength === args.depth + 1 &&
            JSON.stringify(doc.nodes[3].strokes[args.index].width_profile ?? { type: "uniform" }) === JSON.stringify(args.profile),
        { index, profile: expectedProfile, depth: view.historyLength });
        await waitForNativeFrame(page, view.renderedFrames, label);
        await waitForNativeScene(page, view.sceneFrames, label);
        const after = await documentState(page);
        assert.deepEqual(after, expected, label + " changes only the chosen profile, required cap and stroke override");
        return { before, after, depth: view.historyLength, label };
    };
    const draft = async (name, text, scrollContainer = "ModalViewport") => {
        await clickNative(page, name, scrollContainer);
        assert.equal((await state(page)).chrome[name].focused, true, name + " owns native keyboard input");
        await page.keyboard.press("Control+a"); await page.keyboard.insertText(text);
        await wait((view, _doc, args) => view.chrome[args.name].text === args.text, { name, text });
    };
    const restore = async (name, text, scrollContainer = "ModalViewport") => {
        await clickNative(page, name, scrollContainer);
        assert.equal((await state(page)).chrome[name].focused, true);
        await page.keyboard.press("Escape");
        await wait((view, _doc, args) => view.chrome[args.name].text === args.text, { name, text });
        await frames(page);
    };
    const openPoints = async () => {
        const before = await state(page);
        await clickNative(page, "StrokeWidthCustomButton", "InspectorViewport");
        await wait(view => view.chrome.ModalBackdrop.visible && view.chrome.ModalTitle.text === "Stroke width" &&
            view.chrome.StrokeWidthPosition1?.visible && view.chrome.StrokeWidthPosition1.arrangeValid &&
            view.chrome.StrokeProfileStackChoice.choice.selectedIndex === 0);
        await waitForNativeScene(page, before.sceneFrames, "Open native width points");
    };
    const closePoints = async () => {
        const before = await state(page);
        await clickNative(page, "StrokeWidthDone", "ModalActions");
        await wait(view => !view.chrome.ModalBackdrop.visible && view.chrome.CanvasHost.focusWithin);
        await waitForNativeScene(page, before.sceneFrames, "Close native width points");
    };
    // The shared successful-choice helper correctly requires the requested
    // index to commit. A rejected draft instead restores the previous index;
    // still prove a real popup, focused requested item, Enter and popup closure.
    const rejectedChoice = async (name, target, scrollContainer) => {
        const before = (await state(page)).chrome[name].choice;
        assert.equal(before.open, false); assert.notEqual(before.selectedIndex, target);
        const expect = expected => wait((view, _doc, args) => {
            const choice = view.chrome[args.name]?.choice;
            return choice && Object.entries(args.expected).every(([key, value]) => choice[key] === value);
        }, { name, expected });
        await clickNative(page, name, scrollContainer);
        await expect({ open: true, focusedIndex: before.selectedIndex });
        await page.keyboard.press("Home"); await expect({ open: true, focusedIndex: 0 });
        for (let index = 1; index <= target; index++) {
            await page.keyboard.press("ArrowDown"); await expect({ open: true, focusedIndex: index });
        }
        await page.keyboard.press("Enter");
        await expect({ open: false, selectedIndex: before.selectedIndex });
        await frames(page);
    };
    const red = { type: "solid", r: 239, g: 68, b: 68, a: 255 };
    const blue = { type: "solid", r: 37, g: 99, b: 235, a: 255 };
    const stroke = (paint, width, cap) => ({ paint, width, cap, opacity: 1, visible: true,
        align: "center", join: "miter", dash: [] });
    await importFixture(page, isolatedDocument(reference, [
        fixtureNode(reference, 2, { kind: "frame", name: "Stroke export viewport", x: 40, y: 40,
            width: 360, height: 200, children: [3], fills: [], strokes: [], corner_radius: 0, corner_radii: null }),
        fixtureNode(reference, 3, { kind: "line", parent: 2, name: "Two editable width profiles", x: 60, y: 100,
            width: 240, height: 0, fills: [], strokes: [stroke(red, 20, "arrow"), stroke(blue, 40, "round")],
            corner_radius: 0, corner_radii: null, vector_network: null }),
    ], "Native stroke width profiles"));
    await clickNative(page, "SelectTool");
    await page.keyboard.down("Control");
    try { await clickWorld(page, 220, 140); } finally { await page.keyboard.up("Control"); }
    assert.deepEqual((await state(page)).selection, [3]);
    assert.equal((await state(page)).chrome.CanvasHost.focusWithin, true);

    for (const [index, type] of [[1, "wedge"], [2, "taper"], [3, "quarter_taper"],
        [4, "eye"], [5, "mirrored_taper"], [0, "uniform"]]) {
        const edit = await editProfile(0, { type }, () => chooseNative(page, "StrokeProfileChoice",
            { index, scrollContainer: "InspectorViewport" }), "Native " + type + " preset");
        await history(edit.before, edit.after, edit.depth, edit.label);
    }
    let before = await documentState(page), depth = (await state(page)).historyLength;
    await chooseNative(page, "StrokeProfileChoice", { index: 0, scrollContainer: "InspectorViewport" });
    await unchanged(before, depth, "Choosing the current preset");
    await captureNativeScreenshot(mode + "-width-profile-presets", { controls: ["StrokeProfileChoice", "StrokeWidthCustomButton"] });
    check("All six native width presets preserve the other stroke and source geometry, convert an arrow cap once, round-trip exact undo/redo, and leave an unchanged choice out of history");

    const invalidNumber = "Stroke width must be a finite nonnegative number, at most 10,000,000.";
    let view = await state(page);
    assert.ok(!view.chrome.Toast.visible || view.chrome.ToastText.text !== invalidNumber, "Base-width rejection must be fresh");
    await draft("StrokeWidthField", "invalid-width", "InspectorViewport");
    await rejectedChoice("StrokeProfileChoice", 1, "InspectorViewport");
    await wait((view, _doc, message) => view.chrome.Toast.visible && view.chrome.ToastText.text === message, invalidNumber);
    assert.equal((await state(page)).chrome.StrokeWidthField.text, "invalid-width");
    await unchanged(before, depth, "Invalid base width followed by a preset choice");
    await restore("StrokeWidthField", "20", "InspectorViewport");
    await unchanged(before, depth, "Escape restores the base-width draft");

    await openPoints();
    assert.equal((await state(page)).chrome.StrokeProfileStackChoice.choice.itemCount, 2);
    await unchanged(before, depth, "Opening custom points");
    const invalidPoints = "Stroke width points need increasing positions in [0, 1] and finite nonnegative widths.";
    assert.equal((await state(page)).chrome.StrokeProfileError.visible, false);
    await draft("StrokeWidthPosition1", "0");
    await clickNative(page, "StrokeWidthAddPoint", "ModalActions");
    await wait((view, _doc, message) => view.chrome.StrokeProfileError.visible &&
        view.chrome.StrokeProfileError.text === message && view.chrome.CanvasHost.focusWithin, invalidPoints);
    view = await state(page);
    assert.equal(view.chrome.StrokeWidthPosition1.text, "0");
    assert.ok(!view.chrome.StrokeWidthPosition2?.visible, "Rejected Add retains the original two fields");
    assert.equal(view.chrome.StrokeProfileStackChoice.choice.selectedIndex, 0);
    await unchanged(before, depth, "Invalid position followed by Add point");
    await captureNativeScreenshot(mode + "-width-profile-invalid-draft", {
        controls: ["StrokeProfileError", "StrokeWidthPosition1", "StrokeWidthAddPoint"] });
    await restore("StrokeWidthPosition1", "100");
    assert.equal((await state(page)).chrome.StrokeProfileError.visible, false);

    await draft("StrokeWidthPosition1", "0");
    await rejectedChoice("StrokeProfileStackChoice", 1, "ModalViewport");
    await wait((view, _doc, message) => view.chrome.StrokeProfileError.visible && view.chrome.StrokeProfileError.text === message, invalidPoints);
    assert.equal((await state(page)).chrome.StrokeWidthPosition1.text, "0");
    await unchanged(before, depth, "Invalid position followed by a stroke-stack change");
    await restore("StrokeWidthPosition1", "100");
    assert.equal((await state(page)).chrome.StrokeProfileError.visible, false);
    await draft("StrokeWidthValue0", "-5"); await page.keyboard.press("Enter");
    await wait(view => view.chrome.StrokeProfileError.visible &&
        view.chrome.StrokeProfileError.text === "Width must be a finite nonnegative percentage.");
    await unchanged(before, depth, "Negative custom width");
    await restore("StrokeWidthValue0", "100");
    assert.equal((await state(page)).chrome.StrokeProfileError.visible, false);
    check("Native invalid base-width and duplicate-point drafts block preset, Add and stack changes without losing text or history; fresh errors are observed and field Escape restores the committed values");

    const edits = [];
    const pointEdit = async (index, name, value, points, label) => {
        const edit = await editProfile(index, custom(points), async () => {
            await draft(name, value); await page.keyboard.press("Enter");
        }, label);
        assert.equal((await state(page)).chrome.StrokeProfileError.visible, false);
        edits.push(edit);
    };
    // The successful Add after Escape also proves that the rejected action did
    // not disable the editor or silently discard a pending field binding.
    edits.push(await editProfile(0, custom([[0, 1], [0.5, 1], [1, 1]]),
        () => clickNative(page, "StrokeWidthAddPoint", "ModalActions"), "Add width point after Escape"));
    await wait(view => view.chrome.StrokeWidthPosition2?.visible && view.chrome.StrokeWidthPosition2.arrangeValid);
    edits.push(await editProfile(0, custom([[0, 1], [1, 1]]),
        () => clickNative(page, "StrokeWidthRemove1", "ModalViewport"), "Remove native width point"));
    await wait(view => !view.chrome.StrokeWidthPosition2?.visible && view.chrome.StrokeWidthPosition1?.arrangeValid);
    await pointEdit(0, "StrokeWidthValue0", "25", [[0, 0.25], [1, 1]], "Top stroke start width");
    await pointEdit(0, "StrokeWidthValue1", "175", [[0, 0.25], [1, 1.75]], "Top stroke end width");
    await pointEdit(0, "StrokeWidthPosition1", "80", [[0, 0.25], [0.8, 1.75]], "Top stroke width position");
    let publication = await state(page);
    await chooseNative(page, "StrokeProfileStackChoice", { index: 1, scrollContainer: "ModalViewport" });
    await wait(view => view.chrome.StrokeWidthValue0?.text === "100" && view.chrome.StrokeWidthPosition1?.text === "100");
    await waitForNativeScene(page, publication.sceneFrames, "Switch the native stroke stack");
    await pointEdit(1, "StrokeWidthValue0", "175", [[0, 1.75], [1, 1]], "Lower stroke start width");
    await pointEdit(1, "StrokeWidthValue1", "25", [[0, 1.75], [1, 0.25]], "Lower stroke end width");
    await captureNativeScreenshot(mode + "-width-profile-custom-stack", {
        controls: ["StrokeProfileStackChoice", "StrokeWidthPosition1", "StrokeWidthValue0", "StrokeWidthValue1", "StrokeWidthDone"] });
    await closePoints();
    await focusCanvas();
    for (const edit of [...edits].reverse()) {
        view = await state(page); await page.keyboard.press("Control+z");
        await waitForNativeFrame(page, view.renderedFrames, edit.label + " undo");
        await waitForNativeScene(page, view.sceneFrames, edit.label + " undo");
        await unchanged(edit.before, edit.depth, edit.label + " exact undo");
    }
    for (const edit of edits) {
        view = await state(page); await page.keyboard.press("Control+Shift+z");
        await waitForNativeFrame(page, view.renderedFrames, edit.label + " redo");
        await waitForNativeScene(page, view.sceneFrames, edit.label + " redo");
        await unchanged(edit.after, edit.depth + 1, edit.label + " exact redo");
    }
    const authored = await documentState(page);
    assert.deepEqual(profile(authored, 0), custom([[0, 0.25], [0.8, 1.75]]));
    assert.deepEqual(profile(authored, 1), custom([[0, 1.75], [1, 0.25]]));
    check("Native ordered point fields author positions and multipliers in both stroke-stack entries; Add/Remove and each field commit preserve untouched properties and restore every exact intermediate document through undo/redo");

    await clickWorld(page, 60, 60);
    assert.deepEqual((await state(page)).selection, [2]);
    assert.equal((await state(page)).unsupportedEffects, 0);
    assert.equal((await state(page)).invalidNodes, 0);
    await captureNativeScreenshot(mode + "-width-profile-artwork", { controls: ["CanvasHost"] });
    const download = async (button, suffix) => {
        const [item] = await Promise.all([page.waitForEvent("download", { timeout: 60000 }),
            clickNative(page, button, "InspectorViewport")]);
        assert.equal(await item.failure(), null);
        const path = resolve(output, mode + "-width-profiles." + suffix);
        await item.saveAs(path);
        return readFile(path);
    };
    const native = PNG.sync.read(await download("ExportPngButton", "png"));
    const svg = (await download("ExportSvgButton", "svg")).toString("utf8");
    assert.deepEqual([native.width, native.height], [360, 200]);
    const samples = [
        { x: 90, y: 100, rgba: [239, 68, 68, 255], label: "top stroke over lower stroke" },
        { x: 90, y: 115, rgba: [37, 99, 235, 255], label: "wide lower stroke outside thin top" },
        { x: 90, y: 135, alpha: 0, label: "outside lower taper" },
        { x: 270, y: 110, rgba: [239, 68, 68, 255], label: "top width extends after its last knot" },
        { x: 270, y: 119, alpha: 0, label: "outside extended top width" },
        { x: 45, y: 100, rgba: [37, 99, 235, 255], label: "round lower start cap" },
        { x: 45, y: 125, rgba: [37, 99, 235, 255], label: "round lower cap flank" },
        { x: 30, y: 130, alpha: 0, label: "outside round cap" },
        { x: 0, y: 0, alpha: 0, label: "transparent export exterior" },
    ];
    const external = await page.evaluate(async ({ source, samples }) => {
        const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
        const outlines = [...parsed.querySelectorAll('path[stroke="none"]')].map(path => ({
            fill: path.getAttribute("fill"), rule: path.getAttribute("fill-rule"), d: path.getAttribute("d"),
        }));
        const url = URL.createObjectURL(new Blob([source], { type: "image/svg+xml" }));
        try {
            const image = new Image(); image.src = url; await image.decode();
            const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
            const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
            return { width: canvas.width, height: canvas.height, outlines,
                pixels: samples.map(({ x, y }) => Array.from(context.getImageData(x, y, 1, 1).data)) };
        } finally { URL.revokeObjectURL(url); }
    }, { source: svg, samples });
    assert.deepEqual([external.width, external.height], [360, 200]);
    for (const color of ["#EF4444", "#2563EB"]) {
        assert.ok(external.outlines.some(path => path.fill?.toUpperCase() === color && path.rule === "nonzero" && path.d.length > 20),
            color + " exports a filled variable-width outline");
    }
    for (const [index, sample] of samples.entries()) {
        const offset = (sample.y * native.width + sample.x) * 4;
        const nativePixel = Array.from(native.data.subarray(offset, offset + 4));
        if (sample.rgba) {
            assert.deepEqual(nativePixel, sample.rgba, "FerroUI PNG: " + sample.label);
            assert.deepEqual(external.pixels[index], sample.rgba, "Independent SVG: " + sample.label);
        } else {
            assert.equal(nativePixel[3], sample.alpha, "FerroUI PNG: " + sample.label);
            assert.equal(external.pixels[index][3], sample.alpha, "Independent SVG: " + sample.label);
        }
    }
    await unchanged(authored, edits.at(-1).depth + 1, "Native PNG and SVG export");
    check("Native FerroUI PNG and independently decoded SVG agree at exact opaque and transparent width/cap/stack samples; both SVG strokes are filled outlines and exports preserve the editable document");

    await importFixture(page, authored);
    assert.deepEqual(await documentState(page), authored, "Native JSON import retains every ordered width point and stroke-stack property");
    assert.deepEqual(profile(await documentState(page), 0), custom([[0, 0.25], [0.8, 1.75]]));
    assert.deepEqual(profile(await documentState(page), 1), custom([[0, 1.75], [1, 0.25]]));
    check("Native JSON serialization and import preserve both custom profiles, source geometry, caps, paints, point order and stroke metadata without converting the artwork");
}
