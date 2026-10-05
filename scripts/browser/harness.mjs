// Drives a built browser site in headless Chrome over the DevTools protocol: serves the site
// directory, opens the page, sends real input events and reads back pixels and page state.
// No dependencies beyond Node 22+ and a Chrome or Chromium binary (CHROME=<path> overrides the
// search). Used by capture.mjs and by the tests under scripts/browser/tests.
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import zlib from "node:zlib";
import { spawn } from "node:child_process";

const TYPES = {
    ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm",
    ".css": "text/css", ".map": "application/json", ".json": "application/json", ".png": "image/png",
    ".ico": "image/x-icon", ".svg": "image/svg+xml", ".br": "application/octet-stream", ".gz": "application/gzip"
};

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// The Chromium builds a Playwright installation keeps (newest first), under the directories
// Playwright uses: PLAYWRIGHT_BROWSERS_PATH, /opt/pw-browsers and the per-user caches.
function playwrightChromium() {
    const roots = [
        process.env.PLAYWRIGHT_BROWSERS_PATH, "/opt/pw-browsers",
        path.join(os.homedir(), ".cache", "ms-playwright"), path.join(os.homedir(), "Library", "Caches", "ms-playwright")
    ].filter((root) => root && fs.existsSync(root));
    const found = [];
    for (const root of roots) {
        for (const entry of fs.readdirSync(root).filter((name) => name.startsWith("chromium")).sort().reverse()) {
            for (const relative of ["chrome-linux/chrome", "chrome-linux64/chrome", "chrome-linux/headless_shell",
                "chrome-mac/Chromium.app/Contents/MacOS/Chromium", "chrome-win/chrome.exe"]) {
                found.push(path.join(root, entry, relative));
            }
        }
    }
    return found;
}

export function findChrome() {
    const candidates = [
        process.env.CHROME,
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/usr/bin/chromium-browser", "/usr/bin/chromium",
        ...playwrightChromium()
    ].filter((candidate) => candidate && fs.existsSync(candidate));
    if (candidates.length === 0) { throw new Error("no Chrome or Chromium found: set CHROME to the binary"); }
    return candidates[0];
}

export function serve(siteDirectory) {
    const root = path.resolve(siteDirectory);
    const server = http.createServer((request, response) => {
        let relative = decodeURIComponent(request.url.split("?")[0]);
        if (relative.endsWith("/")) { relative += "index.html"; }
        const file = path.join(root, relative);
        if (!file.startsWith(root) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
            response.statusCode = 404; response.end(); return;
        }
        response.setHeader("content-type", TYPES[path.extname(file)] ?? "application/octet-stream");
        fs.createReadStream(file).pipe(response);
    });
    return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

// Decodes an 8-bit RGB or RGBA, non-interlaced PNG (what the screenshots are).
export function decodePng(buffer) {
    let offset = 8; let width = 0; let height = 0; let channels = 4; const idat = [];
    while (offset < buffer.length) {
        const length = buffer.readUInt32BE(offset); const type = buffer.toString("latin1", offset + 4, offset + 8);
        const body = buffer.subarray(offset + 8, offset + 8 + length);
        if (type === "IHDR") { width = body.readUInt32BE(0); height = body.readUInt32BE(4); channels = body[9] === 6 ? 4 : 3; }
        if (type === "IDAT") { idat.push(body); }
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
            if (filter === 1) { predicted = a; } else if (filter === 2) { predicted = b; } else if (filter === 3) { predicted = (a + b) >> 1; } else if (filter === 4) {
                const p = a + b - c; const pa = Math.abs(p - a); const pb = Math.abs(p - b); const pc = Math.abs(p - c);
                predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
            }
            pixels[y * stride + x] = (value + predicted) & 255;
        }
    }
    return {
        width,
        height,
        pixel: (x, y) => Array.from(pixels.subarray(y * stride + x * channels, y * stride + x * channels + 3))
    };
}

const KEYS = {
    Tab: { key: "Tab", code: "Tab", keyCode: 9 },
    Enter: { key: "Enter", code: "Enter", keyCode: 13, text: "\r" },
    Escape: { key: "Escape", code: "Escape", keyCode: 27 },
    Backspace: { key: "Backspace", code: "Backspace", keyCode: 8 },
    Space: { key: " ", code: "Space", keyCode: 32, text: " " },
    ArrowLeft: { key: "ArrowLeft", code: "ArrowLeft", keyCode: 37 },
    ArrowUp: { key: "ArrowUp", code: "ArrowUp", keyCode: 38 },
    ArrowRight: { key: "ArrowRight", code: "ArrowRight", keyCode: 39 },
    ArrowDown: { key: "ArrowDown", code: "ArrowDown", keyCode: 40 },
    PageDown: { key: "PageDown", code: "PageDown", keyCode: 34 },
    Home: { key: "Home", code: "Home", keyCode: 36 },
    End: { key: "End", code: "End", keyCode: 35 }
};
const MODIFIERS = { alt: 1, ctrl: 2, meta: 4, shift: 8 };

