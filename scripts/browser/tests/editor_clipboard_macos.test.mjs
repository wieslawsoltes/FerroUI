// Run on a dedicated macOS desktop because the two peers use its general pasteboard.
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { createInterface } from "node:readline";
import { assert, run } from "../harness.mjs";
import { checkNativeBrowserClipboard } from "./editor_clipboard_native_roundtrip.mjs";

const [site, native] = process.argv.slice(2);
assert(process.platform === "darwin" && site && native,
    "Usage on macOS: node editor_clipboard_macos.test.mjs SITE NATIVE_TEST");
const executable = path.resolve(native);

async function withNativeCopy(shape, verify) {
    const owner = spawn(executable, ["--fixture", "copy", shape], {
        stdio: ["ignore", "pipe", "inherit"],
    });
    const exited = once(owner, "close");
    const earlyExit = exited.then(([code, signal]) => {
        throw new Error(`Native pasteboard owner exited before paste (${signal ?? code})`);
    });
    const output = createInterface({ input: owner.stdout });
    const ready = (async () => {
        for await (const line of output) {
            if (line === "FERROUI_EDITOR_CLIPBOARD_READY") return;
            console.log(line);
        }
        throw new Error("Native pasteboard owner did not publish its selection");
    })();
    let timeout;
    try {
        await Promise.race([ready, earlyExit, new Promise((_, reject) => {
            timeout = setTimeout(() => reject(new Error("Native pasteboard owner did not become ready")), 35_000);
        })]);
        clearTimeout(timeout);
        // Promised custom formats must remain available through Chrome's read and paste.
        await Promise.race([verify(), earlyExit]);
    } finally {
        clearTimeout(timeout);
        output.close();
        owner.kill();
        await exited.catch(() => {});
    }
}

await run([["Software2D: macOS browser and native clipboard",
    () => checkNativeBrowserClipboard(site, "Software2D", {
        browser: "Chrome",
        platform: "macOS",
        command: "meta",
        rectangleModifier: "ctrl",
        keyOptions: { nativeKeyCodes: false },
        refocus: true,
        nativePaste: shape => execFileSync(executable, ["--fixture", "paste", shape], {
            timeout: 35_000, stdio: "inherit",
        }),
        withNativeCopy,
    }),
]]);
