// End-to-end test of a browser site in headless Chrome.
//
//   node scripts/browser/test-site.mjs [<site directory>] [--screenshot <file.png>]
//                                      (CHROME=<path of a Chrome or Chromium binary>)
//
// The site directory defaults to target/browser/control-catalog-browser, as written by
// `scripts/build-browser.sh control-catalog-browser`. The test serves it from a local web server and,
// once for WebGL2 (`?RenderingMode=WebGL2`) and once for the 2D canvas (`?RenderingMode=Software2D`):
//
// - loads the page and waits until the application has started: the splash screen is closed and the
//   view has a canvas;
// - checks that the canvas has the context of the mode (a WebGL2 context, or a 2D context);
// - waits until the main view is drawn: the canvas is not blank;
// - enlarges the page and checks that the canvas follows in device pixels and that the area the
//   larger size adds is drawn (the view is laid out again, not only stretched or clipped), then makes
//   it smaller and checks the size of the canvas again;
// - fails when the page logs an error to the console, throws an uncaught exception or fails to load
//   a resource.
//
// For the ControlCatalog site it also checks that no file of the site carries a brand asset of the
// upstream project that samples/ControlCatalog/PlaceholderAssets replaces.
//
// With --screenshot the picture of the WebGL2 run at the first size is written to the given file.
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import zlib from "node:zlib";
import { fileURLToPath } from "node:url";
import { launchChrome, sleep } from "./chrome.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
let screenshotFile;
const positional = [];
for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--screenshot") screenshotFile = path.resolve(argv[++i]);
    else positional.push(argv[i]);
}
const site = path.resolve(positional[0] ?? path.join(root, "target", "browser", "control-catalog-browser"));
if (!fs.existsSync(path.join(site, "index.html"))) {
    console.error(`no site at ${site}: build it with scripts/build-browser.sh control-catalog-browser`);
    process.exit(2);
}

// Start-up includes compiling the WebAssembly module, which takes a while on a cold CI runner.
const START_TIMEOUT = 180_000;
const FRAME_TIMEOUT = 60_000;
const FIRST_SIZE = { width: 1024, height: 700 };
const LARGER_SIZE = { width: 1440, height: 900 };
const SMALLER_SIZE = { width: 800, height: 600 };

