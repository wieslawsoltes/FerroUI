import { paragraphRegressions } from "./paragraphs-browser.mjs";
import { widthProfileRegressions } from "./width-profiles-browser.mjs";
import { widthHandleRegressions } from "./width-handles-browser.mjs";
import { gradientHandleRegressions } from "./gradient-handles-browser.mjs";
import { cornerRegressions } from "./corners-browser.mjs";
import { polygonCornerRegressions } from "./polygon-corners-browser.mjs";
import { cornerRadiusHandleRegressions } from "./corner-radius-handles-browser.mjs";
import { prototypeMotionRegressions } from "./prototype-browser.mjs";
import { nativeAuthoringRegressions } from "./authoring-browser.mjs";
import { typographyRegressions } from "./typography-browser.mjs";
import { vectorNetworkRegressions } from "./network-browser.mjs";
import { componentAuthoringRegressions } from "./components-browser.mjs";
import { imageCropRegressions } from "./image-crop-browser.mjs";
import { compositingRegressions } from "./compositing-browser.mjs";
import { svgCompositingRegressions } from "./svg-compositing-browser.mjs";
import { maskAuthoringRegressions } from "./masks-browser.mjs";
import { pageInspectorRegressions } from "./page-inspector-browser.mjs";
import { layoutAuthoringRegressions } from "./layout-browser.mjs";
import { layoutPaddingRegressions } from "./layout-padding-browser.mjs";
import { layoutVariableRegressions } from "./layout-variables-browser.mjs";
import { layoutDragRegressions } from "./layout-drag-browser.mjs";
import { effectAuthoringRegressions } from "./effects-browser.mjs";
import { inspectorLifecycleRegressions } from "./inspector-lifecycle-browser.mjs";
import { floatingPanelRegressions } from "./panels-browser.mjs";
import { layoutTransferRegressions } from "./layout-transfer-browser.mjs";
import { selectionRegressions } from "./selection-browser.mjs";
import { matchingRegressions } from "./matching-browser.mjs";
import { geometryAuthoringRegressions } from "./geometry-authoring-browser.mjs";
import { flattenRegressions } from "./flatten-browser.mjs";
import { angularPaintRegressions } from "./angular-paints-browser.mjs";
import { localStyleRegressions } from "./styles-browser.mjs";
import { gridStyleRegressions } from "./grid-styles-browser.mjs";
import { guideBindingRegressions } from "./guide-bindings-browser.mjs";
import { regionPaintRegressions } from "./region-paints-browser.mjs";
import { compactAutosaveRegressions } from "./serialization-browser.mjs";
import { boundedHistoryRegressions } from "./history-browser.mjs";
import { networkBucketRegressions } from "./network-bucket-browser.mjs";
import { captureWithinDeadline, runIsolatedScenario } from "./isolated-scenario.mjs";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { readFile, writeFile, mkdir, stat } from "node:fs/promises";
import { resolve, dirname, extname, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { PNG } from "pngjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = resolve(process.env.FERRODESIGN_DIST || resolve(root, "dist"));
const output = resolve(process.env.FERRODESIGN_REPORT || resolve(root, "browser-report"));
const selectedScenarios = process.env.FERRODESIGN_SCENARIOS
    ? new Set(process.env.FERRODESIGN_SCENARIOS.split(",").map(value => value.trim()).filter(Boolean)) : null;
const selectedModes = process.env.FERRODESIGN_MODES
    ? [...new Set(process.env.FERRODESIGN_MODES.split(",").map(value => value.trim()).filter(Boolean))]
    : ["Software2D", "WebGL2"];
assert.ok(selectedModes.length && selectedModes.every(mode => ["Software2D", "WebGL2"].includes(mode)),
    "FERRODESIGN_MODES accepts Software2D and WebGL2 only");
assert.ok(!selectedScenarios || selectedScenarios.size, "FERRODESIGN_SCENARIOS cannot be empty");
await mkdir(output, { recursive: true });
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png", ".svg": "image/svg+xml" };
const server = createServer(async (request, response) => {
    try {
        const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
        let file = resolve(dist, "." + pathname);
        if (file !== dist && !file.startsWith(dist + sep)) { response.writeHead(403).end(); return; }
        if ((await stat(file)).isDirectory()) file = resolve(file, "index.html");
        response.writeHead(200, { "content-type": types[extname(file)] || "application/octet-stream", "cache-control": "no-store" });
        response.end(await readFile(file));
    } catch { response.writeHead(404).end("Not found"); }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const base = "http://127.0.0.1:" + server.address().port;
const report = {
    created: new Date().toISOString(), browser: "", results: [],
    scope: { full: !selectedScenarios && selectedModes.length === 2,
        modes: selectedModes, scenarios: selectedScenarios ? [...selectedScenarios] : "all" },
};
let browser;
let failed = false;
const state = (page) => page.evaluate(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()));
const frames = (page) => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
const svgFixtureName = "Browser SVG fixture.svg";
const svgFixture = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="220" viewBox="0 0 320 220">
  <defs>
    <linearGradient id="qa-gradient" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#ef4444"/>
      <stop offset="1" stop-color="#2563eb"/>
    </linearGradient>
  </defs>
  <rect id="qa-background" width="320" height="220" fill="#f8fafc"/>
  <g id="qa-affine" transform="matrix(1 .15 .2 1 24 24)">
    <rect id="qa-panel" width="180" height="78" rx="8" fill="url(#qa-gradient)"/>
    <path id="qa-curve" d="M12 94 C52 50 114 144 176 96" fill="none" stroke="#10b981" stroke-width="8" stroke-linecap="round"/>
  </g>
  <rect id="qa-chip" x="264" y="24" width="32" height="32" fill="#10b981"/>
  <text id="qa-unicode" x="20" y="194" font-family="Inter" font-size="20" fill="#111827">Orbit · café Ω</text>
</svg>`;
function colors(bytes) {
    const png = PNG.sync.read(bytes), found = new Set();
    for (let i = 0; i < png.data.length; i += 4 * 11) {
        found.add((png.data[i] >> 3) + "," + (png.data[i + 1] >> 3) + "," + (png.data[i + 2] >> 3));
    }
    return { width: png.width, height: png.height, colors: found.size };
}
function changedPixels(before, after) {
    const a = PNG.sync.read(before), b = PNG.sync.read(after);
    assert.equal(a.width, b.width); assert.equal(a.height, b.height);
    let changed = 0;
    for (let i = 0; i < a.data.length; i += 4) {
        if (Math.abs(a.data[i] - b.data[i]) + Math.abs(a.data[i + 1] - b.data[i + 1]) + Math.abs(a.data[i + 2] - b.data[i + 2]) > 24) changed++;
    }
    return changed;
}
function pixel(png, x, y) {
    const offset = (y * png.width + x) * 4;
    return [...png.data.subarray(offset, offset + 4)];
}
function assertColor(actual, expected, label) {
    assert.ok(actual[3] >= 240, label + " must be opaque");
    for (let channel = 0; channel < 3; channel++) {
        assert.ok(Math.abs(actual[channel] - expected[channel]) <= 12,
            label + ": expected " + expected.join(",") + ", got " + actual.join(","));
    }
}
function countPixels(png, bounds, matches) {
    let count = 0;
    for (let y = bounds.top; y < bounds.bottom; y++) {
        for (let x = bounds.left; x < bounds.right; x++) {
            const offset = (y * png.width + x) * 4;
            if (matches(png.data[offset], png.data[offset + 1], png.data[offset + 2], png.data[offset + 3])) count++;
        }
    }
    return count;
}
function assertInside(child, parent, name) {
    assert.ok(child && parent && [child.x, child.y, child.width, child.height].every(Number.isFinite), name + " must have native layout bounds");
    assert.ok(child.width > 0 && child.height > 0, name + " must be arranged");
    assert.ok(child.x >= parent.x - 1 && child.y >= parent.y - 1 &&
        child.x + child.width <= parent.x + parent.width + 1 &&
        child.y + child.height <= parent.y + parent.height + 1,
        name + " must remain inside its native container: " + JSON.stringify({ child, parent }));
}
async function flushStorage(page) {
    return page.evaluate(async () => {
        if (!globalThis.ferroDesignStorage) throw new Error("The browser persistence host was not initialized.");
        let timeout;
        try {
            const saved = await Promise.race([
                globalThis.ferroDesignStorage.flush(),
                new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("The autosave transaction did not finish within 30 seconds.")), 30000); }),
            ]);
            return { saved, ...globalThis.ferroDesignStorage.status() };
        } finally { clearTimeout(timeout); }
    });
}
async function committedDocumentInfo(page) {
    return page.evaluate(async () => {
        const snapshot = await globalThis.ferroDesignStorage.backend.load();
        if (typeof snapshot?.text !== "string") throw new Error("No committed IndexedDB document exists.");
        const exported = globalThis.ferroDesign.ferroDesignExportJson();
        if (JSON.stringify(JSON.parse(snapshot.text)) !== JSON.stringify(JSON.parse(exported))) {
            throw new Error("The durable autosave must preserve every exported document field.");
        }
        const encoder = new TextEncoder();
        return {
            exportBytes: encoder.encode(exported).length,
            autosaveBytes: encoder.encode(snapshot.text).length,
            compact: !snapshot.text.includes("\n"),
            revision: snapshot.revision,
            serializationMs: JSON.parse(globalThis.ferroDesign.ferroDesignState()).lastSerializationMs,
        };
    });
}
async function waitForReload(page) {
    await page.waitForFunction(() => {
        if (!globalThis.ferroDesignReady || !globalThis.ferroDesign?.ferroDesignState) return false;
        const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return data?.renderedFrames > 0 && data?.sceneFrames > 0;
    }, undefined, { timeout: 120000 });
    await page.locator(".ferroui-splash").waitFor({ state: "hidden", timeout: 120000 });
    await frames(page);
}
function largeDocument(reference, count = 10000) {
    const node = Object.values(reference.nodes).find((item) => item.metadata?.svg_id === "qa-chip");
    assert.ok(node, "The stress fixture uses a valid exported rectangle");
    const result = {
        ...structuredClone(reference), name: "10,000-layer browser fixture", nodes: {},
        pages: [{ ...structuredClone(reference.pages[0]), id: 1, name: "Large canvas", children: [], prototype_start: null }],
        styles: {}, variables: {}, comments: [], guides: {}, variable_mode: "Default", next_id: count + 2,
    };
    const columns = Math.ceil(Math.sqrt(count));
    for (let index = 0; index < count; index++) {
        const id = index + 2;
        result.nodes[id] = {
            ...structuredClone(node), id, parent: 1, children: [], name: "Shape " + index,
            x: index % columns * 64, y: Math.floor(index / columns) * 64,
            width: 48, height: 48, corner_radius: 6, metadata: { fixture: "large-browser-document" },
        };
        result.pages[0].children.push(id);
    }
    return result;
}
const digest = (text) => createHash("sha256").update(text).digest("hex");
const documentState = (page) => page.evaluate(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()));
const documentNode = (page, id) => page.evaluate((id) => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[id], id);
async function captureFailure(page, stem) {
    const diagnostic = await page.evaluate(() => {
        const read = callback => { try { return callback(); } catch (error) { return { error: String(error) }; } };
        const element = input => input && ({
            tag: input.tagName, id: input.id, value: input.value,
            selectionStart: input.selectionStart, selectionEnd: input.selectionEnd,
        });
        return {
            state: read(() => JSON.parse(globalThis.ferroDesign.ferroDesignState())),
            storage: read(() => globalThis.ferroDesignStorage?.status()),
            focus: element(document.activeElement),
            textInputs: [...document.querySelectorAll(".ferroui-input-element")].map(element),
            documentJson: read(() => globalThis.ferroDesign.ferroDesignExportJson()),
        };
    }).catch(error => ({ error: String(error) }));
    if (typeof diagnostic.documentJson === "string") {
        const file = stem + ".ferrodesign";
        await writeFile(resolve(output, file), diagnostic.documentJson);
        diagnostic.document = { file, bytes: Buffer.byteLength(diagnostic.documentJson), sha256: digest(diagnostic.documentJson) };
    }
    delete diagnostic.documentJson;
    await page.screenshot({ path: resolve(output, stem + ".png"), timeout: 15000 }).catch(error => {
        diagnostic.screenshotError = String(error);
    });
    return diagnostic;
}
function worldPoint(view, x, y) {
    return { x: view.chrome.CanvasHost.x + view.pan.x + x * view.zoom, y: view.chrome.CanvasHost.y + view.pan.y + y * view.zoom };
}
async function moveWorld(page, x, y) {
    const point = worldPoint(await state(page), x, y);
    await page.mouse.move(point.x, point.y);
    return point;
}
async function waitForNativeFrame(page, afterFrame, label) {
    try {
        await page.waitForFunction((afterFrame) =>
            JSON.parse(globalThis.ferroDesign.ferroDesignState()).renderedFrames > afterFrame,
        afterFrame, { timeout: 10000 });
    } catch (error) {
        const view = await state(page);
        throw new Error(label + " did not produce a committed native frame: " + JSON.stringify({
            afterFrame, renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames, selection: view.selection,
            documentRevision: view.documentRevision, tool: view.tool, chrome: view.chrome,
        }), { cause: error });
    }
}
async function waitForNativeScene(page, afterScene, label) {
    try {
        await page.waitForFunction((afterScene) =>
            JSON.parse(globalThis.ferroDesign.ferroDesignState()).sceneFrames > afterScene,
        afterScene, { timeout: 10000 });
    } catch (error) {
        const view = await state(page);
        throw new Error(label + " did not publish a native hit-test scene: " + JSON.stringify({
            afterScene, renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames,
            selection: view.selection, documentRevision: view.documentRevision, chrome: view.chrome,
        }), { cause: error });
    }
}
async function clickWorld(page, x, y, options) {
    const before = await state(page), point = worldPoint(before, x, y);
    await page.mouse.click(point.x, point.y, options);
    const after = await state(page);
    // Selection updates the native properties synchronously, but its inspector
    // visibility and hit-test scene are committed by the native renderer. The
    // WebGL2 trace showed the old bounds accepting a click while
    // the previous Page inspector still owned hit testing. Wait for that real
    // selection frame before the next interaction; a DOM rAF alone is not a
    // native frame, and unchanged clicks need no extra rendering.
    if (JSON.stringify(after.selection) !== JSON.stringify(before.selection)) {
        await waitForNativeFrame(page, before.renderedFrames, "Canvas selection");
        // The canvas counter advances inside the render pass. FerroUI's
        // scene-invalidated notification follows the completed compositor
        // render, when the new inspector hit-test scene is available.
        await waitForNativeScene(page, before.sceneFrames, "Canvas selection");
    }
}
async function clickNative(page, name, scrollContainer) {
    // Model transactions can finish before FerroUI arranges a rebuilt modal.
    // Wait for the actual native control to be visible and laid out before
    // deriving pointer coordinates; a DOM animation frame alone is not proof.
    try {
        await page.waitForFunction((name) => {
            const bounds = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome[name];
            return bounds?.visible === true && bounds?.enabled === true &&
                bounds?.measureValid === true && bounds?.arrangeValid === true && bounds?.width > 0 && bounds?.height > 0 &&
                [bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite);
        }, name, { timeout: 10000 });
    } catch (error) {
        const view = await state(page);
        throw new Error(name + " is not ready for native pointer input: " + JSON.stringify({
            bounds: view.chrome[name], scrollContainer: view.chrome[scrollContainer],
            modal: view.chrome.ModalBackdrop, selection: view.selection,
            renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames,
        }), { cause: error });
    }
    for (let attempt = 0; attempt < 16; attempt++) {
        const view = await state(page), bounds = view.chrome[name];
        assert.ok(bounds && bounds.visible !== false && bounds.width > 0 && bounds.height > 0, name + " must exist in the native visual tree");
        const clip = scrollContainer ? view.chrome[scrollContainer] : { x: 0, y: 0, width: 1600, height: 1000 };
        assert.ok(clip, "Native scroll bounds must exist: " + scrollContainer);
        const x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
        if (x >= clip.x && x <= clip.x + clip.width && y >= clip.y + 3 && y <= clip.y + clip.height - 3) {
            await page.mouse.click(x, y);
            await frames(page);
            return;
        }
        assert.ok(scrollContainer, name + " must be on screen: " + JSON.stringify(bounds));
        await page.mouse.move(clip.x + clip.width / 2, clip.y + clip.height / 2);
        const delta = y - (clip.y + clip.height / 2);
        await page.mouse.wheel(0, Math.sign(delta) * Math.min(240, Math.max(80, Math.abs(delta))));
        await frames(page);
    }
    assert.fail("Could not scroll the native " + scrollContainer + " to " + name);
}
async function chooseNative(page, name, { index, offset = 1, scrollContainer } = {}) {
    const before = (await state(page)).chrome[name]?.choice;
    assert.ok(before, name + " must be a native ComboBox with selection diagnostics");
    assert.equal(before.open, false, name + " must start with its popup closed");
    const target = index ?? before.selectedIndex + offset;
    assert.ok(Number.isInteger(target) && target >= 0 && target < before.itemCount,
        name + " requires an in-range native option: " + JSON.stringify({ before, target }));
    const waitChoice = async (expected, label) => {
        try {
            await page.waitForFunction(({ name, expected }) => {
                const choice = JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome[name]?.choice;
                return choice && Object.entries(expected).every(([key, value]) => choice[key] === value);
            }, { name, expected }, { timeout: 10000 });
        } catch (error) {
            const view = await state(page);
            throw new Error(name + " " + label + " failed: " + JSON.stringify({
                expected, actual: view.chrome[name], selection: view.selection,
                selectionBounds: view.selectionBounds, renderedFrames: view.renderedFrames,
            }), { cause: error });
        }
    };
    await clickNative(page, name, scrollContainer);
    // Opening the popup must actually transfer native keyboard focus. Inspect
    // the realized item: popup visuals have their own focus ancestry, so the
    // browser's MAIN focus and the ComboBox's selected value cannot prove this.
    await waitChoice({ open: true, focusedIndex: before.selectedIndex }, "popup focus");
    let focused = before.selectedIndex;
    if (index !== undefined) {
        await page.keyboard.press("Home");
        focused = 0;
        await waitChoice({ open: true, focusedIndex: focused }, "Home navigation");
    }
    while (focused !== target) {
        const step = Math.sign(target - focused);
        await page.keyboard.press(step > 0 ? "ArrowDown" : "ArrowUp");
        focused += step;
        await waitChoice({ open: true, focusedIndex: focused }, "option navigation");
    }
    await page.keyboard.press("Enter");
    await waitChoice({ open: false, selectedIndex: target }, "selection commit");
    await frames(page);
}
async function typeNative(page, name, value) {
    await clickNative(page, name);
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText(value);
    await page.keyboard.press("Enter");
    await frames(page);
}
function isolatedDocument(reference, nodes, name) {
    return {
        ...structuredClone(reference), name, nodes: Object.fromEntries(nodes.map((node) => [node.id, node])),
        pages: [{ ...structuredClone(reference.pages[0]), id: 1, name: "Native input", children: nodes.filter((node) => node.parent === 1).map((node) => node.id), prototype_start: null }],
        styles: {}, variables: {}, comments: [], guides: {}, next_id: Math.max(1, ...nodes.map((node) => node.id)) + 1,
    };
}
function fixtureNode(reference, id, properties = {}) {
    const template = Object.values(reference.nodes).find((node) => node.metadata?.svg_id === "qa-chip");
    assert.ok(template);
    return {
        ...structuredClone(template), id, name: "Native input " + id, parent: 1, children: [],
        x: 0, y: 0, width: 240, height: 180, rotation: 0, flip_x: false, flip_y: false,
        affine_transform: null, visible: true, locked: false, opacity: 1, clip_content: false,
        layout: null, effects: [], strokes: [], metadata: {}, interactions: [],
        fill_style: null, text_style: null, effect_style: null, component_id: null, component_source: null,
        variable_bindings: {}, overridden_properties: [],
        path: { segments: [], fill_rule: "non_zero" }, ...properties,
    };
}
async function importFixture(page, fixture) {
    const previous = await state(page);
    assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), JSON.stringify(fixture)), true);
    await page.waitForFunction(({ frames, count }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.nodeCount === count && view.renderedFrames > frames;
    }, { frames: previous.renderedFrames, count: Object.keys(fixture.nodes).length });
    await frames(page);
}

// Chromium can time out one compositor readback while its page, native app,
// and trace screencast remain healthy. Keep the actual PNG gate, record every
// attempt, and retry only that recoverable operation against unchanged state.
async function captureNativeScreenshot(page, result, name, { controls = [], afterFrame = -1 } = {}) {
    const ready = () => page.waitForFunction(({ controls, afterFrame }) => {
        const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
        return view.renderedFrames > afterFrame && controls.every((name) => {
            const bounds = view.chrome[name];
            return bounds?.visible && bounds.width > 0 && bounds.height > 0 &&
                [bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite);
        });
    }, { controls, afterFrame }, { timeout: 10000 });
    const snapshot = async () => {
        let timer;
        try {
            const value = await Promise.race([
                page.evaluate(({ controls, kind }) => {
                    const view = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                    const canvas = document.querySelector("canvas.ferroui-canvas");
                    // The existing surface was already identified at startup;
                    // never initialize a different context for diagnostics.
                    const gl = kind === "WebGL2" ? canvas?.getContext("webgl2") : null;
                    return {
                        documentJson: globalThis.ferroDesign.ferroDesignExportJson(),
                        view: {
                            activePage: view.activePage, selection: view.selection, tool: view.tool,
                            pan: view.pan, zoom: view.zoom, presentation: view.presentation,
                            prototypeOverlays: view.prototypeOverlays, imageCrop: view.imageCrop,
                            layoutDrag: view.layoutDrag,
                            vectorNode: view.vectorNode, vectorPoint: view.vectorPoint,
                            modalVisible: view.chrome.ModalBackdrop.visible,
                            controls: Object.fromEntries(controls.map((name) => [name, view.chrome[name]])),
                        },
                        renderedFrames: view.renderedFrames, sceneFrames: view.sceneFrames, documentRevision: view.documentRevision,
                        fonts: { status: document.fonts.status, faces: [...document.fonts].map((font) => ({ family: font.family, status: font.status })) },
                        surface: { kind, width: canvas?.width, height: canvas?.height, contextLost: gl?.isContextLost() ?? false },
                        visibility: document.visibilityState,
                        focus: document.activeElement && { tag: document.activeElement.tagName, id: document.activeElement.id },
                        startupErrors: globalThis.ferroDesignErrors || [],
                        renderPanics: globalThis.ferroDesignRenderPanics?.() || [],
                    };
                }, { controls, kind: result.surface.kind }),
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Native screenshot diagnostics did not respond within 5 seconds.")), 5000); }),
            ]);
            value.document = { sha256: digest(value.documentJson), bytes: Buffer.byteLength(value.documentJson) };
            delete value.documentJson;
            return value;
        } finally { clearTimeout(timer); }
    };
    await ready();
    await frames(page);
    const record = { name, before: await snapshot(), attempts: [] };
    (result.nativeCaptures ||= []).push(record);
    const identity = (value) => JSON.stringify({ document: value.document.sha256, view: value.view });
    const unchanged = (value) => {
        assert.equal(identity(value), identity(record.before), "The native screenshot must capture the same authored document, selection, camera and arranged controls");
        assert.equal(value.surface.contextLost, false, "A lost WebGL context is not a recoverable screenshot timeout");
        assert.deepEqual(value.startupErrors, []);
        assert.deepEqual(value.renderPanics, []);
    };
    const diagnosticPath = resolve(output, name + "-capture.json");
    for (let attempt = 0; attempt < 2; attempt++) {
        // CI trace frames arrived 27–32 seconds late while native state stayed
        // responsive. Budget 60 seconds for the single guarded retry; this
        // measured allowance does not establish the timeout's cause or recovery.
        const entry = { attempt: attempt + 1, timeoutMs: attempt === 0 ? 30000 : 60000 };
        const started = performance.now();
        record.attempts.push(entry);
        try {
            const bytes = await page.screenshot({ path: resolve(output, name + ".png"), timeout: entry.timeoutMs });
            entry.pixels = colors(bytes);
            const viewport = page.viewportSize();
            assert.equal(entry.pixels.width, viewport.width);
            assert.equal(entry.pixels.height, viewport.height);
            assert.ok(entry.pixels.colors >= 24, "Native screenshot must contain actual rendered UI and paint pixels");
            entry.after = await snapshot();
            unchanged(entry.after);
            entry.status = "passed";
            if (attempt === 1) record.recoveredTimeout = true;
            return bytes;
        } catch (error) {
            entry.error = error.stack || String(error);
            entry.status = "failed";
            if (attempt !== 0 || error.name !== "TimeoutError") throw error;
            entry.after = await snapshot();
            unchanged(entry.after);
            await ready();
            await frames(page);
            record.retriedTimeout = true;
            console.warn(name + ": one compositor screenshot timed out; retrying once after verified unchanged native state. See " + name + "-capture.json.");
        } finally {
            entry.durationMs = Math.round(performance.now() - started);
            await writeFile(diagnosticPath, JSON.stringify(record, null, 2));
        }
    }
}

async function nativeEditingRegressions(page, canvas, reference, result, check, mode) {
    const vector = fixtureNode(reference, 2, {
        kind: "vector", name: "Editable curve", x: 20, y: 20, width: 240, height: 160,
        path: { fill_rule: "non_zero", segments: [
            { type: "move_to", point: { x: 20, y: 100 } },
            { type: "cubic_to", control1: { x: 50, y: 10 }, control2: { x: 170, y: 10 }, point: { x: 200, y: 100 } },
            { type: "line_to", point: { x: 220, y: 140 } },
            { type: "line_to", point: { x: 20, y: 140 } }, { type: "close" },
        ] },
    });
    const originalText = "Orbit · café Ω";
    const text = fixtureNode(reference, 3, {
        kind: "text", name: "Editable Unicode", x: 320, y: 60, width: 270, height: 110,
        text: originalText, font_family: "Inter", font_size: 24, font_weight: 400, text_auto_resize: "fixed",
        text_runs: [{ start: 0, end: Buffer.byteLength(originalText), font_family: null, font_size: null, font_weight: 600, color: null, italic: null, decoration: null }],
    });
    const fixture = isolatedDocument(reference, [vector, text], "Vector and text input fixture");
    await importFixture(page, fixture);
    await clickWorld(page, 130, 142);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection[0] === 2);
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).vectorNode === 2);
    const beforeDrag = await documentNode(page, 2);
    const view = await state(page), start = worldPoint(view, 40, 120), end = worldPoint(view, 62, 94);
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(end.x, end.y, { steps: 18 });
    await page.mouse.up();
    await frames(page);
    const moved = await documentNode(page, 2), point = moved.path.segments[0].point;
    assert.ok(Math.abs(moved.x + point.x - 62) < 0.6 && Math.abs(moved.y + point.y - 94) < 0.6,
        "Native pointer dragging must move the selected vector anchor to document coordinates");
    const untouched = moved.path.segments[1].point;
    assert.ok(Math.abs(moved.x + untouched.x - 220) < 0.001 && Math.abs(moved.y + untouched.y - 120) < 0.001,
        "Rebasing vector bounds must preserve unrelated anchors in world coordinates");
    assert.deepEqual((await state(page)).vectorPoint, { segment: 0, kind: "Anchor" });
    await canvas.screenshot({ path: resolve(output, mode + "-vector-edit.png") });
    await page.keyboard.press("Control+z");
    assert.deepEqual(await documentNode(page, 2), beforeDrag, "One undo must restore the complete vector before its multi-event drag");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual(await documentNode(page, 2), moved, "One redo must restore the complete vector gesture");
    await page.keyboard.press("Enter");
    assert.equal((await state(page)).vectorNode, null);
    check("Enter enters native vector editing; an anchor drag rebases bounds without moving other anchors and forms one undo/redo step");

    await clickNative(page, "PenTool");
    const penView = await state(page);
    for (const [x, y, dx, dy] of [[70, 230, 40, -30], [215, 235, 35, 35]]) {
        const anchor = worldPoint(penView, x, y), handle = worldPoint(penView, x + dx, y + dy);
        await page.mouse.move(anchor.x, anchor.y);
        await page.mouse.down();
        await page.mouse.move(handle.x, handle.y, { steps: 12 });
        await page.mouse.up();
    }
    await page.keyboard.press("Enter");
    await frames(page);
    const afterPen = await state(page), penId = afterPen.selection[0], pen = await documentNode(page, penId);
    assert.equal(afterPen.nodeCount, 3);
    assert.equal(pen.kind, "vector");
    const curve = pen.path.segments.find((segment) => segment.type === "cubic_to");
    assert.ok(curve && (curve.control2.x !== curve.point.x || curve.control2.y !== curve.point.y), "Dragging with Pen must create actual cubic control handles");
    await page.keyboard.press("Control+z");
    assert.equal((await state(page)).nodeCount, 2, "One undo must remove the entire completed Pen path");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual(await documentNode(page, penId), pen);
    check("Native Pen pointer drags create a cubic curve and the complete path is one undo/redo step");

    await clickNative(page, "PenTool");
    await clickWorld(page, 340, 230);
    const heldView = await state(page), heldStart = worldPoint(heldView, 440, 235), heldEnd = worldPoint(heldView, 465, 260);
    await page.mouse.move(heldStart.x, heldStart.y);
    await page.mouse.down();
    await page.mouse.move(heldEnd.x, heldEnd.y, { steps: 6 });
    await page.keyboard.press("Enter");
    const heldId = (await state(page)).selection[0], finalizedHeldPen = await documentNode(page, heldId);
    await page.mouse.move(heldEnd.x + 30, heldEnd.y + 20, { steps: 4 });
    await page.mouse.up();
    assert.deepEqual(await documentNode(page, heldId), finalizedHeldPen, "Mouse movement after held-pointer Enter must not mutate the finalized Pen curve");
    await page.keyboard.press("Control+z");
    assert.equal((await state(page)).nodeCount, 3, "Held-pointer Pen finalization must still produce one undo entry");
    check("Enter while the Pen pointer is held finalizes the gesture and ignores later handle movement until release");

    await importFixture(page, fixture);
    const openTextEditor = async () => {
        // Start each double-click sequence at a distinct pointer position.
        // Fast commit/undo/reopen cycles otherwise accumulate native clicks 3/4.
        await clickWorld(page, 40, 260);
        await clickWorld(page, 420, 88, { clickCount: 2, delay: 80 });
        await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === 3);
    };
    await openTextEditor();
    const inline = (await state(page)).chrome.InlineTextBox;
    assert.ok(inline?.visible && inline.width > 0 && inline.height > 0, "Double-click must open an arranged native FerroUI TextBox");
    const editedText = "FerroUI · café Ω 🚀\nEditable text";
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("FerroUI · café Ω 🚀");
    await page.keyboard.press("Enter");
    await page.keyboard.insertText("Editable text");
    await page.waitForFunction((expected) => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[3].text === expected, editedText);
    await canvas.screenshot({ path: resolve(output, mode + "-inline-text.png") });
    await page.keyboard.press("Control+Enter");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    const committedText = await documentNode(page, 3), bytes = Buffer.from(editedText);
    assert.equal(committedText.text, editedText);
    assert.ok(committedText.text_runs.length > 0, "Typing must retain inherited rich-text formatting");
    for (const run of committedText.text_runs) {
        assert.ok(run.start < run.end && run.end <= bytes.length, "Rich-text ranges must be valid UTF-8 byte offsets");
        assert.doesNotThrow(() => new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, run.start)));
        assert.doesNotThrow(() => new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, run.end)));
    }
    await page.keyboard.press("Control+z");
    assert.equal((await documentNode(page, 3)).text, originalText, "A complete native text edit must be one document undo");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual(await documentNode(page, 3), committedText);
    await openTextEditor();
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("Discard this draft");
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.deepEqual(await documentNode(page, 3), committedText, "Escape must restore the original text and every formatting run");
    await page.keyboard.press("Control+z");
    assert.equal((await documentNode(page, 3)).text, originalText, "A canceled draft must not add an undo entry");
    await page.keyboard.press("Control+Shift+z");
    assert.equal((await flushStorage(page)).saved, true);
    const persistedText = await page.evaluate(() => JSON.parse(globalThis.ferroDesignStorage.load()).nodes[3]);
    assert.deepEqual(persistedText, committedText, "Committed Unicode text and formatting must reach durable storage");
    result.nativeEditing = { vectorAnchor: { x: moved.x + point.x, y: moved.y + point.y }, penSegments: pen.path.segments.length, unicodeBytes: bytes.length, textRuns: committedText.text_runs.length };
    check("Native on-canvas TextBox handles multiline Unicode, UTF-8 rich-text ranges, Ctrl+Enter commit, Escape cancel, one-step history, and autosave");

    // Drive browser composition events through the production bridge and the
    // actual native TextBox. These deterministic lifecycle regressions are not
    // a substitute for testing an operating-system IME on each target device.
    const ime = page.locator(".ferroui-input-element");
    assert.equal(await ime.count(), 1, "The editor has one native browser text-input bridge");
    const beginComposition = async () => {
        await openTextEditor();
        await page.keyboard.press("Control+a");
        await ime.evaluate((input) => {
            if (document.activeElement !== input) throw new Error("The native TextBox must own browser input focus");
            input.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "" }));
            input.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data: "日本" }));
            input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertCompositionText", data: "日本", isComposing: true }));
        });
        assert.equal((await state(page)).editingComposition, true, "The actual native text client must have pending preedit");
        assert.deepEqual(await documentNode(page, 3), committedText, "The document must retain confirmed text and runs while native preedit replaces the selected range");
    };
    await beginComposition();
    const composedText = "日本語 · café Ω 🚀";
    await ime.evaluate((input, data) => {
        input.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, cancelable: true, data }));
        input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data, isComposing: false }));
    }, composedText);
    assert.equal((await state(page)).editingComposition, false, "Composition end must clear the native pending state");
    assert.equal((await documentNode(page, 3)).text, composedText, "A composition and trailing insertText must commit exactly once");
    await page.keyboard.press("Control+Enter");
    await page.keyboard.press("Control+z");
    assert.deepEqual(await documentNode(page, 3), committedText, "The complete committed composition must be one document undo");

    await beginComposition();
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.deepEqual(await documentNode(page, 3), committedText, "Losing the text client must discard its preedit");
    await openTextEditor();
    await page.keyboard.press("Control+a");
    const resumedText = "After canceled IME · Ω 🚀";
    await page.keyboard.insertText(resumedText);
    assert.equal((await documentNode(page, 3)).text, resumedText, "A new text client must accept input before the canceled composition sends its late end");
    await ime.evaluate((input) => {
        input.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data: "late preedit" }));
        input.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, cancelable: true, data: "late commit" }));
        input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "late commit", isComposing: false }));
    });
    assert.equal((await documentNode(page, 3)).text, resumedText, "Late composition events must not edit the replacement client");
    await page.keyboard.insertText("!");
    assert.equal((await documentNode(page, 3)).text, resumedText + "!", "Independent input remains usable after a late composition end");
    await page.keyboard.press("Escape");
    assert.deepEqual(await documentNode(page, 3), committedText, "Canceling the replacement draft must restore the complete text and formatting");
    check("Browser composition events commit once, keep preedit out of the document, and cannot block or overwrite a new native text client after cancellation");

    await openTextEditor();
    await page.keyboard.press("Control+a");
    await page.keyboard.insertText("Confirmed before composition · Ω 🚀");
    const confirmedDraft = await documentNode(page, 3);
    await page.keyboard.press("Control+a");
    await ime.evaluate((input) => {
        input.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "" }));
        input.dispatchEvent(new CompositionEvent("compositionupdate", { bubbles: true, data: "未確定" }));
    });
    assert.equal((await state(page)).editingComposition, true, "The lifecycle regression must start with pending native preedit");
    assert.deepEqual(await documentNode(page, 3), confirmedDraft, "Pending composition must not replace the last confirmed document draft");
    // Use the real page lifecycle listener, whose synchronous native committer
    // must preserve confirmed input even without a browser compositionend.
    await page.evaluate(() => globalThis.dispatchEvent(new Event("pagehide")));
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).editingText === null);
    assert.equal((await flushStorage(page)).saved, true);
    assert.deepEqual(await page.evaluate(() => JSON.parse(globalThis.ferroDesignStorage.load()).nodes[3]), confirmedDraft);
    // Hiding the inline host releases native focus. Resume through a real
    // canvas click before exercising document keyboard history.
    await clickWorld(page, 620, 240);
    await page.keyboard.press("Control+z");
    assert.deepEqual(await documentNode(page, 3), committedText, "Lifecycle commit must remain one undo step for the confirmed input");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual(await documentNode(page, 3), confirmedDraft);
    assert.equal((await flushStorage(page)).saved, true);
    await page.reload({ waitUntil: "domcontentloaded", timeout: 120000 });
    await waitForReload(page);
    assert.deepEqual(await documentNode(page, 3), confirmedDraft, "A real reload must recover confirmed Unicode and formatting, never the intermediate IME deletion");
    check("Pagehide during unfinished composition durably saves confirmed Unicode and formatting, preserves one-step history, and survives a real reload");

    const paintFixture = isolatedDocument(reference, [fixtureNode(reference, 2)], "Paint stack input fixture");
    await importFixture(page, paintFixture);
    await clickWorld(page, 120, 90);
    await clickNative(page, "EditPaintsButton", "InspectorViewport");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).chrome.ModalBackdrop.visible);
    await chooseNative(page, "PaintType0");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].fills[0].paint.type === "linear_gradient");
    await typeNative(page, "PaintStopColor0_0", "#FF3B6B80");
    let fill = (await documentNode(page, 2)).fills[0];
    assert.deepEqual(fill.paint.stops[0].color, { r: 255, g: 59, b: 107, a: 128 });
    await typeNative(page, "PaintOpacity0", "42");
    assert.equal((await documentNode(page, 2)).fills[0].opacity, 0.42);
    await typeNative(page, "PaintOpacity0", "NaN");
    assert.equal((await documentNode(page, 2)).fills[0].opacity, 0.42, "Invalid native field input must not alter the model");
    await typeNative(page, "PaintOpacity0", "65");
    await typeNative(page, "PaintStopOffset0_0", "12.5");
    assert.equal((await documentNode(page, 2)).fills[0].paint.stops[0].offset, 0.125);
    const paintFrameBeforeAdd = (await state(page)).renderedFrames;
    await clickNative(page, "PaintStopAdd0");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignExportJson()).nodes[2].fills[0].paint.stops.length === 3);
    fill = (await documentNode(page, 2)).fills[0];
    assert.equal(fill.opacity, 0.65);
    await captureNativeScreenshot(page, result, mode + "-paint-stack", {
        controls: ["PaintCard0", "PaintStopColor0_2", "PaintDone"], afterFrame: paintFrameBeforeAdd,
    });
    await clickNative(page, "PaintDone");
    assert.equal((await state(page)).chrome.ModalBackdrop.visible, false);
    await page.keyboard.press("Control+z");
    assert.equal((await documentNode(page, 2)).fills[0].paint.stops.length, 2, "Adding a stop must form one undo entry");
    await page.keyboard.press("Control+Shift+z");
    assert.deepEqual((await documentNode(page, 2)).fills[0], fill);
    check("Native paint dialog pointer/keyboard input converts a fill to a gradient, edits alpha/opacity/stop positions, rejects NaN, and supports one-step stop undo");
}

async function prototypeInputRegressions(page, reference, check) {
    const transition = { kind: "instant", duration_ms: 0, easing: "linear" };
    const interaction = (trigger, target, key = null) => ({ trigger, action: { type: "navigate_to", target }, transition, delay_ms: 0, key });
    const framesAndButtons = [];
    for (const [index, id] of [10, 20, 30, 40, 50].entries()) {
        framesAndButtons.push(fixtureNode(reference, id, { kind: "frame", name: "Prototype " + id, x: index * 400, y: 0, width: 320, height: 220, clip_content: true, children: [id + 1] }));
        const interactions = id === 10 ? [interaction("hover", 20)]
            : id === 20 ? [interaction("press", 30)]
            : id === 30 ? [interaction("drag", 40), interaction("click", 50)]
            : id === 40 ? [interaction("key", 50, "Ctrl+K")] : [];
        framesAndButtons.push(fixtureNode(reference, id + 1, { parent: id, x: 40, y: 60, width: 120, height: 60, locked: id === 10, interactions }));
    }
    const fixture = isolatedDocument(reference, framesAndButtons, "Prototype input fixture");
    fixture.pages[0].prototype_start = 10;
    await importFixture(page, fixture);
    const authored = await documentState(page);
    // The same sanctioned command as the Present button initializes playback;
    // every interaction below is dispatched by actual native pointer/key events.
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("present")), true);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 10);
    await frames(page);
    await moveWorld(page, 100, 90);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 20);
    await frames(page);
    await moveWorld(page, 500, 90);
    await page.mouse.down();
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 30);
    await page.mouse.up();
    await frames(page);
    assert.equal((await state(page)).presentation, 30, "Press must navigate before mouse release");
    const dragStart = await moveWorld(page, 900, 90);
    await page.mouse.down();
    await page.mouse.move(dragStart.x + 45, dragStart.y + 12, { steps: 8 });
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 40);
    await page.mouse.up();
    await frames(page);
    assert.equal((await state(page)).presentation, 40, "A drag gesture must suppress its competing click navigation");
    await page.keyboard.press("Control+k");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 50);
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === null);
    assert.deepEqual(await documentState(page), authored, "Stopping prototype playback must preserve the authored document exactly");
    check("Real prototype hover (including locked layers), press, drag and key chord navigate frames; drag suppresses click and Escape restores authored state");

    const timed = structuredClone(fixture);
    timed.name = "Prototype delayed event fixture";
    for (const node of Object.values(timed.nodes)) node.interactions = [];
    timed.nodes[10].interactions = [{ ...interaction("after_delay", 20), delay_ms: 80 }];
    await importFixture(page, timed);
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("present")), true);
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 20, undefined, { timeout: 10000 });
    await page.keyboard.press("Escape");
    assert.equal((await state(page)).presentation, null);
    check("The actual native dispatcher timer fires an authored After delay prototype interaction");

    const boundaries = structuredClone(fixture);
    boundaries.name = "Prototype transaction boundaries";
    for (const node of Object.values(boundaries.nodes)) node.interactions = [];
    boundaries.nodes[11].interactions = [interaction("press", 10), interaction("click", 20)];
    boundaries.nodes[10].interactions = [interaction("key", 20, "Escape")];
    await importFixture(page, boundaries);
    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("present")), true);
    await frames(page);
    await moveWorld(page, 100, 90);
    await page.mouse.down();
    await page.mouse.up();
    await frames(page);
    assert.equal((await state(page)).presentation, 10, "Same-frame Press navigation must clear the old press so release cannot dispatch its competing Click");
    check("Same-frame Press navigation suppresses a stale Click on mouse release");
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === 20);
    await page.keyboard.press("Escape");
    await page.waitForFunction(() => JSON.parse(globalThis.ferroDesign.ferroDesignState()).presentation === null);
    check("An authored Escape key interaction runs first; an unhandled Escape exits presentation");

    assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("present")), true);
    assert.equal((await state(page)).presentation, 10);
    const replacement = isolatedDocument(reference, [fixtureNode(reference, 2, { name: "Imported replacement" })], "Imported while preview was active");
    await importFixture(page, replacement);
    assert.equal((await state(page)).presentation, null, "A direct import bridge call must stop preview before replacing the authored document");
    const imported = await documentState(page);
    assert.equal(imported.name, replacement.name);
    assert.deepEqual(Object.keys(imported.nodes), ["2"]);
    await page.keyboard.press("Escape");
    assert.deepEqual(await documentState(page), imported, "Escape must not restore the old preview snapshot over a successful direct import");
    check("Direct bridge import during presentation becomes the authored document and survives Escape");
}
try {
    browser = await chromium.launch({
        headless: true,
        executablePath: process.env.FERRODESIGN_CHROMIUM || undefined,
        args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
    });
    report.browser = browser.version();
    for (const mode of selectedModes) {
        const context = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 1, acceptDownloads: true });
        await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
        const page = await context.newPage();
        const result = { requestedRenderer: "Vello", requestedMode: mode, status: "running", pageErrors: [], consoleErrors: [], checks: [] };
        report.results.push(result);
        page.on("pageerror", (error) => result.pageErrors.push(error.stack || String(error)));
        page.on("console", (message) => { if (message.type() === "error") result.consoleErrors.push(message.text()); });
        const check = (name) => result.checks.push(name);
        try {
            const started = performance.now();
            await page.goto(base + "/?RenderingMode=" + mode, { waitUntil: "domcontentloaded", timeout: 120000 });
            await page.waitForFunction(() => globalThis.ferroDesignReady === true || globalThis.ferroDesignErrors?.length > 0, undefined, { timeout: 120000 });
            const startupErrors = await page.evaluate(() => globalThis.ferroDesignErrors || []);
            assert.deepEqual(startupErrors, [], "The application must start without a runtime or module-loading error");
            await page.waitForFunction(() => {
                const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return data && data.nodeCount > 0 && data.renderedFrames > 0;
            }, undefined, { timeout: 120000 });
            await page.locator(".ferroui-splash").waitFor({ state: "hidden", timeout: 120000 });
            const canvas = page.locator("canvas.ferroui-canvas").first();
            await canvas.waitFor({ state: "visible" });
            await frames(page);
            result.firstFrameMs = Math.round(performance.now() - started);
            result.initial = await state(page);
            assert.equal(result.initial.renderer, "FerroUI Vello", "The default application backend must be Vello");
            assert.equal(result.initial.zoomLabel, Math.round(result.initial.zoom * 100) + "%", "Native zoom label must reflect the initial fitted camera");
            assert.ok(result.initial.pageCount > 0);
            const baseline = await canvas.screenshot({ path: resolve(output, mode + "-initial.png") });
            result.pixels = colors(baseline);
            assert.ok(result.pixels.width >= 1000 && result.pixels.height >= 600, "Full application canvas must be rendered");
            assert.ok(result.pixels.colors >= 24, "The canvas must contain actual application pixels");
            result.surface = await page.evaluate(() => {
                const canvas = document.querySelector("canvas.ferroui-canvas");
                const gl = canvas.getContext("webgl2");
                if (!gl) return { kind: canvas.getContext("2d") ? "Software2D" : "unknown" };
                const debug = gl.getExtension("WEBGL_debug_renderer_info");
                return { kind: "WebGL2", device: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : "unreported" };
            });
            assert.notEqual(result.surface.kind, "unknown");
            assert.equal(result.surface.kind, mode, "The requested surface must be rendered, without substituting another mode");
            check("Native FerroUI canvas contains rendered UI and document pixels");

            // Focus the document viewport through a real pointer event, then invoke its keyboard shortcut.
            await page.mouse.click(720, 630);
            await page.keyboard.press("r");
            await page.mouse.move(720, 630);
            await page.mouse.down();
            await page.mouse.move(900, 770, { steps: 12 });
            await page.mouse.up();
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            result.afterDraw = await state(page);
            assert.equal(result.afterDraw.selectionCount, 1, "The newly drawn rectangle must be selected");
            await frames(page);
            const drawn = await canvas.screenshot({ path: resolve(output, mode + "-edited.png") });
            result.changedPixels = changedPixels(baseline, drawn);
            assert.ok(result.changedPixels > 200, "Pointer drawing must change actual canvas pixels");
            check("R shortcut and pointer drag create and render a selected rectangle");

            await page.keyboard.press("Control+z");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count, result.initial.nodeCount);
            check("Keyboard undo removes the pointer-created node");
            await page.keyboard.press("Control+Shift+z");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            check("Keyboard redo restores the node");

            const saved = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.doesNotThrow(() => JSON.parse(saved));
            await page.keyboard.press("Delete");
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count, result.initial.nodeCount);
            check("Delete keyboard action removes the selection");
            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), saved), true);
            await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).nodeCount === count + 1, result.initial.nodeCount);
            const restored = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.deepEqual(JSON.parse(restored), JSON.parse(saved), "Document import/export must preserve every serialized field");
            check("Document JSON round-trip restores the complete edited document");

            // Import through the same public bridge used by the real file picker.
            const beforeSvg = await state(page);
            assert.equal(await page.evaluate(({ source, name }) => globalThis.ferroDesign.ferroDesignImportFile(source, name),
                { source: svgFixture, name: svgFixtureName }), true, "The SVG fixture must import successfully");
            await page.waitForFunction((before) => {
                const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return data.nodeCount > before.nodeCount && data.selectionCount === 1 && data.renderedFrames > before.renderedFrames;
            }, beforeSvg);
            await frames(page);
            result.svgImport = await state(page);
            assert.equal(result.svgImport.invalidNodes, 0, "Imported SVG must produce no invalid renderer nodes");
            assert.equal(result.svgImport.missingImages, 0, "Imported SVG must not introduce missing resources");
            const svgJson = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            const svgDocument = JSON.parse(svgJson);
            const importedRootId = result.svgImport.selection[0];
            const importedRoot = svgDocument.nodes[String(importedRootId)];
            assert.ok(importedRoot, "Selected import root must exist in the serialized document");
            assert.equal(importedRoot.kind, "frame");
            assert.equal(importedRoot.name, svgFixtureName);
            assert.equal(importedRoot.width, 320);
            assert.equal(importedRoot.height, 220);
            assert.equal(importedRoot.parent, result.svgImport.activePage);
            assert.ok(svgDocument.pages.find((item) => item.id === result.svgImport.activePage).children.includes(importedRootId));
            const findSvgNode = (id) => Object.values(svgDocument.nodes).find((node) => node.metadata?.svg_id === id);
            assert.ok(findSvgNode("qa-affine")?.affine_transform, "Group shear must remain an editable affine transform");
            assert.equal(findSvgNode("qa-panel")?.fills[0]?.paint.type, "linear_gradient");
            assert.ok(findSvgNode("qa-curve")?.path.segments.some((segment) => segment.type === "cubic_to"), "Curves must remain editable path segments");
            assert.equal(findSvgNode("qa-unicode")?.text, "Orbit · café Ω");
            await canvas.screenshot({ path: resolve(output, mode + "-svg.png") });
            check("SVG file import selects an editable frame with affine groups, gradients, curves, and Unicode text");

            // Capture the actual application download. This exercises the FerroUI
            // offscreen render target and PNG encoder, independently of screenshots.
            const [download, exported] = await Promise.all([
                page.waitForEvent("download", { timeout: 60000 }),
                page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("export-png")),
            ]);
            assert.equal(exported, true, "The application must handle the PNG export command");
            assert.equal(await download.failure(), null, "PNG download must finish successfully");
            assert.match(download.suggestedFilename(), /\.png$/i);
            const pngPath = resolve(output, mode + "-export.png");
            await download.saveAs(pngPath);
            const pngBytes = await readFile(pngPath);
            assert.deepEqual(pngBytes.subarray(0, 8), Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), "Download must be a real PNG file");
            const png = PNG.sync.read(pngBytes);
            assert.equal(png.width, 320, "PNG width must match the selected SVG frame");
            assert.equal(png.height, 220, "PNG height must match the selected SVG frame");
            const background = pixel(png, 8, 8), chip = pixel(png, 280, 40);
            assertColor(background, [248, 250, 252], "Exported artboard background");
            assertColor(chip, [16, 185, 129], "Exported solid green shape");
            const red = pixel(png, 45, 50), blue = pixel(png, 205, 113);
            assert.ok(red[3] > 240 && red[0] > 160 && red[0] > red[2] + 50, "Gradient start must render red");
            assert.ok(blue[3] > 240 && blue[2] > 160 && blue[2] > blue[0] + 50,
                "Gradient end must render blue at its affine-transformed position");
            const curvePixels = countPixels(png, { left: 30, top: 90, right: 245, bottom: 170 },
                (r, g, b, a) => a > 200 && r < 100 && g > 130 && b > 40 && b < 180 && g > b + 20);
            assert.ok(curvePixels > 100, "The exported curved stroke must contain visible green pixels");
            const textPixels = countPixels(png, { left: 15, top: 165, right: 305, bottom: 210 },
                (r, g, b, a) => a > 200 && r < 90 && g < 100 && b < 130);
            assert.ok(textPixels > 60, "The exported Unicode text must contain rendered glyph pixels");
            result.pngExport = { filename: download.suggestedFilename(), bytes: pngBytes.length, ...colors(pngBytes), background, chip, red, blue, curvePixels, textPixels };
            assert.ok(result.pngExport.colors >= 32, "Export must retain the gradient's color variation");
            check("PNG download has exact artboard dimensions and verified background, affine gradient, curved stroke, and text pixels");

            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), svgJson), true);
            await frames(page);
            const reopenedSvgJson = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.deepEqual(JSON.parse(reopenedSvgJson), svgDocument, "Saving and reopening must preserve every imported SVG field");
            const afterSvgRoundTrip = await state(page);
            assert.equal(afterSvgRoundTrip.invalidNodes, 0, "Reopened SVG must still contain valid renderer nodes");
            assert.equal(afterSvgRoundTrip.missingImages, 0, "Reopened SVG must not introduce missing resources");
            check("JSON round-trip preserves the complete imported SVG hierarchy and visual properties");

            await nativeEditingRegressions(page, canvas, svgDocument, result, check, mode);
            // Static native checkpoints share the same bounded capture and
            // unchanged-state diagnostics. Timed prototype animation captures
            // below intentionally keep their existing timing-sensitive path.
            const nativeHelpers = {
                fixtureNode, isolatedDocument, importFixture, documentState, state,
                clickWorld, clickNative, chooseNative, moveWorld, frames, waitForNativeFrame, waitForNativeScene,
                captureNativeScreenshot: (name, options) => captureNativeScreenshot(page, result, name, options),
            };
            // Each feature owns a real application session. The production
            // undo history is deliberately bounded at 100 entries; accumulating
            // unrelated feature edits in one editor makes depth + 1 assertions
            // invalid at that boundary. Flush the actual native draft/storage
            // lifecycle, verify durable content, then reload without clearing
            // editor state through a test-only mutation or raising its limit.
            const scenario = async (name, run) => {
                const entry = { name, status: "running", checksBefore: result.checks.length };
                (result.scenarios ||= []).push(entry);
                if (selectedScenarios && !selectedScenarios.has(name)) {
                    entry.status = "not-requested";
                    return;
                }
                console.log(mode + ": starting " + name);
                const passed = await runIsolatedScenario(entry, {
                    prepare: async () => {
                        const before = await documentState(page);
                        assert.equal(await page.evaluate(() => globalThis.ferroDesign.ferroDesignCommand("commit-edit")), true,
                            name + " requires a successful native draft flush");
                        assert.deepEqual(await documentState(page), before,
                            name + " must not inherit an uncommitted edit from the previous scenario");
                        const saved = await flushStorage(page);
                        assert.equal(saved.saved, true, name + " requires durable storage before reload");
                        assert.equal(saved.dirty, false);
                        await committedDocumentInfo(page);
                        const previousPanics = await page.evaluate(() => ({
                            startup: globalThis.ferroDesignErrors || [],
                            render: globalThis.ferroDesignRenderPanics?.() || [],
                        }));
                        assert.deepEqual(previousPanics, { startup: [], render: [] },
                            "Reload cannot hide a previous scenario's native panic");
                        await page.reload({ waitUntil: "domcontentloaded", timeout: 120000 });
                        await waitForReload(page);
                        assert.deepEqual(await documentState(page), before,
                            name + " reload must preserve every committed document field");
                        const reloaded = await state(page);
                        assert.equal(reloaded.historyLength, 0, "A new application session starts with empty undo history");
                        assert.equal(reloaded.canRedo, false);
                    },
                    run,
                    checkCount: () => result.checks.length,
                    captureFailure: async current => {
                        failed = true;
                        console.error(mode + ": " + name + " failed during " + current.failureStage + ": " + current.error);
                        const stem = mode + "-" + name + "-failure";
                        result.failureScreenshot ||= stem + ".png";
                        current.failure = await captureFailure(page, stem);
                    },
                });
                // Keep the failed gate and complete only later scenarios whose
                // normal session-boundary checks independently succeed. A
                // boundary failure throws and aborts this renderer mode.
                if (!passed) failed = true;
            };
            await scenario("vector-network", () => vectorNetworkRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("native-authoring", () => nativeAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("corners", () => cornerRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("polygon-corners", () => polygonCornerRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("corner-radius-handles", () => cornerRadiusHandleRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("width-profiles", () => widthProfileRegressions(page, svgDocument, check, nativeHelpers, output, mode));
            await scenario("width-handles", () => widthHandleRegressions(page, svgDocument, check, nativeHelpers, output, mode));
            await scenario("gradient-handles", () => gradientHandleRegressions(page, svgDocument, check, nativeHelpers, output, mode));
            await scenario("paragraphs", () => paragraphRegressions(page, svgDocument, check, nativeHelpers, output, mode));
            await scenario("typography", () => typographyRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("components", () => componentAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("image-crop", () => imageCropRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("compositing", () => compositingRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("svg-compositing", () => svgCompositingRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("masks", () => maskAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("effects", () => effectAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("inspector-lifecycle", () => inspectorLifecycleRegressions(page, svgDocument, check,
                { ...nativeHelpers, flushStorage, waitForReload }));
            await scenario("floating-panels", () => floatingPanelRegressions(page, svgDocument, check,
                nativeHelpers, mode));
            await scenario("page-inspector", () => pageInspectorRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("auto-layout", () => layoutAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("layout-padding", () => layoutPaddingRegressions(page, svgDocument, check,
                { ...nativeHelpers, flushStorage }, output, mode));
            await scenario("layout-variables", () => layoutVariableRegressions(page, svgDocument, check,
                { ...nativeHelpers, flushStorage }, output, mode));
            await scenario("layout-drag", () => layoutDragRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("layout-transfer", () => layoutTransferRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("selection", () => selectionRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("matching", () => matchingRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("geometry-authoring", () => geometryAuthoringRegressions(page, svgDocument, check,
                nativeHelpers, mode));
            await scenario("flatten", () => flattenRegressions(page, svgDocument, check,
                nativeHelpers, mode));
            await scenario("local-styles", () => localStyleRegressions(page, svgDocument, check,
                nativeHelpers, mode));
            await scenario("angular-paints", () => angularPaintRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("grid-styles", () => gridStyleRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("guide-bindings", () => guideBindingRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("region-paints", () => regionPaintRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("network-bucket", () => networkBucketRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("compact-autosave", () => compactAutosaveRegressions(page, svgDocument, check,
                { ...nativeHelpers, flushStorage }, output, mode));
            await scenario("prototype-input", () => prototypeInputRegressions(page, svgDocument, check));
            await scenario("prototype-motion", () => prototypeMotionRegressions(page, svgDocument, check,
                nativeHelpers, output, mode));
            await scenario("bounded-history", () => boundedHistoryRegressions(page, svgDocument, check,
                nativeHelpers));
            if (selectedScenarios) {
                for (const name of selectedScenarios) assert.ok(result.scenarios.some(item => item.name === name),
                    "Unknown FERRODESIGN_SCENARIOS entry: " + name);
            }
            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), svgJson), true);
            await frames(page);

            // Validate the desktop minimum with the actual native layout. These
            // rectangles come from FerroUI's visual tree, not HTML stand-ins.
            await page.setViewportSize({ width: 980, height: 650 });
            await page.waitForFunction(() => {
                const canvas = document.querySelector("canvas.ferroui-canvas");
                const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                return canvas?.width === 980 && canvas.height === 650 && data.canvas.width <= 396.1;
            });
            await frames(page);
            const compact = await state(page);
            assertInside(compact.chrome.Toolbar, compact.chrome.CanvasHost, "Floating toolbar");
            for (const name of ["SelectTool", "FrameTool", "RectTool", "EllipseTool", "PenTool", "TextTool", "HandTool", "CommentTool", "ActionsButton"]) {
                assertInside(compact.chrome[name], compact.chrome.Toolbar, name);
            }
            for (const [name, tool] of [["HandTool", "Hand"], ["SelectTool", "Select"]]) {
                const bounds = compact.chrome[name];
                await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
                await page.waitForFunction((expected) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).tool === expected, tool);
            }
            await frames(page);
            result.compact = { viewport: { width: 980, height: 650 }, chrome: (await state(page)).chrome };
            await canvas.screenshot({ path: resolve(output, mode + "-compact.png") });
            check("All nine toolbar controls fit at 980×650; native toolbar pointer input switches tools");

            for (let added = 0; added < 8; added++) {
                const before = await state(page);
                const bounds = before.chrome.AddPageButton;
                await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
                await page.waitForFunction((count) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).pageCount === count + 1, before.pageCount);
            }
            await frames(page);
            const manyPages = await state(page);
            assert.equal(manyPages.pageCount, compact.pageCount + 8);
            assert.ok(manyPages.chrome.PagesViewport.height <= 155.1, "Long page lists must stay within their scrolling viewport");
            assert.ok(manyPages.chrome.LayersViewport.height >= 180, "The layer list must retain usable height when many pages exist");
            assertInside(manyPages.chrome.ActivePage, manyPages.chrome.PagesViewport, "Newly active page");
            result.compact.manyPages = { pageCount: manyPages.pageCount, chrome: manyPages.chrome };
            await canvas.screenshot({ path: resolve(output, mode + "-many-pages.png") });
            check("Native Add page clicks retain a scrollable page list and usable layer viewport at 980×650");
            assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), svgJson), true);
            assert.deepEqual(JSON.parse(await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson())), svgDocument);
            await page.setViewportSize({ width: 1600, height: 1000 });
            await frames(page);

            const saveStarted = performance.now();
            const persisted = await flushStorage(page);
            const saveCompleted = performance.now();
            assert.equal(persisted.saved, true, "Autosave must finish its storage transaction");
            assert.equal(persisted.backend, "indexedDB");
            assert.equal(persisted.dirty, false);
            const beforeReload = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            const stored = await committedDocumentInfo(page);
            assert.equal(stored.compact, true, "Autosave serializes compact JSON");
            assert.ok(stored.autosaveBytes < stored.exportBytes);
            const reloadStarted = performance.now();
            await page.reload({ waitUntil: "domcontentloaded", timeout: 120000 });
            await waitForReload(page);
            const afterReload = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
            assert.deepEqual(JSON.parse(afterReload), JSON.parse(beforeReload), "Reload must restore the complete authored document from IndexedDB before Rust creates the shell");
            assert.equal((await state(page)).saveStatus, "All changes saved locally");
            result.persistence = { backend: persisted.backend, revision: persisted.revision, saveMs: Math.round(saveCompleted - saveStarted), reloadMs: Math.round(performance.now() - reloadStarted), exportBytes: Buffer.byteLength(beforeReload), autosaveBytes: stored.autosaveBytes, autosaveSerializationMs: stored.serializationMs };
            await canvas.screenshot({ path: resolve(output, mode + "-reloaded.png") });
            check("A committed IndexedDB autosave restores the exact document after a real browser reload");

            // Exercise the actual Rust model, 10,000 native layer rows, renderer,
            // pointer editing, history and persistence together. Time figures
            // describe this machine; correctness and bounded control realization
            // are the gates, not an invented device-independent frame rate.
            if (mode === "Software2D") {
                const fixture = largeDocument(svgDocument);
                const fixtureJson = JSON.stringify(fixture);
                assert.ok(Buffer.byteLength(fixtureJson) > 5 * 1024 * 1024, "The stress document must exceed common localStorage capacity");
                const beforeLarge = await state(page);
                const importedAt = performance.now();
                assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), fixtureJson), true);
                await page.waitForFunction((before) => {
                    const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                    return data.nodeCount === 10000 && data.renderedFrames > before;
                }, beforeLarge.renderedFrames, { timeout: 120000 });
                await frames(page);
                const large = await state(page);
                assert.equal(large.layerRowCount, 10000, "Every stress layer must be present in the native row model");
                assert.ok(large.realizedLayerRows > 0 && large.realizedLayerRows <= 200,
                    "The native layer list must realize a bounded viewport of controls: " + large.realizedLayerRows);
                assert.equal(large.invalidNodes, 0);
                assert.ok(large.visibleNodes >= 9000, "Fit page must render the dense 10,000-node fixture");
                result.largeDocument = { nodeCount: large.nodeCount, importBytes: Buffer.byteLength(fixtureJson), importAndFirstFrameMs: Math.round(performance.now() - importedAt), layerRows: large.layerRowCount, realizedRows: large.realizedLayerRows, visibleNodes: large.visibleNodes };
                await canvas.screenshot({ path: resolve(output, mode + "-large-document.png") });
                const layers = large.chrome.LayersViewport;
                await page.mouse.move(layers.x + layers.width / 2, layers.y + layers.height / 2);
                await page.mouse.wheel(0, 5000);
                await frames(page);
                const scrolled = await state(page);
                assert.ok(scrolled.realizedLayerRows > 0 && scrolled.realizedLayerRows <= 200,
                    "Scrolling 10,000 layers must recycle native rows");
                result.largeDocument.realizedAfterScroll = scrolled.realizedLayerRows;

                const selectedId = 5052, selected = fixture.nodes[selectedId];
                const view = await state(page);
                const x = view.chrome.CanvasHost.x + view.pan.x + (selected.x + selected.width / 2) * view.zoom;
                const y = view.chrome.CanvasHost.y + view.pan.y + (selected.y + selected.height / 2) * view.zoom;
                await page.mouse.click(x, y);
                await page.waitForFunction((id) => JSON.parse(globalThis.ferroDesign.ferroDesignState()).selection[0] === id, selectedId);
                await frames(page);
                const beforeNudge = await state(page);
                const preparationAfter = async (before, expectedX, label) => {
                    await page.waitForFunction(({ before, expectedX }) => {
                        const data = JSON.parse(globalThis.ferroDesign.ferroDesignState());
                        return data.selectionBounds.x === expectedX &&
                            (data.totalFullPrepares > before.totalFullPrepares ||
                             data.totalIncrementalPrepares > before.totalIncrementalPrepares);
                    }, { before, expectedX });
                    const after = await state(page);
                    const delta = {
                        full: after.totalFullPrepares - before.totalFullPrepares,
                        incremental: after.totalIncrementalPrepares - before.totalIncrementalPrepares,
                        nodes: after.totalPreparedNodes - before.totalPreparedNodes,
                        visuals: after.totalRebuiltVisuals - before.totalRebuiltVisuals,
                    };
                    assert.deepEqual(delta, { full: 0, incremental: 1, nodes: 1, visuals: 0 },
                        label + " must prepare only the changed leaf and reuse its immutable visual resources");
                    const layerProjection = {
                        builds: after.layerProjectionBuilds - before.layerProjectionBuilds,
                        reuses: after.layerProjectionReuses - before.layerProjectionReuses,
                    };
                    assert.equal(layerProjection.builds, 0,
                        label + " must reuse the 10,000-layer projection without rebuilding rows or names");
                    assert.ok(layerProjection.reuses >= 1,
                        label + " must pass through the native shell's layer-projection cache");
                    return { after, delta, layerProjection };
                };
                const nudgedAt = performance.now();
                await page.keyboard.press("ArrowRight");
                const nudgePreparation = await preparationAfter(beforeNudge, selected.x + 1, "Native nudge");
                result.largeDocument.nudgeMs = Math.round(performance.now() - nudgedAt);
                await page.keyboard.press("Control+z");
                const undoPreparation = await preparationAfter(nudgePreparation.after, selected.x, "Native undo");
                await page.keyboard.press("Control+Shift+z");
                const redoPreparation = await preparationAfter(undoPreparation.after, selected.x + 1, "Native redo");
                result.largeDocument.preparation = {
                    nudge: nudgePreparation.delta, undo: undoPreparation.delta, redo: redoPreparation.delta,
                };
                result.largeDocument.layerProjection = {
                    nudge: nudgePreparation.layerProjection,
                    undo: undoPreparation.layerProjection,
                    redo: redoPreparation.layerProjection,
                };
                check("Native nudge/undo/redo in 10,000 layers each prepare one leaf, rebuild no immutable visuals, and reuse the scene and layer projection");
                const beforeZoom = await state(page);
                await page.keyboard.press("2");
                await waitForNativeFrame(page, beforeZoom.renderedFrames, "Native zoom to selection");
                const zoomed = await state(page);
                assert.ok(zoomed.zoom > beforeZoom.zoom, "Native zoom to selection must narrow the view of the 10,000-layer fixture");
                assert.ok(zoomed.rootCandidates > 0 && zoomed.rootCandidates < 250,
                    "The root spatial index must enumerate only local paint candidates: " + zoomed.rootCandidates);
                assert.ok(zoomed.rootBoundsTests < 1000,
                    "A narrow view must avoid an exhaustive 10,000-root bounds scan: " + zoomed.rootBoundsTests);
                assert.ok(zoomed.visibleNodes > 0 && zoomed.visibleNodes <= zoomed.rootCandidates,
                    "Spatial candidates must include the actual visible leaf layers");
                result.largeDocument.spatialQuery = {
                    roots: zoomed.nodeCount, candidates: zoomed.rootCandidates,
                    boundsTests: zoomed.rootBoundsTests, reused: zoomed.rootQueryReused,
                    visibleNodes: zoomed.visibleNodes, zoom: zoomed.zoom,
                };
                check("Native zoom into 10,000 layers narrows root bounds testing and paint candidates to the local viewport without dropping visible layers");
                const largeSaved = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
                const persistedAt = performance.now();
                const largePersistence = await flushStorage(page);
                assert.equal(largePersistence.saved, true);
                assert.equal(largePersistence.dirty, false);
                result.largeDocument.commitMs = Math.round(performance.now() - persistedAt);
                const largeStored = await committedDocumentInfo(page);
                assert.equal(largeStored.compact, true);
                assert.ok(largeStored.autosaveBytes < largeStored.exportBytes);
                result.largeDocument.autosaveBytes = largeStored.autosaveBytes;
                result.largeDocument.autosaveSerializationMs = largeStored.serializationMs;
                const largeReloadAt = performance.now();
                await page.reload({ waitUntil: "domcontentloaded", timeout: 120000 });
                await waitForReload(page);
                const largeReopened = await page.evaluate(() => globalThis.ferroDesign.ferroDesignExportJson());
                assert.equal(digest(largeReopened), digest(largeSaved), "Every field of the edited 10,000-node design must survive browser reload");
                const restoredLarge = await state(page);
                assert.equal(restoredLarge.nodeCount, 10000);
                assert.equal(restoredLarge.layerRowCount, 10000);
                assert.ok(restoredLarge.realizedLayerRows > 0 && restoredLarge.realizedLayerRows <= 200);
                result.largeDocument.reloadMs = Math.round(performance.now() - largeReloadAt);
                result.largeDocument.exportBytes = Buffer.byteLength(largeSaved);
                result.largeDocument.sha256 = digest(largeSaved);
                check("10,000 rendered layers retain bounded native row realization, real pointer/nudge/undo/redo, and exact durable reload");
                assert.equal(await page.evaluate((json) => globalThis.ferroDesign.ferroDesignImportJson(json), svgJson), true);
                assert.equal((await flushStorage(page)).saved, true);
                await frames(page);
            }
            result.final = await state(page);
            assert.deepEqual(result.pageErrors, [], "No uncaught browser exceptions");
            const panics = await page.evaluate(() => ({
                startup: globalThis.ferroDesignErrors || [],
                render: globalThis.ferroDesignRenderPanics?.() || [],
            }));
            assert.deepEqual(panics, { startup: [], render: [] }, "No startup or rendering panics");
            assert.ok(!result.consoleErrors.some((line) => /panicked|unreachable|RuntimeError|TypeError|ReferenceError|RangeError/i.test(line)), "No runtime errors in the browser console");
            const failedScenarios = result.scenarios.filter(entry => entry.status === "failed");
            result.status = failedScenarios.length ? "failed" : "passed";
            if (failedScenarios.length) {
                failed = true;
                result.error = failedScenarios.map(entry => entry.name + ": " + entry.error).join("\n\n");
            }
        } catch (error) {
            failed = true;
            result.status = "failed";
            const primary = error.stack || String(error);
            const previous = (result.scenarios || []).filter(entry => entry.status === "failed");
            result.error = previous.map(entry => entry.name + ": " + entry.error)
                .concat(previous.some(entry => entry.error === primary) ? [] : [primary]).join("\n\n");
            console.error(mode + ": " + result.error);
            result.failure = await captureWithinDeadline(() => captureFailure(page, mode + "-failure"))
                .catch(captureError => ({ error: String(captureError) }));
            result.failureScreenshot = mode + "-failure.png";
        } finally {
            await context.tracing.stop({ path: resolve(output, mode + "-trace.zip") }).catch(() => {});
            await context.close();
        }
    }
} catch (error) {
    failed = true;
    report.error = error.stack || String(error);
    console.error(error);
} finally {
    await browser?.close();
    await new Promise((done) => server.close(done));
    report.passed = !failed;
    await writeFile(resolve(output, "report.json"), JSON.stringify(report, null, 2));
    const escape = (text) => String(text).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
    const body = report.results.map((item) => "<section><h2>" + escape(item.requestedMode + ": " + item.status) + "</h2><p>Actual surface: " + escape(item.surface?.kind || "not initialized") + "</p><ul>" + item.checks.map((check) => "<li>" + escape(check) + "</li>").join("") + "</ul><img src='" + (item.status === "passed" ? item.requestedMode + "-edited.png" : item.failureScreenshot || item.requestedMode + "-failure.png") + "' alt='Actual browser screenshot'>" + (item.svgImport ? "<h3>Imported SVG in the application</h3><img src='" + item.requestedMode + "-svg.png' alt='SVG imported and rendered by FerroUI'>" : "") + (item.pngExport ? "<h3>Actual PNG download</h3><img class='export' src='" + item.requestedMode + "-export.png' alt='PNG exported by the application'>" : "") + "<pre>" + escape(item.error || JSON.stringify(item, null, 2)) + "</pre></section>").join("");
    const coverage = report.scope.full ? "Full suite requested · Software2D and WebGL2"
        : "Selected coverage requested · modes: " + selectedModes.join(", ") + "; scenarios: " + (selectedScenarios ? [...selectedScenarios].join(", ") : "all");
    await writeFile(resolve(output, "index.html"), "<!doctype html><meta charset='utf-8'><title>FerroDesign browser validation</title><style>body{font:15px system-ui;margin:32px;background:#f6f6f6;color:#222}section{background:white;padding:24px;margin:24px 0;border-radius:12px}img{width:100%;border:1px solid #ddd}pre{overflow:auto;font-size:12px;white-space:pre-wrap}</style><h1>FerroDesign browser validation</h1><p><strong>" + escape(coverage) + "</strong></p><p>Chromium " + escape(report.browser) + ". Each explicit rendering mode must match its observed canvas context. This harness uses SwiftShader for WebGL2 and does not measure hardware-GPU throughput.</p>" + body);
    console.log(JSON.stringify({ passed: report.passed, scope: report.scope, browser: report.browser, results: report.results.map(({requestedMode,status,surface,checks,scenarios = []}) => ({requestedMode,status,surface,checks,
        scenarios: scenarios.map(({name,status,ran,checks,failureStage,error,durationMs}) => ({name,status,ran,checks,failureStage,error,durationMs}))})) }, null, 2));
}
if (failed) process.exitCode = 1;
