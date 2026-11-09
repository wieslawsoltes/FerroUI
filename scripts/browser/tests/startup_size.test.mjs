// Inspect the pixels of the very first software frame, before a later frame can hide a DPI
// startup regression. Run against themed_view built from this checkout, with or without threads:
//   node scripts/browser/tests/startup_size.test.mjs <site directory>
import path from "node:path";
import fs from "node:fs";
import { open, run, assert } from "../harness.mjs";

const site = path.resolve(process.argv[2] ?? "target/browser/themed_view");
const threaded = fs.existsSync(path.join(site, "ferroui-threads.js"));
const probe = `
globalThis.startupFrames = [];
import("/ferroui.js").then(({ CanvasSurface, SoftwareRenderTarget, FerroExports }) => {
    const create = CanvasSurface.create;
    CanvasSurface.create = function (...args) {
        const surface = create.apply(this, args);
        globalThis.startupSize = [surface.width, surface.height, surface.scaling];
        return surface;
    };
    const put = SoftwareRenderTarget.staticPutPixelData;
    SoftwareRenderTarget.staticPutPixelData = function (target, pointer, length, width, height) {
        if (globalThis.startupFrames.length < 8) {
            const x = Math.floor(width * 0.9), y = Math.floor(height * 0.9);
            const pixels = FerroExports.heapU8();
            globalThis.startupFrames.push({ width, height,
                corner: Array.from(pixels.subarray(pointer + (y * width + x) * 4, pointer + (y * width + x) * 4 + 4)) });
        }
        return put.call(this, target, pointer, length, width, height);
    };
});`;

await run([1, 1.25, 2].map(scale => [`the first frame fills the view at ${scale}x DPI`, async () => {
    // Keep rendering on the page so every frame can be inspected synchronously, even when the
    // module supports threads. BrowserSurface's notification order is shared by both modes.
    const page = await open(site, { width: 800, height: 600, scale, isolated: threaded,
        query: "?RenderingMode=Software2D&RenderThread=false", initScript: probe });
    try {
        let previousOrigin = 0;
        for (const load of ["navigation", "refresh"]) {
            if (load === "refresh") await page.send("Page.reload");
            await page.waitFor(`performance.timeOrigin !== ${previousOrigin} && globalThis.startupFrames?.length >= 1`);
            const result = await page.evaluate(`({ initial: startupSize, frames: startupFrames,
                origin: performance.timeOrigin, dpr: devicePixelRatio,
                css: [getComputedStyle(document.querySelector(".ferroui-canvas")).width,
                    getComputedStyle(document.querySelector(".ferroui-canvas")).height] })`);
            previousOrigin = result.origin;
            assert(page.errors.length === 0, page.errors.join("\n"));
            const expected = result.css.map(value => Math.round(parseFloat(value) * result.dpr));
            assert(result.initial.join() === [...expected, result.dpr].join(),
                `${load}: initial surface ${result.initial}; expected ${[...expected, result.dpr]}; frames ${JSON.stringify(result.frames)}`);
            const first = result.frames[0];
            assert(first.width === expected[0] && first.height === expected[1],
                `${load}: first frame ${first.width}x${first.height}; expected ${expected.join("x")}`);
            assert(first.corner[3] === 255, `${load}: the first frame leaves the far corner transparent: ${JSON.stringify(result)}`);
            assert(result.frames.every(frame => frame.corner[3] === 255),
                `${load}: a startup frame leaves the far corner transparent: ${JSON.stringify(result)}`);
        }
    } finally { await page.close(); }
}]));
