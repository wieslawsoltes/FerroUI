// Run with a linked editor site and EditorDemo built with the Linux wayland feature.
// Node 22+, Chromium, wl-clipboard, Xvfb and xdotool are required, with Sway
// by default or Weston when --weston is passed.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { closeSync, mkdtempSync, openSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { assert, open, sleep } from "../harness.mjs";
import { checkClipboardRoundTrip } from "./editor_clipboard_roundtrip.mjs";
import { startXvfb } from "./editor_clipboard_xvfb.mjs";

const [site, executable, option] = process.argv.slice(2);
const weston = option === "--weston";
assert(process.platform === "linux" && site && executable && (option === undefined || weston),
    "Usage on Linux: node editor_clipboard_wayland.test.mjs SITE EDITOR_DEMO [--weston]");
const compositorName = weston ? "Weston" : "Sway";

const scratch = mkdtempSync(path.join(tmpdir(), "ferroui-wayland-editor-"));
const config = path.join(scratch, weston ? "weston.ini" : "sway.conf");
writeFileSync(config, weston
    ? "[shell]\nbinding-modifier=super\nstartup-animation=none\nlocking=false\n[input-method]\npath=\n"
    : "xwayland disable\nfocus_follows_mouse no\noutput * mode 1200x850\n");
delete process.env.DISPLAY;
delete process.env.WAYLAND_DISPLAY;
delete process.env.SWAYSOCK;
process.env.XDG_RUNTIME_DIR = scratch;
process.env.XDG_CONFIG_HOME = path.join(scratch, "config");
process.env.XDG_DATA_HOME = path.join(scratch, "data");
const clipboardOwners = new Set();
let xserver;
let compositor;
let native;
let page;

function command(program, ...args) {
    return execFileSync(program, args, { encoding: "utf8", timeout: 8000 });
}

async function waitFor(check, message) {
    for (let i = 0; i < 120; i++) {
        const value = check();
        if (value) return value;
        await sleep(50);
    }
    throw new Error(message);
}

function findWindow(tree, predicate) {
    if (tree.app_id && predicate(tree)) return tree;
    for (const node of [...tree.nodes, ...tree.floating_nodes]) {
        const match = findWindow(node, predicate);
        if (match) return match;
    }
}

function getWindow(predicate) {
    return findWindow(JSON.parse(command("swaymsg", "-t", "get_tree")), predicate);
}

function focus(window) {
    command("swaymsg", `[con_id=${window.id}] focus`);
}

function seedClipboard(text) {
    const owner = spawn("wl-copy", ["--foreground", "--type", "text/plain;charset=utf-8"], {
        stdio: ["pipe", "ignore", "inherit"],
    });
    clipboardOwners.add(owner);
    owner.once("exit", () => clipboardOwners.delete(owner));
    owner.stdin.end(text);
    return new Promise((resolve, reject) => { owner.once("spawn", resolve); owner.once("error", reject); });
}

