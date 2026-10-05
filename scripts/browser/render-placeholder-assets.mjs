// Renders the neutral placeholder artwork of the ControlCatalog sample.
//
//   node scripts/browser/render-placeholder-assets.mjs       (CHROME=<path of a Chrome or Chromium binary>)
//
// The published browser site of the sample must not carry the brand assets of the upstream project. With
// the feature `placeholder-branding` of the crate `control-catalog` (selected by the browser host,
// samples/ControlCatalog.Browser) the build script of the sample embeds the file of the same name from
// samples/ControlCatalog/PlaceholderAssets in place of each such asset. This script draws those files: the
// placeholder mark of samples/ControlCatalog.Browser/wwwroot/favicon.svg (a plain letter on a rounded
// square, original artwork), a word mark, a neutral banner background and four plain shapes, each at the
// pixel size of the asset it replaces, rasterised by headless Chrome. The output is committed; run the
// script again only when the artwork changes.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchChrome } from "./chrome.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const out = path.join(root, "samples", "ControlCatalog", "PlaceholderAssets");
const mark = fs.readFileSync(path.join(root, "samples", "ControlCatalog.Browser", "wwwroot", "favicon.svg"), "utf8");
const markBody = mark.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");

const svg = (width, height, body, viewBox = `0 0 ${width} ${height}`) =>
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${viewBox}">${body}</svg>`;
const shape = (size, body) => svg(size, size, body, "0 0 100 100");

// [file, width, height, svg]
const images = [
    ["icon-32.png", 32, 32, svg(32, 32, markBody, "0 0 64 64")],
    ["banner-logo.png", 600, 120, svg(600, 120,
        `<g transform="translate(24 12) scale(1.5)">${markBody}</g>` +
        `<text x="150" y="84" font-family="Helvetica, Arial, sans-serif" font-size="68" font-weight="600" fill="#ffffff">FerroUI</text>`)],
    ["banner-bg.png", 900, 600, svg(900, 600,
        `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">` +
        `<stop offset="0" stop-color="#2f3b4c"/><stop offset="0.55" stop-color="#46556b"/><stop offset="1" stop-color="#5b6b82"/>` +
        `</linearGradient></defs><rect width="900" height="600" fill="url(#g)"/>`)],
    ["logo1.png", 456, 456, shape(456, `<circle cx="50" cy="50" r="38" fill="#ffffff" fill-opacity="0.16"/>`)],
    ["logo2.png", 419, 419, shape(419,
        `<rect x="18" y="18" width="64" height="64" rx="14" transform="rotate(15 50 50)" fill="#ffffff" fill-opacity="0.2"/>`)],
    ["logo3.png", 405, 405, shape(405, `<circle cx="50" cy="50" r="32" fill="none" stroke="#ffffff" stroke-opacity="0.22" stroke-width="10"/>`)],
    ["logo4.png", 480, 480, shape(480,
        `<path d="M50 14 L86 78 Q88 84 82 84 L18 84 Q12 84 14 78 Z" fill="#ffffff" fill-opacity="0.15"/>`)]
];
// The sizes of the application icon, each a PNG image inside the icon file.
const iconSizes = [16, 32, 48, 256];

const chrome = await launchChrome({ width: 1000, height: 700 });
const page = await chrome.openPage();
await page.send("Page.enable");
await page.send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });

async function render(content, width, height) {
    const html = `<!doctype html><html><body style="margin:0;background:transparent">${content}</body></html>`;
    await page.send("Page.navigate", { url: `data:text/html;base64,${Buffer.from(html).toString("base64")}` });
    await page.waitFor("document.readyState === 'complete'", 10000);
    return page.screenshot({ x: 0, y: 0, width, height });
}

fs.mkdirSync(out, { recursive: true });
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
    page.close();
    await chrome.close();
}