// Opens `siteDirectory` (query appended to index.html) and resolves to the page driver.
export async function open(siteDirectory, { query = "", width = 460, height = 520, scale = 1, chromeArgs = [] } = {}) {
    const server = await serve(siteDirectory);
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-browser-"));
    const chrome = spawn(findChrome(), [
        "--headless=new", "--no-first-run", "--no-default-browser-check", "--no-sandbox", "--hide-scrollbars",
        // WebGL through the software rasteriser where there is no GPU (CI runners, containers).
        "--enable-unsafe-swiftshader", "--use-angle=swiftshader",
        `--user-data-dir=${profile}`, "--remote-debugging-port=0",
        `--window-size=${width},${height}`, `--force-device-scale-factor=${scale}`, "--force-color-profile=srgb",
        ...chromeArgs, "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
    let chromeErrors = "";
    chrome.stderr.on("data", (chunk) => { chromeErrors = (chromeErrors + chunk).slice(-4000); });

    // A cold machine (a fresh CI runner) can take tens of seconds to start the browser.
    let debugPort;
    for (let i = 0; i < 600 && !debugPort; i++) {
        await sleep(100);
        try { debugPort = fs.readFileSync(path.join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]; } catch { }
    }
    if (!debugPort) { chrome.kill(); server.close(); throw new Error(`Chrome did not start within 60 s\n${chromeErrors}`); }
    let target;
    for (let i = 0; i < 300 && !target; i++) {
        await sleep(100);
        try { target = (await (await fetch(`http://127.0.0.1:${debugPort}/json`)).json()).find((t) => t.type === "page"); } catch { }
    }
    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve) => socket.addEventListener("open", resolve));

    // `log` holds every console message, uncaught exception and browser log entry; `errors` the ones
    // that are errors (console.error and console.assert, uncaught exceptions, failed loads).
    let id = 0; const pending = new Map(); const log = []; const errors = []; const navigations = [];
    socket.addEventListener("message", (event) => {
        const message = JSON.parse(event.data);
        if (message.id && pending.has(message.id)) { pending.get(message.id)(message.result ?? { error: message.error }); pending.delete(message.id); return; }
        if (message.method === "Runtime.consoleAPICalled") {
            const line = `[console.${message.params.type}] ` + message.params.args.map((a) => a.value ?? a.description ?? "").join(" ");
            log.push(line);
            if (message.params.type === "error" || message.params.type === "assert") { errors.push(line); }
        } else if (message.method === "Runtime.exceptionThrown") {
            const line = "[exception] " + (message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text);
            log.push(line); errors.push(line);
        } else if (message.method === "Log.entryAdded") {
            const entry = message.params.entry;
            const line = `[log.${entry.level}] ${entry.text}${entry.url ? ` (${entry.url})` : ""}`;
            log.push(line);
            if (entry.level === "error") { errors.push(line); }
        } else if (message.method === "Page.frameNavigated" && !message.params.frame.parentId) {
            navigations.push(message.params.frame.url);
        }
    });
    const send = (method, params = {}) => new Promise((resolve) => { pending.set(++id, resolve); socket.send(JSON.stringify({ id, method, params })); });
    await send("Runtime.enable"); await send("Log.enable"); await send("Page.enable");
    // A fixed viewport. With a scale factor other than 1 the override would report unscaled device
    // pixels to the page, so the window size and the real scale factor of the browser are used.
    if (scale === 1) { await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false }); }
    const url = `http://127.0.0.1:${server.address().port}/index.html${query}`;
    await send("Page.navigate", { url });

    // The browser target, for the commands the page target does not take (permissions).
    let browserSocket;
    const browserPending = new Map(); let browserId = 0;
    const browserSend = async (method, params = {}) => {
        if (!browserSocket) {
            const version = await (await fetch(`http://127.0.0.1:${debugPort}/json/version`)).json();
            browserSocket = new WebSocket(version.webSocketDebuggerUrl);
            await new Promise((resolve) => browserSocket.addEventListener("open", resolve));
            browserSocket.addEventListener("message", (event) => {
                const message = JSON.parse(event.data);
                if (message.id && browserPending.has(message.id)) { browserPending.get(message.id)(message.result ?? { error: message.error }); browserPending.delete(message.id); }
            });
        }
        return new Promise((resolve) => { browserPending.set(++browserId, resolve); browserSocket.send(JSON.stringify({ id: browserId, method, params })); });
    };

    const page = {
        url,
        log,
        errors,
        navigations,
        send,
        browserSend,
        // Sets a permission of the page ("clipboard-read", "window-management", ...) to "granted",
        // "denied" or "prompt".
        async setPermission(name, setting) {
            const result = await browserSend("Browser.setPermission", { permission: { name }, setting, origin: new URL(url).origin });
            if (result.error) { throw new Error(`Browser.setPermission ${name}: ${result.error.message}`); }
        },
        // Evaluates an expression in the page and returns its JSON-serialisable value.
        async evaluate(expression) {
            const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
            if (result.exceptionDetails) { throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text); }
            return result.result?.value;
        },
        // Waits until the expression is truthy; throws after `timeout` milliseconds.
        async waitFor(expression, timeout = 120000) {
            const end = Date.now() + timeout;
            while (Date.now() < end) {
                if (await page.evaluate(`!!(${expression})`).catch(() => false)) { return; }
                await sleep(100);
            }
            throw new Error(`timed out waiting for ${expression}\n${log.join("\n")}`);
        },
        // Waits for the first rendered frame of the view (the splash is closed then) and lets it settle.
        async waitForView(settle = 800) {
            await page.waitFor("document.querySelector('canvas') && (document.querySelector('.ferroui-splash') === null || document.querySelector('.ferroui-splash').classList.contains('splash-close'))");
            await sleep(settle);
        },
        // Sets the size of the viewport in CSS pixels (the device scale factor stays that of the browser).
        async resize(newWidth, newHeight) {
            await send("Emulation.setDeviceMetricsOverride", { width: newWidth, height: newHeight, deviceScaleFactor: 0, mobile: false });
        },
        // A screenshot of the viewport, or of `clip` ({ x, y, width, height } in CSS pixels) of it,
        // written to `file` when one is given and returned decoded (with the PNG bytes as `png`).
        async screenshot(file, clip) {
            const shot = await send("Page.captureScreenshot", clip ? { format: "png", clip: { ...clip, scale: 1 } } : { format: "png" });
            const buffer = Buffer.from(shot.data, "base64");
            if (file) { fs.mkdirSync(path.dirname(path.resolve(file)), { recursive: true }); fs.writeFileSync(file, buffer); }
            return { ...decodePng(buffer), png: buffer };
        },
        async mouseMove(x, y) { await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, pointerType: "mouse" }); },
        async mouseDown(x, y, button = "left") { await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button, buttons: 1, clickCount: 1, pointerType: "mouse" }); },
        async mouseUp(x, y, button = "left") { await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button, buttons: 0, clickCount: 1, pointerType: "mouse" }); },
        async click(x, y) { await page.mouseMove(x, y); await page.mouseDown(x, y); await page.mouseUp(x, y); await sleep(150); },
        async wheel(x, y, deltaX, deltaY) { await send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX, deltaY, pointerType: "mouse" }); await sleep(150); },
        // Presses a named key (see KEYS) or a single character, with optional modifiers ["ctrl", "shift", ...]
        // and editing commands the browser runs as the default action of the key ({ commands: ["paste"] }).
        async press(name, modifiers = [], { commands } = {}) {
            const key = KEYS[name] ?? { key: name, code: /^[a-z]$/i.test(name) ? `Key${name.toUpperCase()}` : "", keyCode: name.toUpperCase().charCodeAt(0), text: name };
            const mask = modifiers.reduce((m, modifier) => m | MODIFIERS[modifier], 0);
            const command = mask & (MODIFIERS.ctrl | MODIFIERS.meta);
            const base = { key: key.key, code: key.code, windowsVirtualKeyCode: key.keyCode, nativeVirtualKeyCode: key.keyCode, modifiers: mask };
            await send("Input.dispatchKeyEvent", { type: key.text && !command ? "keyDown" : "rawKeyDown", ...base, ...(key.text && !command ? { text: key.text } : {}), ...(commands ? { commands } : {}) });
            await send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
            await sleep(80);
        },
        async type(text) { for (const character of text) { await page.press(character); } },
        async close() {
            socket.close(); browserSocket?.close(); chrome.kill(); server.close();
            await sleep(200);
            fs.rmSync(profile, { recursive: true, force: true });
        }
    };
    return page;
}

// Runs named checks, prints one line each and exits non-zero when one fails.
export async function run(checks) {
    let failed = 0;
    for (const [name, check] of checks) {
        try { await check(); console.log(`ok    ${name}`); } catch (error) { failed++; console.log(`FAIL  ${name}\n      ${String(error.message ?? error).split("\n").join("\n      ")}`); }
    }
    console.log(failed === 0 ? `all ${checks.length} checks passed` : `${failed} of ${checks.length} checks failed`);
    process.exit(failed === 0 ? 0 : 1);
}

export function assert(condition, message) { if (!condition) { throw new Error(message); } }

// Whether two RGB triples are within `tolerance` per channel.
export function near(a, b, tolerance = 6) { return a.every((value, i) => Math.abs(value - b[i]) <= tolerance); }
