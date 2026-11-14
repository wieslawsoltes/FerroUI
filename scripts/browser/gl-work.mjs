// Counts the WebGL work of a page: what a frame asks of the graphics driver says what the frame
// costs wherever it runs, and does not move with the machine the way a time does.
//
// `GL_WORK_PROBE` is an init script (`open(.., { initScript: GL_WORK_PROBE })`): it counts the calls
// of every WebGL2 context of the thread of the page from the load of the page on, in
// `globalThis.__ferroGlWork`. The contexts of a worker are not counted: a module built with threads
// is measured with the frames drawn on the thread of the page (`RenderThread=false`).
//
// - `programs`: programs linked;
// - `draws`: draw calls;
// - `textures`, `texturePixels`: textures allocated or uploaded, and their pixels;
// - `masks`, `maskPixels`: those of them with one channel. Skia uploads the coverage of what it
//   rasterises on the processor that way: the masks of paths and of clips that are not rectangles,
//   and the glyphs of text when they are new. A clip that is not a rectangle costs a mask for every
//   draw under it (docs/porting/desktop-performance.md, section 9.56);
// - `framebufferBinds`, `clears`: changes of the render target, and clears.

const COUNTS = ["programs", "draws", "textures", "texturePixels", "masks", "maskPixels", "framebufferBinds", "clears"];

export const GL_WORK_PROBE = `(() => {
    const work = globalThis.__ferroGlWork = { ${COUNTS.map((name) => `${name}: 0`).join(", ")} };
    const prototype = globalThis.WebGL2RenderingContext?.prototype;
    if (!prototype) { return; }
    const wrap = (name, count) => {
        const original = prototype[name];
        prototype[name] = function (...a) { count(a); return original.apply(this, a); };
    };
    // R8, RED, ALPHA and LUMINANCE.
    const oneChannel = (format) => format === 0x8229 || format === 0x1903 || format === 0x1906 || format === 0x1909;
    const texture = (format, width, height) => {
        const pixels = Number.isFinite(width * height) ? width * height : 0;
        work.textures++; work.texturePixels += pixels;
        if (oneChannel(format)) { work.masks++; work.maskPixels += pixels; }
    };
    wrap("linkProgram", () => { work.programs++; });
    for (const name of ["drawArrays", "drawElements", "drawArraysInstanced", "drawElementsInstanced", "drawRangeElements"]) {
        wrap(name, () => { work.draws++; });
    }
    // texImage2D(target, level, internalformat, width, height, border, format, type, source), or with
    // an element as the source and no size; texSubImage2D(target, level, x, y, width, height, format,
    // type, source), or with an element and no size; texStorage2D(target, levels, internalformat,
    // width, height).
    wrap("texImage2D", (a) => (a.length >= 9 ? texture(a[2], a[3], a[4]) : texture(a[2], 0, 0)));
    wrap("texSubImage2D", (a) => (a.length >= 9 ? texture(a[6], a[4], a[5]) : texture(a[4], 0, 0)));
    wrap("texStorage2D", (a) => texture(a[2], a[3], a[4]));
    wrap("bindFramebuffer", () => { work.framebufferBinds++; });
    wrap("clear", () => { work.clears++; });
})()`;

/** Starts the counts of the page over. */
export async function resetGlWork(page) {
    await page.evaluate(`(() => { for (const name of Object.keys(__ferroGlWork)) { __ferroGlWork[name] = 0; } })()`);
}

/** The counts of the page since its load, or since they were started over. */
export async function readGlWork(page) {
    return JSON.parse(await page.evaluate("JSON.stringify(globalThis.__ferroGlWork ?? null)"));
}

export const GL_WORK_COUNTS = COUNTS;
