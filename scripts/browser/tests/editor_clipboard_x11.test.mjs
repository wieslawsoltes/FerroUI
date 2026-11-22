// Run with a linked editor site and a native Linux EditorDemo binary.
// Node 22+, Chromium, Xvfb, xclip and xdotool are required.
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { assert, open } from "../harness.mjs";
import { checkClipboardRoundTrip } from "./editor_clipboard_roundtrip.mjs";
import { startXvfb } from "./editor_clipboard_xvfb.mjs";

const [site, executable] = process.argv.slice(2);
assert(process.platform === "linux" && site && executable,
    "Usage on Linux: node editor_clipboard_x11.test.mjs SITE EDITOR_DEMO");

const scratch = mkdtempSync(path.join(tmpdir(), "ferroui-x11-editor-"));
let xserver;
let native;
let page;

function command(program, args, input) {
    return execFileSync(program, args, {
        encoding: "utf8", timeout: 8000, input,
        ...(input === undefined ? {} : { stdio: ["pipe", "ignore", "inherit"] }),
    });
}

try {
    xserver = await startXvfb();
    process.env.DISPLAY = xserver.display;
    native = spawn(path.resolve(executable), [], {
        env: { ...process.env, XDG_CONFIG_HOME: path.join(scratch, "config"), XDG_DATA_HOME: path.join(scratch, "data") },
        stdio: ["ignore", "inherit", "inherit"],
    });
    await new Promise((resolve, reject) => { native.once("spawn", resolve); native.once("error", reject); });
    const xdo = (...args) => command("xdotool", args).trim();
    const window = xdo("search", "--sync", "--name", "^FerroUI Editor Demo$").split("\n").at(-1);
    const nativeKeys = (...keys) => xdo("key", "--clearmodifiers", ...keys);
    const clip = target => command("xclip", ["-selection", "clipboard", "-o", "-target", target]);

    page = await open(site, {
        query: "?RenderingMode=Software2D", width: 1180, height: 800, network: true,
        headless: false, chromeArgs: ["--ozone-platform=x11"],
    });
    const focusNative = () => {
        xdo("windowraise", window);
        xdo("windowfocus", "--sync", window);
        xdo("mousemove", "--window", window, "240", "165", "click", "1");
    };
    await checkClipboardRoundTrip(page, {
        platform: "X11", focusNative, nativeKeys,
        seedClipboard: text => command("xclip", ["-selection", "clipboard", "-i"], text),
        clipboardTypes: () => clip("TARGETS").split(/\s+/),
        readClipboard: clip,
        readClipboardText: () => clip("UTF8_STRING"),
    });
} finally {
    try {
        await page?.close();
    } finally {
        native?.kill();
        xserver?.process.kill();
        rmSync(scratch, { recursive: true, force: true });
    }
}
