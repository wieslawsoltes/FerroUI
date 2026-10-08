// Pixel test of the software render target in headless Chrome.
//
// The framework hands over frames with premultiplied alpha; the canvas has to show them blended
// with the page behind it. The test puts half-transparent and opaque pixels on a canvas over a
// page with a known background and compares what the browser composites with the expected blend.
//
// The frames are read from the module memory through FerroExports.heapU8(). The page stands in for
// the module with an object that has a buffer, as a WebAssembly.Memory has, and replaces the buffer
// the ways a memory does: by growing (the old buffer is detached, the frame lies beyond its end)
// and, in a module with threads, by growing a shared buffer (the old one stays as it is). The page
// is served cross-origin isolated so that it has shared buffers.
//
//   npm run build && npm run test:pixels        (CHROME=<path of a Chrome or Chromium binary>)
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";

const here = path.dirname(fileURLToPath(import.meta.url));
const bundle = path.join(here, "..", "..", "dist", "ferroui.js");
if (!fs.existsSync(bundle)) { console.error(`missing ${bundle}: run "npm run build" first`); process.exit(2); }

const candidates = [
    process.env.CHROME,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/usr/bin/google-chrome", "/usr/bin/chromium-browser", "/usr/bin/chromium"
].filter((p) => p && fs.existsSync(p));
if (candidates.length === 0) { console.error("no Chrome found: set CHROME"); process.exit(2); }

// Page background: opaque green. Frame: four 20 pixel wide columns, premultiplied RGBA.
const page = `<!doctype html><html><body style="margin:0;background:rgb(0,255,0)">
<canvas id="c" width="80" height="20" style="display:block"></canvas>
<script type="module">
import { SoftwareRenderTarget, FerroExports } from "./ferroui.js";
const columns = [[128, 0, 0, 128], [0, 0, 64, 64], [10, 20, 30, 255], [0, 0, 0, 0]];
const reversed = columns.slice().reverse();
const FRAME = 80 * 20 * 4;
const canvas = document.getElementById("c");
// Writes a frame into the memory at an address, as the module would, through the current view.
const fill = (pattern, pointer) => {
    const heap = FerroExports.heapU8();
    for (let y = 0; y < 20; y++) for (let x = 0; x < 80; x++) heap.set(pattern[Math.floor(x / 20)], pointer + (y * 80 + x) * 4);
};
// One pixel of each column, as the canvas holds it.
const read = () => {
    const data = canvas.getContext("2d").getImageData(0, 0, 80, 20).data;
    return [10, 30, 50, 70].map((x) => Array.from(data.slice(x * 4, x * 4 + 4)));
};

const memory = { buffer: new ArrayBuffer(1024 + FRAME) };
FerroExports.attach({ wasmMemory: memory });
const first = FerroExports.heapU8();
const accessor = { stable: FerroExports.heapU8() === first && first.buffer === memory.buffer };
fill(columns, 1024);
const target = new SoftwareRenderTarget(canvas);
SoftwareRenderTarget.staticPutPixelData(target, 1024, FRAME, 80, 20);
// A second frame through the retained buffer must give the same picture.
SoftwareRenderTarget.staticPutPixelData(target, 1024, FRAME, 80, 20);
globalThis.straight = read();

// The memory grows: a larger buffer with the same content, and the old one is detached. The next
// frame lies beyond the end of the old buffer.
const grownFrame = memory.buffer.byteLength + 4096;
memory.buffer = memory.buffer.transfer(grownFrame + FRAME);
const second = FerroExports.heapU8();
accessor.detached = first.length === 0;
accessor.renewed = second !== first && second.buffer === memory.buffer && second.length === grownFrame + FRAME;
accessor.kept = Array.from(second.subarray(1024, 1028)).join() === columns[0].join();
fill(reversed, grownFrame);
SoftwareRenderTarget.staticPutPixelData(target, grownFrame, FRAME, 80, 20);
globalThis.straightGrown = read();

// The memory of a module with threads: a shared buffer, which is not detached when the memory
// grows but keeps its length, and which ImageData does not take.
accessor.isolated = globalThis.crossOriginIsolated === true;
if (accessor.isolated) {
    memory.buffer = new SharedArrayBuffer(2048);
    const third = FerroExports.heapU8();
    memory.buffer = new SharedArrayBuffer(2048 + FRAME);
    const fourth = FerroExports.heapU8();
    accessor.shared = third.length === 2048 && fourth !== third && fourth.buffer === memory.buffer && fourth.length === 2048 + FRAME;
    fill(columns, 2048);
    SoftwareRenderTarget.staticPutPixelData(target, 2048, FRAME, 80, 20);
    globalThis.straightShared = read();
}
globalThis.accessor = accessor;
globalThis.done = true;
</script></body></html>`;

