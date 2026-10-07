// Renders the artwork of the project that the ControlCatalog sample shows.
//
//   node scripts/browser/render-placeholder-assets.mjs       (CHROME=<path of a Chrome or Chromium binary>)
//
// The sample must not show the brand assets of the upstream project, and its published browser site must
// not carry them. With the feature `placeholder-branding` of the crate `control-catalog` (on by default;
// the browser host samples/ControlCatalog.Browser names it too) the build script of the sample embeds the file of the same name from
// samples/ControlCatalog/PlaceholderAssets in place of each such asset. This script draws those files: the
// mark of the project in samples/ControlCatalog.Browser/wwwroot/favicon.svg (the letter F and a spark on a
// rounded square, original artwork), a word mark, a banner background and four shapes, each at the
// pixel size of the asset it replaces, rasterised by headless Chrome. The output is committed; run the
// script again only when the artwork changes.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { open } from "./harness.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const out = path.join(root, "samples", "ControlCatalog", "PlaceholderAssets");
const mark = fs.readFileSync(path.join(root, "samples", "ControlCatalog.Browser", "wwwroot", "favicon.svg"), "utf8");
const markBody = mark.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");

const svg = (width, height, body, viewBox = `0 0 ${width} ${height}`) =>
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${viewBox}">${body}</svg>`;
const shape = (size, body) => svg(size, size, body, "0 0 100 100");

// The palette: the dark blue of cold steel, and the orange of the mark (iron at forging heat).
const ember = (id) => `<linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="#ffc163"/><stop offset="0.5" stop-color="#f4601f"/><stop offset="1" stop-color="#b3261e"/></linearGradient>`;
const steel = (id) => `<linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="#9cc4ff"/><stop offset="0.5" stop-color="#3f7fe0"/><stop offset="1" stop-color="#173a7a"/></linearGradient>`;
// A highlight along the upper left edge of a shape, which makes it read as a solid.
const sheen = (id) => `<linearGradient id="${id}" x1="0" y1="0" x2="0.7" y2="1">` +
    `<stop offset="0" stop-color="#ffffff" stop-opacity="0.55"/><stop offset="0.45" stop-color="#ffffff" stop-opacity="0"/></linearGradient>`;
const shadow = `<filter id="s" x="-30%" y="-30%" width="160%" height="160%"><feDropShadow dx="0" dy="3" stdDeviation="3" flood-color="#050a14" flood-opacity="0.45"/></filter>`;
const solid = (size, fill, d, extra = "") => shape(size,
    `<defs>${fill("f")}${sheen("h")}${shadow}</defs><g filter="url(#s)" opacity="0.92">` +
    `<path d="${d}" fill="url(#f)" ${extra}/><path d="${d}" fill="url(#h)" ${extra}/></g>`);

