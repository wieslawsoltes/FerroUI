// Real-browser acceptance checks of the linked Rust editor application.
// Build: scripts/build-browser.sh editor-demo-browser
// Run: node scripts/browser/tests/editor_demo.test.mjs [site] [--screenshot file.png]
// The only application exports called are startup and read-only diagnostics.
// Every edit and command below comes from native browser pointer/key/input events.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const args = process.argv.slice(2);
const screenshotIndex = args.indexOf("--screenshot");
const screenshot = screenshotIndex < 0 ? null : args.splice(screenshotIndex, 2)[1];
const site = path.resolve(args[0] ?? path.join(process.env.CARGO_TARGET_DIR ?? path.join(root, "target"), "browser/editor-demo-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    throw new Error(`No browser site at ${site}; run scripts/build-browser.sh editor-demo-browser`);
}
const stateExpression = "JSON.parse(globalThis.editorDemo.editorState())";
const state = (page) => page.evaluate(stateExpression);
async function settle(page) {
    assert(await page.evaluate("globalThis.editorDemo.editorRequestFrame()"), "editor has no compositor");
    await page.waitFor("globalThis.editorDemo.editorFrameDrawn()", 60_000);
}
async function waitState(page, predicate) {
    await page.waitFor(`(${predicate})(${stateExpression})`, 30_000);
    await settle(page);
    return state(page);
}
async function clickElement(page, predicate) {
    await settle(page);
    await page.waitFor(`${stateExpression}.elements.some(e => e.hit && (${predicate})(e))`, 30_000);
    const element = await page.evaluate(`${stateExpression}.elements.find(e => e.hit && (${predicate})(e))`);
    await page.click(element.x + element.width / 2, element.y + element.height / 2);
    await settle(page);
}
const toolbar = (page, name) => clickElement(page, `e => e.name === ${JSON.stringify(name)}`);
const focusEditor = (page) => clickElement(page, "e => e.type.endsWith('TextArea')");
async function commitComposition(page, text) {
    const before = (await state(page)).text;
    // The browser platform receives composition events from its hidden native
    // input element. CDP's insertText commits a composition but does not start
    // one: begin it explicitly to exercise the IME client and preedit rendering.
    const start = await page.send("Input.imeSetComposition", {
        text, selectionStart: text.length, selectionEnd: text.length,
    });
    assert(!start.error, `could not start IME composition: ${JSON.stringify(start)}`);
    await settle(page);
    assert((await state(page)).text === before, "IME preedit changed the document before commit");
    const commit = await page.send("Input.insertText", { text });
    assert(!commit.error, `could not commit IME composition: ${JSON.stringify(commit)}`);
}
async function replaceDocument(page, text) {
    await focusEditor(page);
    await page.press("a", ["ctrl"]);
    if (text.length) await commitComposition(page, text);
    else await page.press("Backspace");
    await waitState(page, `s => s.text === ${JSON.stringify(text)}`);
}

const checks = [];
for (const mode of ["Software2D", "WebGL2"]) {
    checks.push([`${mode}: linked editor, real Oniguruma, input, popups, undo, snippets, folding and resize`, async () => {
        const page = await open(site, { query: `?RenderingMode=${mode}`, width: 1180, height: 800, network: true });
        try {
            await page.waitFor("document.documentElement.dataset.editorReady === 'true'", 180_000);
            await page.waitForView();
            const initial = await waitState(page, "s => s && s.frames > 0 && s.tokenCount > 10 && s.cachedLines > 0");
            assert(initial.grammar === "source.rust", `unexpected initial grammar: ${initial.grammar}`);
            assert(initial.font === "Noto Mono", `code font did not resolve: ${initial.font}`);
            assert(initial.grammarError === null && initial.editorError === null, `initial editor error: ${JSON.stringify(initial)}`);
            const picture = await page.screenshot(mode === "Software2D" ? screenshot : undefined);
            const colors = new Set();
            for (let y = 0; y < picture.height; y += 4) {
                for (let x = 0; x < picture.width; x += 4) colors.add(picture.pixel(x, y).join(","));
            }
            assert(colors.size > 30, `editor canvas is blank or unpainted: ${colors.size} colors`);

            // UTF-16 supplementary and combining sequences travel through the
            // real browser IME composition/commit path and the native undo stack.
            const unicode = "// 😀 e\u0301";
            await replaceDocument(page, unicode);
            assert((await state(page)).caret === unicode.length, "caret did not count UTF-16 units");
            await page.press("z", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(initial.text)}`);
            await page.press("y", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(unicode)}`);
            await page.press("Enter");
            await page.type("let browser_smoke = 42;");
            const clipboardText = `${unicode}\nlet browser_smoke = 42;`;
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardText)}`);

            // Native rich copy/cut/paste uses the browser Clipboard API. Read
            // both formats back independently of the editor's own export code.
            await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
            await page.setPermission("clipboard-read", "granted");
            await page.setPermission("clipboard-write", "granted");
            await page.press("a", ["ctrl"]);
            await page.press("c", ["ctrl"]);
            await page.waitFor(`navigator.clipboard.readText().then(text => text === ${JSON.stringify(clipboardText)})`, 15_000);
            const richCopy = await page.evaluate(`navigator.clipboard.read().then(async items => {
                const item = items.find(item => item.types.includes('text/html'));
                return item ? (await item.getType('text/html')).text() : null;
            })`);
            assert(richCopy?.includes("browser_smoke"), "native copy did not publish HTML beside plain text");
            await page.press("x", ["ctrl"]);
            await waitState(page, "s => s.text === ''");
            await page.press("v", ["ctrl"]);
            await waitState(page, `s => s.text === ${JSON.stringify(clipboardText)}`);

            // Completion keeps editor focus; Tab commits the actual native popup.
            await replaceDocument(page, "pri");
            await page.press("Space", ["ctrl"]);
            await waitState(page, "s => s.elements.some(e => e.text === 'println!')");
            await page.press("Tab");
            await waitState(page, `s => s.text === 'println!("Hello, FerroUI!");'`);

            // Linked fields update together and Tab advances to the next field.
            await replaceDocument(page, "");
            await toolbar(page, "Snippet");
            await waitState(page, "s => s.selectedText === 'name'");
            await page.type("browser_name");
            await waitState(page, "s => (s.text.match(/browser_name/g) ?? []).length === 2");
            await page.press("Tab");
            await waitState(page, "s => s.selectedText === 'value'");
            await page.type("123");
            await page.press("Tab");
            await waitState(page, "s => s.text.includes('browser_name = 123;') && s.selectedText === ''");

            // Search uses the actual popup TextBox and routes Escape to the editor.
            await toolbar(page, "RustLanguage");
            await toolbar(page, "Find");
            await waitState(page, "s => s.searchOpen && s.focus.endsWith('TextBox')");
            await page.type("FerroUI");
            await waitState(page, "s => s.searchPattern === 'FerroUI'");
            await page.press("Enter");
            await page.press("Escape");
            await waitState(page, "s => !s.searchOpen && s.selectedText === 'FerroUI'");

            await toolbar(page, "Replace");
            await clickElement(page, "e => e.placeholder === 'Replace'");
            await page.type("BrowserEdit");
            await clickElement(page, "e => e.type.endsWith('Button') && e.text === 'Replace all'");
            await waitState(page, "s => s.text.includes('BrowserEdit') && !s.text.includes('FerroUI')");
            await page.press("Escape");
            await waitState(page, "s => !s.searchOpen");

            // Folding and visual-line regeneration remain valid after changes.
            await toolbar(page, "Fold");
            await waitState(page, "s => s.foldedCount === 2");
            await toolbar(page, "Unfold");
            await waitState(page, "s => s.foldedCount === 0");
            await toolbar(page, "Wrap");
            await waitState(page, "s => s.wordWrap");
            await toolbar(page, "Theme");
            await waitState(page, "s => s.theme && s.theme !== null && s.grammarError === null");
            assert((await state(page)).theme !== initial.theme, "theme did not change");
            await toolbar(page, "CSharpLanguage");
            await waitState(page, "s => s.language === 'CSharp' && s.grammar === 'source.cs' && s.tokenCount > 10");

            await page.resize(800, 620);
            await waitState(page, "s => s.width === 800 && s.height === 620 && s.cachedLines > 0");
            await page.resize(1320, 900);
            await waitState(page, "s => s.width === 1320 && s.height === 900 && s.cachedLines > 0");
            const final = await state(page);
            assert(final.grammarError === null && final.editorError === null && final.renderPanics === 0,
                `editor/renderer errors: ${JSON.stringify(final)}`);
            assert(page.errors.length === 0, page.errors.join("\n"));
            assert(page.requests.filter(url => new URL(url).pathname.endsWith("ferroui-textmate-oniguruma.js")).length === 1,
                "scanner module was missing or downloaded repeatedly");
        } catch (error) {
            error.message += `\n${page.log.slice(-12).join("\n")}`;
            throw error;
        } finally {
            await page.close();
        }
    }]);
}
await run(checks);
