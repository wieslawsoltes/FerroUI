// Run with a linked browser site and the editor_clipboard_windows test executable
// built with the Direct2D feature. The executable launches both private peers.
import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { assert, run } from "../harness.mjs";
import { checkNativeBrowserClipboard } from "./editor_clipboard_native_roundtrip.mjs";

const [site, native, browser] = process.argv.slice(2);
assert(process.platform === "win32" && site && native && browser,
    "Usage on Windows: node editor_clipboard_windows.test.mjs SITE NATIVE_TEST EDGE");

process.env.CHROME = path.resolve(native);
process.env.FERROUI_TEST_EDGE_EXE = path.resolve(browser);
process.env.FERROUI_TEST_WINDOWS_DESKTOP = `FerroClipboard-${randomUUID()}`;
const nativeOperation = (action, shape) => execFileSync(process.env.CHROME, ["--fixture", action, shape], {
    timeout: 35_000, stdio: "inherit",
});

await run(["Software2D", "WebGL2"].map(mode => [
    `${mode}: Windows browser and native clipboard`,
    () => checkNativeBrowserClipboard(site, mode, {
        browser: "Edge",
        platform: "Windows",
        command: "ctrl",
        rectangleModifier: "alt",
        nativePaste: shape => nativeOperation("paste", shape),
        withNativeCopy: async (shape, verify) => {
            nativeOperation("copy", shape);
            await verify();
        },
    }),
]));
