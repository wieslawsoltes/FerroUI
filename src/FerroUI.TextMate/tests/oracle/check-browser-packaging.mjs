// Exercises the crate's real build script and FerroUI's real site composer
// without installing Emscripten or rebuilding the renderer. This checks asset
// layout and scanner execution; it is not an Emscripten application link test.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";

const crate = path.resolve(fileURLToPath(new URL("../..", import.meta.url)));
const repository = path.resolve(crate, "../..");
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-textmate-packaging-"));
const moduleName = "ferroui-textmate-oniguruma.js";

function run(command, args, options = {}) {
    const result = spawnSync(command, args, { encoding: "utf8", ...options });
    if (result.error) throw result.error;
    assert.equal(result.status, 0, `${command} ${args.join(" ")}\n${result.stdout}\n${result.stderr}`);
    return result.stdout;
}

try {
    const buildScript = path.join(temporary, process.platform === "win32" ? "build.exe" : "build");
    run(process.env.RUSTC || "rustc", ["--edition=2021", path.join(crate, "build.rs"), "-o", buildScript]);
    for (const target of ["emscripten", "unknown"]) {
        const output = path.join(temporary, target);
        fs.mkdirSync(output);
        run(buildScript, [], {
            env: {
                ...process.env,
                CARGO_MANIFEST_DIR: crate,
                OUT_DIR: output,
                CARGO_CFG_TARGET_ARCH: "wasm32",
                CARGO_CFG_TARGET_OS: target,
            },
        });
        const bindings = fs.readFileSync(path.join(output, "browser_bindings.rs"), "utf8");
        if (target === "emscripten") {
            assert.ok(bindings.includes(`raw_module = "./${moduleName}"`));
            assert.ok(!bindings.includes("inline_js"));
        } else {
            assert.ok(bindings.includes("inline_js"));
            assert.ok(!fs.existsSync(path.join(output, "browser-site")));
        }
    }

    const assets = path.join(temporary, "emscripten/browser-site");
    for (const notice of ["LICENSE", "NOTICES"]) {
        assert.deepEqual(
            fs.readFileSync(path.join(assets, `ferroui-textmate-oniguruma-${notice}.txt`)),
            fs.readFileSync(path.join(crate, `browser/vendor/${notice}.txt`)),
        );
    }

    // A minimal application shell makes the production composer inspect the
    // same imports and shared assets it receives from linked browser builds.
    for (const variant of ["plain", "threaded"]) {
        const site = path.join(temporary, variant);
        fs.mkdirSync(site);
        fs.cpSync(assets, site, { recursive: true });
        fs.writeFileSync(path.join(site, "package.json"), '{"type":"module"}\n');
        fs.writeFileSync(path.join(site, "index.html"), '<!doctype html>\n<html><head>\n<link rel="modulepreload" href="editor.js">\n<link rel="preload" href="editor.wasm">\n</head><body></body></html>\n');
        fs.writeFileSync(path.join(site, "ferroui.js"), "export const platform = {};\n");
        fs.writeFileSync(path.join(site, "ferroui-loader.js"), "// Application loader placeholder.\n");
        fs.writeFileSync(path.join(site, "editor.wasm"), Uint8Array.from([0, 97, 115, 109, 1, 0, 0, 0]));
        fs.writeFileSync(path.join(site, "editor.js"),
            `import "./ferroui.js";\nimport { ferroOnigurumaReady } from "./${moduleName}";\n`
            + 'export const wasm = "editor.wasm";\n'
            + (variant === "threaded" ? "export const ferrouiThreads = true;\n" : ""));
        if (variant === "threaded") fs.writeFileSync(path.join(site, "ferroui-threads.js"), "// Cross-origin-isolation check placeholder.\n");
    }
    const combined = path.join(temporary, "combined");
    run(process.execPath, [path.join(repository, "scripts/browser/combine-site.mjs"), path.join(temporary, "plain"), path.join(temporary, "threaded"), combined]);
    const plain = await import(pathToFileURL(path.join(combined, moduleName)).href);
    const threaded = await import(pathToFileURL(path.join(combined, "threads", moduleName)).href);
    assert.equal(plain.ferroInitializeOniguruma, threaded.ferroInitializeOniguruma);
    assert.equal(plain.ferroOnigurumaReady(), false);
    assert.throws(() => plain.ferroCompileOniguruma("x"), /initialize_browser_oniguruma/);
    const bytes = new Uint8Array(fs.readFileSync(path.join(crate, "browser/vendor/onig.wasm")));
    const initialization = plain.ferroInitializeOniguruma(bytes);
    assert.equal(initialization, threaded.ferroInitializeOniguruma(bytes));
    await initialization;
    assert.equal(threaded.ferroOnigurumaReady(), true);
    const scanner = threaded.ferroCompileOniguruma("(?<=😀)([a-z]+)\\1");
    assert.deepEqual(plain.ferroSearchOniguruma(scanner, "😀abab", 0), [[2, 6], [2, 4]]);
    threaded.ferroDisposeOniguruma(scanner);
    console.log("Emscripten flat assets/notices, production site composition, shared initialization, lookbehind/backreferences and UTF-16 scanner captures pass");
    console.log("wasm32-unknown-unknown keeps the verified inline module packaging");
} finally {
    fs.rmSync(temporary, { recursive: true, force: true });
}