const server = http.createServer((req, res) => {
    // A cross-origin isolated page: only such a page has SharedArrayBuffer.
    res.setHeader("cross-origin-opener-policy", "same-origin"); res.setHeader("cross-origin-embedder-policy", "require-corp");
    if (req.url.startsWith("/ferroui.js")) { res.setHeader("content-type", "text/javascript"); res.end(fs.readFileSync(bundle)); return; }
    res.setHeader("content-type", "text/html"); res.end(page);
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const port = server.address().port;

const profile = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-pixels-"));
const chrome = spawn(candidates[0], ["--headless=new", "--no-first-run", "--no-sandbox", "--force-device-scale-factor=1",
    "--force-color-profile=srgb", `--user-data-dir=${profile}`, "--remote-debugging-port=0", "--window-size=200,100", "about:blank"],
{ stdio: ["ignore", "ignore", "pipe"] });
let chromeErrors = "";
chrome.stderr.on("data", (chunk) => { chromeErrors = (chromeErrors + chunk).slice(-4000); });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const finish = async (code) => { chrome.kill(); server.close(); await sleep(200); fs.rmSync(profile, { recursive: true, force: true }); process.exit(code); };

let debugPort;
// A cold runner can take well over ten seconds to start the browser.
for (let i = 0; i < 600 && !debugPort; i++) {
    await sleep(100);
    try { debugPort = fs.readFileSync(path.join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]; } catch { }
}
if (!debugPort) { console.error(`Chrome did not start within 60 s\n${chromeErrors}`); await finish(2); }
let pageTarget;
for (let i = 0; i < 50 && !pageTarget; i++) {
    await sleep(100);
    try { pageTarget = (await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json()).find((t) => t.type === "page"); } catch { }
}
const ws = new WebSocket(pageTarget.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
let id = 0; const pending = new Map(); const errors = [];
ws.addEventListener("message", (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result ?? m.error); pending.delete(m.id); }
    else if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
});
const send = (method, params = {}) => new Promise((r) => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
await send("Runtime.enable"); await send("Page.enable");
await send("Page.navigate", { url: `http://127.0.0.1:${port}/` });
let done = false;
for (let i = 0; i < 100 && !done && errors.length === 0; i++) {
    await sleep(100);
    done = (await send("Runtime.evaluate", { expression: "globalThis.done === true", returnByValue: true })).result?.value === true;
}
if (!done) { console.error("the page did not finish:", errors.join("\n")); await finish(1); }
const text = async (expression) => (await send("Runtime.evaluate", { expression: `JSON.stringify(${expression})`, returnByValue: true })).result.value;
const straight = await text("globalThis.straight");
const straightGrown = await text("globalThis.straightGrown");
const straightShared = await text("globalThis.straightShared");
const accessor = await text("globalThis.accessor");
const shot = await send("Page.captureScreenshot", { format: "png", clip: { x: 0, y: 0, width: 80, height: 20, scale: 1 } });

// Minimal PNG reader for the screenshot (8-bit RGB or RGBA, no interlace).
function decodePng(buffer) {
    let offset = 8; let width = 0; let height = 0; let channels = 4; const idat = [];
    while (offset < buffer.length) {
        const length = buffer.readUInt32BE(offset); const type = buffer.toString("latin1", offset + 4, offset + 8);
        const body = buffer.subarray(offset + 8, offset + 8 + length);
        if (type === "IHDR") { width = body.readUInt32BE(0); height = body.readUInt32BE(4); channels = body[9] === 6 ? 4 : 3; }
        if (type === "IDAT") idat.push(body);
        offset += 12 + length;
    }
    const raw = zlib.inflateSync(Buffer.concat(idat)); const stride = width * channels; const pixels = Buffer.alloc(stride * height);
    for (let y = 0; y < height; y++) {
        const filter = raw[y * (stride + 1)];
        for (let x = 0; x < stride; x++) {
            const value = raw[y * (stride + 1) + 1 + x];
            const a = x >= channels ? pixels[y * stride + x - channels] : 0;
            const b = y > 0 ? pixels[(y - 1) * stride + x] : 0;
            const c = x >= channels && y > 0 ? pixels[(y - 1) * stride + x - channels] : 0;
            let predicted = 0;
            if (filter === 1) predicted = a; else if (filter === 2) predicted = b; else if (filter === 3) predicted = (a + b) >> 1;
            else if (filter === 4) { const p = a + b - c; const pa = Math.abs(p - a); const pb = Math.abs(p - b); const pc = Math.abs(p - c); predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
            pixels[y * stride + x] = (value + predicted) & 255;
        }
    }
    return (x, y) => Array.from(pixels.subarray(y * stride + x * channels, y * stride + x * channels + 3));
}
const pixel = decodePng(Buffer.from(shot.data, "base64"));

// Premultiplied source over green (0,255,0): result = source + (1 - alpha) * background.
const expected = [
    ["half-transparent red", 10, [128, 127, 0]],
    ["quarter-transparent blue", 30, [0, 191, 64]],
    ["opaque", 50, [10, 20, 30]],
    ["transparent", 70, [0, 255, 0]]
];
let failed = false;
for (const [name, x, want] of expected) {
    const got = pixel(x, 10);
    const ok = got.every((v, i) => Math.abs(v - want[i]) <= 3);
    console.log(`${ok ? "ok  " : "FAIL"} ${name}: composited ${got} expected ${want}`);
    failed ||= !ok;
}
const wanted = [[255, 0, 0, 128], [0, 0, 255, 64], [10, 20, 30, 255], [0, 0, 0, 0]];
console.log("canvas pixels (straight alpha):", straight);
if (straight !== JSON.stringify(wanted)) {
    console.log("FAIL the canvas does not hold the frame with straight alpha");
    failed = true;
}
console.log("canvas pixels after the memory grew:", straightGrown);
if (straightGrown !== JSON.stringify(wanted.slice().reverse())) {
    console.log("FAIL the canvas does not hold the frame that lies beyond the end of the old buffer");
    failed = true;
}
console.log("canvas pixels from a shared memory:", straightShared);
if (straightShared !== JSON.stringify(wanted)) {
    console.log("FAIL the canvas does not hold the frame of the shared memory");
    failed = true;
}
console.log("the view of the memory:", accessor);
if (accessor !== JSON.stringify({ stable: true, detached: true, renewed: true, kept: true, isolated: true, shared: true })) {
    console.log("FAIL heapU8 did not follow the buffer of the memory (stable: one view while the buffer stays; renewed, shared: a new view over the new buffer)");
    failed = true;
}
await finish(failed ? 1 : 0);