try {
    xserver = await startXvfb();
    // Only the compositor uses this X server. The two applications and the clipboard tool
    // connect to the Wayland socket.
    const westonSocket = "wayland-editor-test";
    const args = weston
        ? ["--backend=x11", "--renderer=pixman", "--width=1200", "--height=850", "--idle-time=0",
            `--socket=${westonSocket}`, `--config=${config}`]
        : ["-c", config];
    compositor = spawn(weston ? "weston" : "sway", args, {
        env: {
            ...process.env, DISPLAY: xserver.display,
            ...(weston ? {} : { WLR_BACKENDS: "x11", WLR_RENDERER: "pixman" }),
        },
        stdio: ["ignore", "ignore", "inherit"],
    });
    await new Promise((resolve, reject) => { compositor.once("spawn", resolve); compositor.once("error", reject); });
    await waitFor(() => {
        assert(compositor.exitCode === null, `The private ${compositorName} compositor exited`);
        const sockets = readdirSync(scratch);
        if (weston) {
            if (!sockets.includes(westonSocket)) return false;
            process.env.WAYLAND_DISPLAY = westonSocket;
            return true;
        }
        const socket = sockets.find(name => /^sway-ipc\.[0-9.]+\.sock$/.test(name));
        const display = sockets.find(name => /^wayland-\d+$/.test(name));
        if (!socket || !display) return false;
        process.env.SWAYSOCK = path.join(scratch, socket);
        process.env.WAYLAND_DISPLAY = display;
        return true;
    }, `${compositorName} did not create its sockets`);

    const xdo = (...args) => execFileSync("xdotool", args, {
        env: { ...process.env, DISPLAY: xserver.display }, encoding: "utf8", timeout: 8000,
    }).trim();
    const output = xdo("mousemove", "--sync", "300", "300", "getmouselocation", "--shell").match(/^WINDOW=(\d+)$/m)?.[1];
    assert(output, "xdotool did not find the nested compositor window");
    xdo("windowfocus", "--sync", output);
    const nativeKeys = (...chords) => xdo("key", "--clearmodifiers", ...chords);
    const clickNative = (x, y) => xdo("mousemove", "--window", output, String(x), String(y), "click", "1");

    const trace = path.join(scratch, "native-wayland.log");
    const traceFd = weston ? openSync(trace, "w") : null;
    try {
        native = spawn(path.resolve(executable), [], {
            env: weston ? { ...process.env, WAYLAND_DEBUG: "client" } : process.env,
            stdio: ["ignore", "inherit", traceFd ?? "inherit"],
        });
    } finally {
        if (traceFd !== null) closeSync(traceFd);
    }
    await new Promise((resolve, reject) => { native.once("spawn", resolve); native.once("error", reject); });
    const editor = await waitFor(() => {
        assert(native.exitCode === null, "EditorDemo exited before creating a Wayland window");
        // A keyboard enter confirms the Weston surface mapped. The trace goes to a file so
        // synchronous wl-paste reads cannot block the native clipboard owner on stderr.
        return weston ? /\bwl_keyboard#\d+\.enter\(/.test(readFileSync(trace, "utf8"))
            : getWindow(window => window.app_id === "org.ferroui.EditorDemo");
    }, "Native EditorDemo is missing from the Wayland tree");

    page = await open(site, {
        query: "?RenderingMode=Software2D", width: 1180, height: 800, network: true,
        headless: false, chromeArgs: weston ? ["--ozone-platform=wayland", "--start-maximized"] : ["--ozone-platform=wayland"],
    });
    const browser = weston ? null : await waitFor(() => getWindow(window => window.id !== editor.id),
        "Chromium is missing from the Wayland tree");
    let nativeInFront = false;
    const switchWestonWindow = async (toNative) => {
        if (nativeInFront === toNative) return;
        nativeKeys("Super_L+Tab");
        nativeInFront = toNative;
        await sleep(150);
    };
    const readClipboard = type => command("wl-paste", "--no-newline", "--type", type);
    const clipboardTypes = () => {
        const result = spawnSync("wl-paste", ["--list-types"], { encoding: "utf8", timeout: 8000 });
        if (result.status === 1 && result.stderr.trim().endsWith("Nothing is copied")) return [];
        assert(result.status === 0, result.error?.message ?? result.stderr);
        return result.stdout.trim().split("\n");
    };

    await checkClipboardRoundTrip(page, {
        platform: `Wayland (${compositorName})`,
        focusNative: async () => {
            if (weston) {
                await switchWestonWindow(true);
                clickNative(500, 300);
            } else {
                focus(editor);
                const { rect } = getWindow(window => window.id === editor.id);
                clickNative(rect.x + 200, rect.y + 250);
            }
            await sleep(75);
        },
        nativeKeys: (...chords) => {
            // On Weston, wl-paste maps a temporary surface for wl_data_device. Restore focus
            // before sending keys. Each native chord group sets its own selection if it needs one.
            if (weston) clickNative(500, 300);
            nativeKeys(...chords);
        },
        showBrowser: () => weston ? switchWestonWindow(false) : focus(browser),
        copyFromBrowser: () => nativeKeys("ctrl+c"),
        seedClipboard, clipboardTypes, readClipboard,
        readClipboardText: () => {
            const types = clipboardTypes();
            const text = ["text/plain;charset=utf-8", "text/plain"].find(type => types.includes(type));
            return text ? readClipboard(text) : "";
        },
    });
} finally {
    try {
        await page?.close();
    } finally {
        for (const owner of clipboardOwners) owner.kill();
        native?.kill();
        compositor?.kill();
        xserver?.process.kill();
        rmSync(scratch, { recursive: true, force: true });
    }
}