const types = {
    ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css",
    ".wasm": "application/wasm", ".svg": "image/svg+xml", ".png": "image/png", ".ico": "image/x-icon",
    ".json": "application/json", ".map": "application/json"
};
const server = http.createServer((request, response) => {
    const name = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
    const file = path.join(site, name.endsWith("/") ? `${name}index.html` : name);
    if (!file.startsWith(site + path.sep) || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
        response.statusCode = 404;
        response.end();
        return;
    }
    response.setHeader("content-type", types[path.extname(file)] ?? "application/octet-stream");
    response.setHeader("cache-control", "no-store");
    fs.createReadStream(file).pipe(response);
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;

/** Decodes a PNG screenshot (8-bit RGB or RGBA, not interlaced) into `{ width, height, pixel(x, y) }`. */
function decodePng(buffer) {
    let offset = 8; let width = 0; let height = 0; let channels = 4; const data = [];
    while (offset < buffer.length) {
        const length = buffer.readUInt32BE(offset);
        const type = buffer.toString("latin1", offset + 4, offset + 8);
        const body = buffer.subarray(offset + 8, offset + 8 + length);
        if (type === "IHDR") { width = body.readUInt32BE(0); height = body.readUInt32BE(4); channels = body[9] === 6 ? 4 : 3; }
        if (type === "IDAT") data.push(body);
        offset += 12 + length;
    }
    const raw = zlib.inflateSync(Buffer.concat(data));
    const stride = width * channels;
    const pixels = Buffer.alloc(stride * height);
    for (let y = 0; y < height; y++) {
        const filter = raw[y * (stride + 1)];
        for (let x = 0; x < stride; x++) {
            const value = raw[y * (stride + 1) + 1 + x];
            const a = x >= channels ? pixels[y * stride + x - channels] : 0;
            const b = y > 0 ? pixels[(y - 1) * stride + x] : 0;
            const c = x >= channels && y > 0 ? pixels[(y - 1) * stride + x - channels] : 0;
            let predicted = 0;
            if (filter === 1) predicted = a;
            else if (filter === 2) predicted = b;
            else if (filter === 3) predicted = (a + b) >> 1;
            else if (filter === 4) {
                const p = a + b - c; const pa = Math.abs(p - a); const pb = Math.abs(p - b); const pc = Math.abs(p - c);
                predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
            }
            pixels[y * stride + x] = (value + predicted) & 255;
        }
    }
    return {
        width,
        height,
        pixel: (x, y) => (pixels[y * stride + x * channels] << 16) | (pixels[y * stride + x * channels + 1] << 8) | pixels[y * stride + x * channels + 2]
    };
}

/** The number of distinct colours in a region of a decoded picture. */
function distinctColours(picture, { x = 0, y = 0, width = picture.width - x, height = picture.height - y } = {}) {
    const colours = new Set();
    for (let row = y; row < y + height; row++) {
        for (let column = x; column < x + width; column++) colours.add(picture.pixel(column, row));
    }
    return colours.size;
}

const failures = [];
const check = (condition, message) => {
    console.log(`${condition ? "ok  " : "FAIL"} ${message}`);
    if (!condition) failures.push(message);
    return condition;
};

const canvasState = `(() => {
    const canvas = document.querySelector("#out canvas");
    const splash = document.querySelector("#out .ferroui-splash");
    return JSON.stringify({
        canvas: canvas ? { width: canvas.width, height: canvas.height } : null,
        splashClosed: !splash || splash.classList.contains("splash-close"),
        dpr: globalThis.devicePixelRatio
    });
})()`;

// Asking a canvas for a context of another kind than the one it has gives null, so this tells the
// kind without creating a context (the canvas has one by the time the view has drawn).
const contextKind = `(() => {
    const canvas = document.querySelector("#out canvas");
    if (!canvas) return "none";
    if (canvas.getContext("webgl2")) return "webgl2";
    if (canvas.getContext("webgl")) return "webgl";
    if (canvas.getContext("2d")) return "2d";
    return "unknown";
})()`;

async function runMode(chrome, mode, expectedContext, screenshot) {
    console.log(`== ${mode}`);
    const page = await chrome.openPage();
    const errors = [];
    page.on((method, params) => {
        if (method === "Runtime.consoleAPICalled" && (params.type === "error" || params.type === "assert")) {
            errors.push(`console.${params.type}: ${params.args.map((a) => a.value ?? a.description ?? a.type).join(" ")}`);
        } else if (method === "Runtime.exceptionThrown") {
            errors.push(`uncaught: ${params.exceptionDetails.exception?.description ?? params.exceptionDetails.text}`);
        } else if (method === "Log.entryAdded" && params.entry.level === "error") {
            errors.push(`log: ${params.entry.text}${params.entry.url ? ` (${params.entry.url})` : ""}`);
        } else if (method === "Runtime.consoleAPICalled" && process.env.FERROUI_TEST_VERBOSE) {
            console.log(`  console.${params.type}: ${params.args.map((a) => a.value ?? a.description).join(" ")}`);
        }
    });
    await page.send("Runtime.enable");
    await page.send("Log.enable");
    await page.send("Page.enable");
    const resize = ({ width, height }) =>
        page.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 0, mobile: false });
    await resize(FIRST_SIZE);

    const started = Date.now();
    await page.send("Page.navigate", { url: `${origin}/index.html?RenderingMode=${mode}` });
    const up = await page.waitFor(`(() => { const s = JSON.parse(${canvasState}); return s.splashClosed && s.canvas && s.canvas.width > 0; })()`, START_TIMEOUT, 250);
    if (!check(up && errors.length === 0, `${mode}: the application starts (${((Date.now() - started) / 1000).toFixed(1)} s)`)) {
        for (const error of errors) console.log(`  ${error}`);
        page.close();
        return;
    }

    /** Waits until the canvas has the device-pixel size of `size` and the picture fills the region. */
    async function waitForFrame(size, region, minimumColours) {
        let state; let picture; let colours = 0;
        for (const end = Date.now() + FRAME_TIMEOUT; Date.now() < end;) {
            state = JSON.parse(await page.evaluate(canvasState));
            const wanted = { width: Math.round(size.width * state.dpr), height: Math.round(size.height * state.dpr) };
            if (state.canvas && state.canvas.width === wanted.width && state.canvas.height === wanted.height) {
                const png = await page.screenshot({ x: 0, y: 0, ...size });
                picture = decodePng(png);
                colours = distinctColours(picture, region);
                if (colours >= minimumColours) return { state, png, picture, colours, ok: true };
            }
            await sleep(250);
        }
        return { state, picture, colours, ok: false };
    }

    const first = await waitForFrame(FIRST_SIZE, undefined, 64);
    check(first.ok, `${mode}: the main view is drawn at ${FIRST_SIZE.width}x${FIRST_SIZE.height} (canvas ${first.state?.canvas?.width}x${first.state?.canvas?.height}, ${first.colours} colours)`);
    const kind = await page.evaluate(contextKind);
    check(kind === expectedContext, `${mode}: the canvas has a ${expectedContext} context (found ${kind})`);
    if (screenshot && first.ok) {
        fs.mkdirSync(path.dirname(screenshot), { recursive: true });
        fs.writeFileSync(screenshot, first.png);
        console.log(`  screenshot written to ${path.relative(root, screenshot)}`);
    }

    // The strip that only the larger size has, below the top and to the right of the first width.
    await resize(LARGER_SIZE);
    const added = { x: FIRST_SIZE.width + 8, y: 40, width: LARGER_SIZE.width - FIRST_SIZE.width - 16, height: LARGER_SIZE.height - 80 };
    const larger = await waitForFrame(LARGER_SIZE, added, 4);
    check(larger.ok, `${mode}: the view is laid out again at ${LARGER_SIZE.width}x${LARGER_SIZE.height} (canvas ${larger.state?.canvas?.width}x${larger.state?.canvas?.height}, ${larger.colours} colours in the added area)`);

    await resize(SMALLER_SIZE);
    const smaller = await waitForFrame(SMALLER_SIZE, undefined, 64);
    check(smaller.ok, `${mode}: the view is laid out again at ${SMALLER_SIZE.width}x${SMALLER_SIZE.height} (canvas ${smaller.state?.canvas?.width}x${smaller.state?.canvas?.height}, ${smaller.colours} colours)`);

    // Let a few more frames run so that errors of the render loop show up.
    await sleep(1000);
    check(errors.length === 0, `${mode}: nothing is logged as an error`);
    for (const error of errors) console.log(`  ${error}`);
    page.close();
}

