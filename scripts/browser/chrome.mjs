// Headless Chrome driven over the DevTools protocol, for the browser scripts of the repository.
//
//   const chrome = await launchChrome({ width: 1280, height: 800 });
//   const page = await chrome.openPage();
//   await page.send("Page.navigate", { url });
//   ...
//   await chrome.close();
//
// The binary is $CHROME, else the first of the usual locations that exists (Google Chrome on
// macOS, google-chrome or chromium on Linux, the Playwright build of the cloud image).
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** The path of the Chrome or Chromium binary to use, or undefined. */
export function findChrome() {
    const playwright = "/opt/pw-browsers";
    const bundled = fs.existsSync(playwright)
        ? fs.readdirSync(playwright).filter((name) => /^chromium-\d+$/.test(name)).sort().reverse()
            .map((name) => path.join(playwright, name, "chrome-linux", "chrome"))
        : [];
    return [
        process.env.CHROME,
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome", "/usr/bin/chromium-browser", "/usr/bin/chromium",
        ...bundled
    ].find((candidate) => candidate && fs.existsSync(candidate));
}

/**
 * Starts headless Chrome with a fresh profile and the given window size and device scale factor.
 * `args` are added to the command line.
 */
export async function launchChrome({ width = 1280, height = 800, scale = 1, args = [] } = {}) {
    const binary = findChrome();
    if (!binary) throw new Error("no Chrome found: set CHROME to the path of a Chrome or Chromium binary");
    const profile = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-chrome-"));
    const child = spawn(binary, [
        "--headless=new", "--no-first-run", "--no-default-browser-check", "--no-sandbox",
        `--force-device-scale-factor=${scale}`, "--force-color-profile=srgb", "--hide-scrollbars",
        // WebGL through the software rasteriser where there is no GPU (CI runners, containers).
        "--enable-unsafe-swiftshader", "--use-angle=swiftshader",
        `--user-data-dir=${profile}`, "--remote-debugging-port=0", `--window-size=${width},${height}`,
        ...args, "about:blank"
    ], { stdio: ["ignore", "ignore", "pipe"] });
    let stderr = "";
    child.stderr.on("data", (chunk) => { stderr = (stderr + chunk).slice(-4000); });

    let port;
    // A cold runner can take well over ten seconds to start the browser.
    for (let i = 0; i < 600 && !port && child.exitCode === null; i++) {
        await sleep(100);
        try { port = fs.readFileSync(path.join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]; } catch { }
    }
    const close = async () => {
        child.kill();
        await sleep(200);
        fs.rmSync(profile, { recursive: true, force: true });
    };
    if (!port) {
        await close();
        throw new Error(`Chrome did not start within 60 s\n${stderr}`);
    }

    return {
        binary,
        /** The browser version string, for the log. */
        async version() {
            return (await (await fetch(`http://127.0.0.1:${port}/json/version`)).json()).Browser;
        },
        /** Connects to the page Chrome opened. */
        async openPage() {
            let target;
            for (let i = 0; i < 50 && !target; i++) {
                await sleep(100);
                try { target = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find((t) => t.type === "page"); } catch { }
            }
            if (!target) throw new Error("Chrome has no page");
            return connect(target.webSocketDebuggerUrl);
        },
        close
    };
}

/** A DevTools protocol session over the web socket of a page. */
async function connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve);
        socket.addEventListener("error", reject);
    });
    let id = 0;
    const pending = new Map();
    const listeners = [];
    socket.addEventListener("message", (event) => {
        const message = JSON.parse(event.data);
        if (message.id && pending.has(message.id)) {
            const { resolve, reject, method } = pending.get(message.id);
            pending.delete(message.id);
            if (message.error) reject(new Error(`${method}: ${message.error.message}`));
            else resolve(message.result);
        } else if (message.method) {
            for (const listener of listeners) listener(message.method, message.params);
        }
    });
    const send = (method, params = {}) => new Promise((resolve, reject) => {
        pending.set(++id, { resolve, reject, method });
        socket.send(JSON.stringify({ id, method, params }));
    });
    return {
        send,
        /** Calls `listener(method, params)` for every event of the page. */
        on(listener) { listeners.push(listener); },
        /** The value of a script expression of the page (awaited when it is a promise). */
        async evaluate(expression) {
            const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
            if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
            return result.result.value;
        },
        /** Waits until `expression` is truthy; false after `timeout` milliseconds. */
        async waitFor(expression, timeout, interval = 100) {
            for (const end = Date.now() + timeout; Date.now() < end;) {
                try { if (await this.evaluate(expression)) return true; } catch { }
                await sleep(interval);
            }
            return false;
        },
        /** A PNG of the viewport, or of `clip` ({ x, y, width, height }) of it, as bytes. */
        async screenshot(clip) {
            const result = await send("Page.captureScreenshot", clip ? { format: "png", clip: { ...clip, scale: 1 } } : { format: "png" });
            return Buffer.from(result.data, "base64");
        },
        close() { socket.close(); }
    };
}
