// Exercise initial content-box measurement against the browser's real ResizeObserver, including
// canvases transferred offscreen, without needing a WebAssembly build.
//   node scripts/browser/tests/canvas_size.test.mjs [<directory containing ferroui.js>]
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open, run, assert } from "../harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const bundle = path.resolve(process.argv[2] ?? path.join(root, "src/Browser/FerroUI.Browser/dist"), "ferroui.js");
const site = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-canvas-size-"));
process.on("exit", () => fs.rmSync(site, { recursive: true, force: true }));
fs.copyFileSync(bundle, path.join(site, "ferroui.js"));
fs.writeFileSync(path.join(site, "index.html"), `<!doctype html>
<script type="module">
import { CanvasSurface, FerroExports } from "./ferroui.js";
globalThis.sizes = [];
FerroExports.attach({ CanvasHelper_OnSizeChanged(id, ...size) { sizes[id].observed = size; } });
for (const [name, css, offscreen] of [
    ["fractional", "", false],
    ["border-box", "canvas { box-sizing: border-box; padding: 4px; border: 3px solid; }", false],
    ["transformed", "canvas { transform: scale(0.5); }", false],
    ["hidden", "div { display: none; }", false],
    ["offscreen", "", true]
]) {
    const host = document.createElement("div");
    host.style.cssText = "position:relative;width:400.25px;height:300.5px";
    // Each fixture's stylesheet is scoped by its own shadow root.
    const fixture = document.createElement("section");
    document.body.append(fixture);
    const shadow = fixture.attachShadow({ mode: "open" });
    const style = document.createElement("style"); style.textContent = css;
    shadow.append(style, host);
    const id = sizes.length;
    const surface = CanvasSurface.create(host, [1], id, offscreen ? -1 : 0);
    sizes.push({ name, initial: [surface.width, surface.height, surface.scaling] });
}
globalThis.synchronousReports = sizes.filter(size => size.observed).length;
</script>`);

await run([1, 1.25, 2].map(scale => [`initial canvas size agrees with ResizeObserver at ${scale}x DPI`, async () => {
    const page = await open(site, { scale });
    try {
        await page.waitFor("globalThis.sizes?.length === 5 && sizes.every(size => size.observed)");
        const result = await page.evaluate("({ sizes, synchronousReports, dpr: devicePixelRatio })");
        assert(page.errors.length === 0, page.errors.join("\n"));
        assert(result.synchronousReports === 0, "construction called back before the framework could finish creating its surface");
        for (const { name, initial, observed } of result.sizes) {
            assert(initial[2] === result.dpr, `${name}: initial scaling ${initial[2]}, expected ${result.dpr}`);
            // Rounding the CSS extent may differ by one pixel from the browser's snapped device box.
            assert(initial.slice(0, 2).every((value, i) => Math.abs(value - observed[i]) <= 1),
                `${name}: initial ${initial}, observed ${observed}`);
            if (name === "hidden") assert(initial[0] === 0 && initial[1] === 0, `hidden canvas: ${initial}`);
        }
    } finally { await page.close(); }
}]));