/**
 * The site must not carry the brand assets of the upstream project that samples/ControlCatalog/PlaceholderAssets
 * replaces: neither the bytes of a replaced asset nor the path data of a replaced geometry resource may
 * appear in a file of the site.
 */
function checkPlaceholderBranding() {
    const catalog = path.join(root, "samples", "ControlCatalog");
    const placeholders = path.join(catalog, "PlaceholderAssets");
    const published = fs.readdirSync(site, { recursive: true }).map((name) => path.join(site, name))
        .filter((file) => fs.statSync(file).isFile()).map((file) => fs.readFileSync(file));
    const shipped = (needle) => published.some((content) => content.indexOf(needle) >= 0);

    for (const name of fs.readdirSync(placeholders)) {
        const original = path.join(catalog, "Assets", name);
        if (!fs.existsSync(original) || !fs.statSync(original).isFile()) continue;
        // A run of bytes from the middle of the file: long enough to be unique to it.
        const bytes = fs.readFileSync(original);
        const length = Math.min(256, Math.floor(bytes.length / 2));
        const middle = Math.floor((bytes.length - length) / 2);
        check(!shipped(bytes.subarray(middle, middle + length)), `the site does not carry Assets/${name}`);
    }

    const geometries = path.join(placeholders, "StreamGeometry");
    const documents = fs.readdirSync(catalog, { recursive: true }).filter((name) => name.endsWith(".xaml"))
        .map((name) => fs.readFileSync(path.join(catalog, name), "utf8"));
    for (const file of fs.readdirSync(geometries).filter((name) => name.endsWith(".txt"))) {
        const key = file.slice(0, -".txt".length);
        const start = `<StreamGeometry x:Key="${key}">`;
        for (const document of documents) {
            let at = document.indexOf(start);
            while (at >= 0) {
                const data = document.slice(at + start.length, document.indexOf("</StreamGeometry>", at)).trim();
                check(!shipped(Buffer.from(data)), `the site does not carry the original path data of the resource ${key}`);
                at = document.indexOf(start, at + 1);
            }
        }
    }
}

let chrome;
try {
    if (path.basename(site) === "control-catalog-browser") checkPlaceholderBranding();
    chrome = await launchChrome({ width: LARGER_SIZE.width, height: LARGER_SIZE.height });
    console.log(`${await chrome.version()} (${chrome.binary}), site ${path.relative(root, site) || site}`);
    await runMode(chrome, "WebGL2", "webgl2", screenshotFile);
    await runMode(chrome, "Software2D", "2d");
} catch (error) {
    check(false, `the test ran to the end: ${error.stack ?? error}`);
} finally {
    await chrome?.close();
    server.close();
}
console.log(failures.length === 0 ? "all checks passed" : `${failures.length} check(s) failed`);
process.exit(failures.length === 0 ? 0 : 1);