// [file, width, height, svg]
const images = [
    ["icon-32.png", 32, 32, svg(32, 32, markBody, "0 0 64 64")],
    ["banner-logo.png", 600, 120, svg(600, 120,
        `<g transform="translate(34 12) scale(1.5)">${markBody}</g>` +
        `<text x="158" y="86" font-family="Helvetica Neue, Helvetica, Arial, sans-serif" font-size="76" font-weight="700" letter-spacing="-2" fill="#ffffff">` +
        `Ferro<tspan fill="#ffb347">UI</tspan></text>`)],
    // The home page shows a band across the middle of the picture (it fills a wide, low banner), so the
    // picture has no detail that a crop would cut through: glows, and the sheen of rolled steel.
    ["banner-bg.png", 900, 600, svg(900, 600,
        `<defs>` +
        `<linearGradient id="base" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0a1222"/>` +
        `<stop offset="0.5" stop-color="#12264a"/><stop offset="1" stop-color="#1b3a70"/></linearGradient>` +
        `<radialGradient id="heat" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#ff7a2a" stop-opacity="0.75"/>` +
        `<stop offset="0.45" stop-color="#e2451c" stop-opacity="0.3"/><stop offset="1" stop-color="#e2451c" stop-opacity="0"/></radialGradient>` +
        `<radialGradient id="cool" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#4f9bff" stop-opacity="0.6"/>` +
        `<stop offset="1" stop-color="#4f9bff" stop-opacity="0"/></radialGradient>` +
        `<linearGradient id="streak" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ffffff" stop-opacity="0"/>` +
        `<stop offset="0.5" stop-color="#ffffff" stop-opacity="0.09"/><stop offset="1" stop-color="#ffffff" stop-opacity="0"/></linearGradient>` +
        `</defs>` +
        `<rect width="900" height="600" fill="url(#base)"/>` +
        `<ellipse cx="170" cy="380" rx="430" ry="330" fill="url(#heat)"/>` +
        `<ellipse cx="780" cy="230" rx="380" ry="300" fill="url(#cool)"/>` +
        `<ellipse cx="930" cy="560" rx="300" ry="220" fill="url(#heat)" opacity="0.55"/>` +
        `<g transform="rotate(-24 450 300)">` +
        [[-160, 150], [40, 60], [210, 190], [430, 90], [560, 240], [760, 120]].map(([x, width]) =>
            `<rect x="${x}" y="-300" width="${width}" height="1200" fill="url(#streak)"/>`).join("") +
        `</g>`)],
    // The shapes that float on the banner: a crystal of iron, a plate, a ring and a spark.
    ["logo1.png", 456, 456, solid(456, ember, "M50 8 L86 29 V71 L50 92 L14 71 V29 Z", `stroke-linejoin="round"`)],
    ["logo2.png", 419, 419, solid(419, steel,
        "M30 14 H70 A16 16 0 0 1 86 30 V70 A16 16 0 0 1 70 86 H30 A16 16 0 0 1 14 70 V30 A16 16 0 0 1 30 14 Z", `transform="rotate(14 50 50)"`)],
    ["logo3.png", 405, 405, solid(405, steel,
        "M50 10 A40 40 0 1 1 49.99 10 Z M50 31 A19 19 0 1 0 50.01 31 Z", `fill-rule="evenodd"`)],
    ["logo4.png", 480, 480, solid(480, ember, "M50 6 Q55 45 94 50 Q55 55 50 94 Q45 55 6 50 Q45 45 50 6 Z")]
];
// The sizes of the application icon, each a PNG image inside the icon file.
const iconSizes = [16, 32, 48, 256];

fs.mkdirSync(out, { recursive: true });
// The page of the harness serves the output directory; every picture is drawn from a data address.
const page = await open(out, { width: 1000, height: 700 });
await page.send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });

async function render(content, width, height) {
    const html = `<!doctype html><html><body style="margin:0;background:transparent">${content}</body></html>`;
    await page.send("Page.navigate", { url: `data:text/html;base64,${Buffer.from(html).toString("base64")}` });
    await page.waitFor("document.readyState === 'complete'", 10000);
    return (await page.screenshot(undefined, { x: 0, y: 0, width, height })).png;
}

try {
    for (const [file, width, height, content] of images) {
        fs.writeFileSync(path.join(out, file), await render(content, width, height));
        console.log(`${file} ${width}x${height}`);
    }

    // An icon file whose entries are PNG images (supported by every reader since Windows Vista).
    const entries = [];
    for (const size of iconSizes) entries.push([size, await render(svg(size, size, markBody, "0 0 64 64"), size, size)]);
    const header = Buffer.alloc(6 + 16 * entries.length);
    header.writeUInt16LE(0, 0);
    header.writeUInt16LE(1, 2);
    header.writeUInt16LE(entries.length, 4);
    let offset = header.length;
    entries.forEach(([size, png], i) => {
        const at = 6 + 16 * i;
        header.writeUInt8(size >= 256 ? 0 : size, at);
        header.writeUInt8(size >= 256 ? 0 : size, at + 1);
        header.writeUInt16LE(1, at + 4);
        header.writeUInt16LE(32, at + 6);
        header.writeUInt32LE(png.length, at + 8);
        header.writeUInt32LE(offset, at + 12);
        offset += png.length;
    });
    fs.writeFileSync(path.join(out, "icon.ico"), Buffer.concat([header, ...entries.map(([, png]) => png)]));
    console.log(`icon.ico ${iconSizes.join(", ")}`);
} finally {
    await page.close();
}
