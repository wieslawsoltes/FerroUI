import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";
import zlib from "node:zlib";
import { build, treeDigest, verifyBuild } from "../build-webapp.mjs";
import { CompressionCache } from "../compression-cache.mjs";

const scripts = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const emitted = ["ferroui.js", "ferroui.js.map", "storage.js", "storage.js.map", "ferroui-sw.js", "ferroui-sw.js.map"];

// These fixtures exercise the supported clean tool environment. Dedicated negative controls
// below cover external module search/preloads rather than inheriting the test runner's setup.
delete process.env.NODE_PATH;
delete process.env.NODE_OPTIONS;

function fixture(t) {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "ferroui-build-tools-"));
    t.after(() => fs.rmSync(root, { recursive: true, force: true }));
    const webapp = path.join(root, "webapp");
    fs.mkdirSync(path.join(webapp, "modules"), { recursive: true });
    fs.writeFileSync(path.join(webapp, "modules/input.ts"), "export const value = 1;\n");
    fs.writeFileSync(path.join(webapp, "package-lock.json"), "{}\n");
    fs.writeFileSync(path.join(webapp, ".eslintrc.json"), "{}\n");
    const calls = [];
    const run = (args) => {
        calls.push(args);
        if (args[0] === "ci") {
            fs.mkdirSync(path.join(webapp, "node_modules/dependency"), { recursive: true });
            fs.writeFileSync(path.join(webapp, "node_modules/dependency/index.js"), "const dependency = 1;\n");
        }
        if (args[1] === "build") {
            fs.mkdirSync(path.join(root, "dist"), { recursive: true });
            for (const name of emitted) fs.writeFileSync(path.join(root, "dist", name), `output ${name}\n`);
        }
    };
    return { root, webapp, run, calls };
}

test("a receipt is written only after the full install, typecheck, lint and bundle recipe", t => {
    const f = fixture(t);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    build(f.webapp, f.run);
    assert.deepEqual(f.calls, [["ci", "--no-audit", "--no-fund"], ["run", "typecheck"], ["run", "lint"], ["run", "build"]]);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    const before = f.calls.length;
    verifyBuild(f.webapp);
    assert.equal(f.calls.length, before);
});

test("source edits invalidate even when file length and timestamp are unchanged", t => {
    const f = fixture(t);
    build(f.webapp, f.run);
    const source = path.join(f.webapp, "modules/input.ts");
    const stat = fs.statSync(source);
    fs.writeFileSync(source, "export const value = 2;\n");
    fs.utimesSync(source, stat.atime, stat.mtime);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

for (const [name, mutate] of [
    ["new input", f => fs.writeFileSync(path.join(f.webapp, "modules/new.ts"), "export {};\n")],
    ["removed input", f => fs.unlinkSync(path.join(f.webapp, "modules/input.ts"))],
    ["lint configuration", f => fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), '{"root":true}')],
    ["dependency", f => fs.appendFileSync(path.join(f.webapp, "node_modules/dependency/index.js"), "changed")],
    ["removed dependency", f => fs.unlinkSync(path.join(f.webapp, "node_modules/dependency/index.js"))],
    ["emitted JavaScript", f => fs.appendFileSync(path.join(f.root, "dist/ferroui.js"), "changed")],
    ["source map", f => fs.appendFileSync(path.join(f.root, "dist/ferroui.js.map"), "changed")],
    ["missing service worker", f => fs.unlinkSync(path.join(f.root, "dist/ferroui-sw.js"))],
]) {
    test(`${name} changes cannot reuse a validated build`, t => {
        const f = fixture(t);
        build(f.webapp, f.run);
        mutate(f);
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
}

test("a failed gate invalidates an earlier receipt", t => {
    const f = fixture(t);
    build(f.webapp, f.run);
    assert.throws(() => build(f.webapp, args => {
        if (args[1] === "lint") throw new Error("lint failed");
        f.run(args);
    }), /lint failed/);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("build environment changes invalidate receipts without storing configuration values", t => {
    const f = fixture(t);
    build(f.webapp, f.run);
    const previous = process.env.npm_config_ferroui_fixture;
    try {
        process.env.npm_config_ferroui_fixture = "private-registry-fixture";
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
        build(f.webapp, f.run);
        assert.doesNotThrow(() => verifyBuild(f.webapp));
        const receipt = fs.readFileSync(path.join(f.root, "dist/.ferroui-validated-webapp.json"), "utf8");
        assert.ok(!receipt.includes("private-registry-fixture"));
    } finally {
        if (previous === undefined) delete process.env.npm_config_ferroui_fixture;
        else process.env.npm_config_ferroui_fixture = previous;
    }
});

test("changing source during a validation command cannot produce a receipt", t => {
    const f = fixture(t);
    assert.throws(() => build(f.webapp, args => {
        f.run(args);
        if (args[1] === "build") fs.appendFileSync(path.join(f.webapp, "modules/input.ts"), "changed");
    }), /inputs changed during validation/);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("inherited ESLint JSON content invalidates even when length and timestamp are unchanged", t => {
    const f = fixture(t);
    const config = path.join(f.root, ".eslintrc.json");
    fs.writeFileSync(config, '{"rules":{"eqeqeq":1}}');
    build(f.webapp, f.run);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    const stat = fs.statSync(config);
    fs.writeFileSync(config, '{"rules":{"eqeqeq":2}}');
    fs.utimesSync(config, stat.atime, stat.mtime);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("creation and deletion of inherited ESLint configuration invalidate receipts", t => {
    const f = fixture(t);
    const config = path.join(f.root, ".eslintrc.json");
    build(f.webapp, f.run);
    fs.writeFileSync(config, '{"rules":{"eqeqeq":"warn"}}');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    build(f.webapp, f.run);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    fs.unlinkSync(config);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("package.json eslintConfig and newly selected higher-priority configs are tracked", t => {
    const f = fixture(t);
    const config = path.join(f.root, "package.json");
    fs.writeFileSync(config, '{"eslintConfig":{"rules":{"eqeqeq":"warn"}}}');
    build(f.webapp, f.run);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    fs.writeFileSync(config, '{"eslintConfig":{"rules":{"eqeqeq":"error"}}}');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    build(f.webapp, f.run);
    fs.writeFileSync(path.join(f.root, ".eslintrc.json"), '{"rules":{}}');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("legacy root:true bounds ancestry while flat config discovery remains covered", t => {
    const f = fixture(t);
    fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), '{"root":true}');
    build(f.webapp, f.run);
    fs.writeFileSync(path.join(f.root, ".eslintrc.js"), 'module.exports = require("./dynamic.cjs");');
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    fs.writeFileSync(path.join(f.root, "eslint.config.js"), 'module.exports = [];');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

for (const [name, content] of [
    [".eslintrc.js", 'module.exports = require("./dynamic.cjs");'],
    [".eslintrc.json", '{"extends":"./external.json"}'],
    ["package.json", '{"eslintConfig":{"overrides":[{"files":"*.ts","parserOptions":{"project":"./external.json"}}]}}'],
    [".eslintrc.json", '{/* legacy JSON comments */ "rules":{}}'],
    [".eslintrc.json", '{"rules":{"import/no-extraneous-dependencies":["error",{"packageDir":"../external"}]}}'],
]) {
    test(`untracked inherited config (${name}: ${content}) keeps full validation and refuses reuse`, t => {
        const f = fixture(t);
        build(f.webapp, f.run);
        fs.writeFileSync(path.join(f.root, name), content);
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
        f.calls.length = 0;
        const result = build(f.webapp, f.run);
        assert.equal(result.reusable, false);
        assert.ok(result.reason);
        assert.deepEqual(f.calls, [["ci", "--no-audit", "--no-fund"], ["run", "typecheck"], ["run", "lint"], ["run", "build"]]);
        assert.equal(fs.existsSync(path.join(f.root, "dist/.ferroui-validated-webapp.json")), false);
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
}

function withEnvironment(name, value, run) {
    const previous = process.env[name];
    try {
        process.env[name] = value;
        return run();
    } finally {
        if (previous === undefined) delete process.env[name];
        else process.env[name] = previous;
    }
}

for (const config of [
    { root: true, extends: "../external.json" },
    { root: true, parserOptions: { project: "../tsconfig.json" } },
    { root: true, overrides: [{ files: "*.ts", parserOptions: { tsconfigRootDir: "..", project: "tsconfig.json" } }] },
    { root: true, rules: { "import/no-extraneous-dependencies": ["error", { packageDir: "../external" }] } },
]) {
    test(`external references in local ESLint configuration refuse reuse: ${JSON.stringify(config)}`, t => {
        const f = fixture(t);
        fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), JSON.stringify(config));
        fs.writeFileSync(path.join(f.root, "external.json"), '{"rules":{}}');
        fs.writeFileSync(path.join(f.root, "tsconfig.json"), '{}');
        const result = build(f.webapp, f.run);
        assert.equal(result.reusable, false);
        assert.equal(f.calls.length, 4);
        fs.writeFileSync(path.join(f.root, "external.json"), '{"rules":{"eqeqeq":2}}');
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
}

test("local installed ESLint config packages retain reuse and track dependency edits", t => {
    const f = fixture(t);
    const dependency = path.join(f.webapp, "node_modules/eslint-config-fixture/index.js");
    fs.mkdirSync(path.dirname(dependency), { recursive: true });
    fs.writeFileSync(dependency, 'module.exports = {rules: {eqeqeq: 1}};');
    fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), '{"extends":"fixture"}');
    assert.equal(build(f.webapp, f.run).reusable, true);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    fs.writeFileSync(dependency, 'module.exports = {rules: {eqeqeq: 2}};');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("the real commented browser TypeScript project remains eligible for receipt reuse", t => {
    const f = fixture(t);
    const project = path.join(f.webapp, "tsconfig.json");
    fs.copyFileSync(path.resolve(scripts, "../../src/Browser/FerroUI.Browser/webapp/tsconfig.json"), project);
    fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), '{"parserOptions":{"project":["tsconfig.json"]}}');
    assert.equal(build(f.webapp, f.run).reusable, true);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    fs.appendFileSync(project, "\n// changed configuration\n");
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("TypeScript JSONC parsing distinguishes strings, comments and trailing commas", t => {
    const f = fixture(t);
    fs.writeFileSync(path.join(f.webapp, "tsconfig.json"), `{
        // The text "extends": "../external.json" here is not a config entry.
        "compilerOptions": { "strict": true, /* trailing comma */ },
        "include": ["modules//**/*.ts", /* comment between comma and bracket */ ],
    }`);
    assert.equal(build(f.webapp, f.run).reusable, true);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
});

for (const [description, text] of [
    ["external extends", '{// supported JSONC syntax\n "extends":"../external.json"}'],
    ["local transitive extends", '{"extends":"./base.json"}'],
    ["project references", '{"references":[{"path":"../external"}]}'],
    ["external files", '{"files":["../external.ts"]}'],
    ["external include", '{"include":["../external/**/*.ts"]}'],
    ["Windows-style external include", '{"include":["..\\\\external\\\\*.ts"]}'],
    ["external path mappings", '{"compilerOptions":{"paths":{"external":["../external"]}}}'],
    ["external type roots", '{"compilerOptions":{"typeRoots":["../external-types"]}}'],
    ["comment-looking string path", '{"include":["/*literal*/../external.ts"]}'],
    ["malformed configuration", '{/* unterminated comment'],
]) {
    test(`TypeScript ${description} preserves full validation without reusing untracked inputs`, t => {
        const f = fixture(t);
        fs.writeFileSync(path.join(f.webapp, "tsconfig.json"), text);
        fs.writeFileSync(path.join(f.webapp, "base.json"), '{"extends":"../external.json"}');
        fs.writeFileSync(path.join(f.root, "external.json"), '{"compilerOptions":{"strict":true}}');
        // No ESLint project is needed here: tsc's own -p tsconfig.json input must also be bounded.
        const result = build(f.webapp, f.run);
        assert.equal(result.reusable, false);
        assert.match(result.reason, /TypeScript/);
        assert.equal(f.calls.length, 4);
        fs.writeFileSync(path.join(f.root, "external.json"), '{"compilerOptions":{"strict":false}}');
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
}

test("TypeScript parser projects outside the default tsconfig receive the same transitive checks", t => {
    const f = fixture(t);
    fs.writeFileSync(path.join(f.webapp, ".eslintrc.json"), '{"parserOptions":{"project":"lint-project.json"}}');
    fs.writeFileSync(path.join(f.webapp, "lint-project.json"), '{"extends":"../external.json"}');
    assert.equal(build(f.webapp, f.run).reusable, false);
    assert.equal(f.calls.length, 4);
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
});

test("newly visible ancestor TypeScript declaration packages invalidate an earlier receipt", t => {
    const f = fixture(t);
    fs.writeFileSync(path.join(f.webapp, "tsconfig.json"), '{"include":["modules/**/*.ts"]}');
    assert.equal(build(f.webapp, f.run).reusable, true);
    assert.doesNotThrow(() => verifyBuild(f.webapp));
    const inherited = path.join(f.root, "node_modules/@types/external/index.d.ts");
    fs.mkdirSync(path.dirname(inherited), { recursive: true });
    fs.writeFileSync(inherited, 'declare const external: unknown;');
    assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    assert.equal(build(f.webapp, f.run).reusable, false);
});

test("a local TypeScript glob cannot hide an external directory behind a symlink", t => {
    const f = fixture(t);
    if (process.platform === "win32") return t.skip("creating symlinks may require Windows privileges");
    const external = path.join(f.root, "external");
    fs.mkdirSync(external);
    fs.writeFileSync(path.join(external, "input.ts"), "export const value = 1;");
    fs.symlinkSync(external, path.join(f.webapp, "linked"));
    fs.writeFileSync(path.join(f.webapp, "tsconfig.json"), '{"include":["linked/**/*.ts"]}');
    assert.equal(build(f.webapp, f.run).reusable, false);
    assert.equal(f.calls.length, 4);
});

for (const [name, value] of [["NODE_PATH", "/external/modules"], ["NODE_OPTIONS", "--require /external/preload.cjs"]]) {
    test(`${name} with untracked external input preserves full builds without a reusable receipt`, t => {
        const f = fixture(t);
        build(f.webapp, f.run);
        withEnvironment(name, value, () => {
            assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
            f.calls.length = 0;
            const result = build(f.webapp, f.run);
            assert.equal(result.reusable, false);
            assert.match(result.reason, new RegExp(name));
            assert.equal(f.calls.length, 4);
            assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
        });
    });
}

test("external esbuild executable contents and disappearance invalidate an unchanged override path", t => {
    const f = fixture(t);
    const binary = path.join(f.root, "external-esbuild");
    fs.writeFileSync(binary, "binary-version-one");
    withEnvironment("ESBUILD_BINARY_PATH", binary, () => {
        build(f.webapp, f.run);
        assert.doesNotThrow(() => verifyBuild(f.webapp));
        const stat = fs.statSync(binary);
        fs.writeFileSync(binary, "binary-version-two");
        fs.utimesSync(binary, stat.atime, stat.mtime);
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
        build(f.webapp, f.run);
        fs.unlinkSync(binary);
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
});

test("a relative missing esbuild override is tracked from the webapp cwd and invalidates when created", t => {
    const f = fixture(t);
    withEnvironment("ESBUILD_BINARY_PATH", "../external-esbuild", () => {
        build(f.webapp, f.run);
        assert.doesNotThrow(() => verifyBuild(f.webapp));
        fs.writeFileSync(path.join(f.root, "external-esbuild"), "new-binary");
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
});

test("an external esbuild symlink follows target content changes", t => {
    const f = fixture(t);
    if (process.platform === "win32") return t.skip("creating symlinks may require Windows privileges");
    const binary = path.join(f.root, "external-esbuild");
    const link = path.join(f.root, "esbuild-link");
    fs.writeFileSync(binary, "binary-version-one");
    fs.symlinkSync(binary, link);
    withEnvironment("ESBUILD_BINARY_PATH", link, () => {
        build(f.webapp, f.run);
        assert.doesNotThrow(() => verifyBuild(f.webapp));
        fs.writeFileSync(binary, "binary-version-two");
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
});

test("a dangling esbuild override preserves full validation and tracks target creation", t => {
    const f = fixture(t);
    if (process.platform === "win32") return t.skip("creating symlinks may require Windows privileges");
    const binary = path.join(f.root, "external-esbuild");
    const link = path.join(f.root, "esbuild-link");
    fs.symlinkSync(binary, link);
    withEnvironment("ESBUILD_BINARY_PATH", link, () => {
        assert.equal(build(f.webapp, f.run).reusable, true);
        assert.doesNotThrow(() => verifyBuild(f.webapp));
        fs.writeFileSync(binary, "new-binary");
        assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
    });
});

test("a symlinked webapp fingerprints physical ESLint ancestors and relative esbuild overrides", t => {
    const f = fixture(t);
    if (process.platform === "win32") return t.skip("creating symlinks may require Windows privileges");
    const linkParent = path.join(f.root, "link-parent");
    fs.mkdirSync(linkParent);
    const link = path.join(linkParent, "webapp-link");
    fs.symlinkSync(f.webapp, link);
    const config = path.join(f.root, ".eslintrc.json");
    fs.writeFileSync(config, '{"rules":{"eqeqeq":1}}');
    const binary = path.join(f.root, "external-esbuild");
    fs.writeFileSync(binary, "binary-version-one");
    // Outputs use the webapp's physical cwd as npm does. This tests the helper through the
    // physical output location while retaining a logical input path at its public API.
    fs.symlinkSync(path.join(f.root, "dist"), path.join(linkParent, "dist"));
    withEnvironment("ESBUILD_BINARY_PATH", "../external-esbuild", () => {
        assert.equal(build(link, f.run).reusable, true);
        assert.doesNotThrow(() => verifyBuild(link));
        fs.writeFileSync(config, '{"rules":{"eqeqeq":2}}');
        assert.throws(() => verifyBuild(link), /absent or stale/);
        build(link, f.run);
        fs.writeFileSync(binary, "binary-version-two");
        assert.throws(() => verifyBuild(link), /absent or stale/);
    });
});

for (const [description, change] of [
    ["inherited JSON config", f => fs.writeFileSync(path.join(f.root, ".eslintrc.json"), '{"rules":{"eqeqeq":"warn"}}')],
    ["new executable config", f => fs.writeFileSync(path.join(f.root, ".eslintrc.js"), 'module.exports = {};')],
    ["external esbuild binary", f => fs.writeFileSync(path.join(f.root, "external-esbuild"), "binary-version-two")],
]) {
    test(`changing ${description} during validation cannot produce a receipt`, t => {
        const f = fixture(t);
        const binary = path.join(f.root, "external-esbuild");
        fs.writeFileSync(binary, "binary-version-one");
        withEnvironment("ESBUILD_BINARY_PATH", binary, () => {
            assert.throws(() => build(f.webapp, args => {
                f.run(args);
                if (args[1] === "build") change(f);
            }), /inputs changed during validation/);
            assert.throws(() => verifyBuild(f.webapp), /absent or stale/);
        });
    });
}

for (const reusable of [true, false]) {
    test(`ordinary --both preserves child validation when receipt reuse is ${reusable ? "available" : "unavailable"}`, t => {
        if (process.platform === "win32") return t.skip("the browser build entry point requires Bash");
        const f = fixture(t);
        const root = f.root;
        const actualWebapp = path.join(root, "src/Browser/FerroUI.Browser/webapp");
        fs.mkdirSync(path.dirname(actualWebapp), { recursive: true });
        fs.renameSync(f.webapp, actualWebapp);
        if (!reusable) fs.writeFileSync(path.join(root, ".eslintrc.js"), "module.exports = {};\n");
        fs.mkdirSync(path.join(root, "scripts/browser"), { recursive: true });
        fs.mkdirSync(path.join(root, "bin"));
        fs.copyFileSync(path.join(scripts, "build-webapp.mjs"), path.join(root, "scripts/browser/build-webapp.mjs"));
        fs.copyFileSync(path.resolve(scripts, "../build-browser.sh"), path.join(root, "scripts/build-browser-actual.sh"));
        // Execute the production --both branch unchanged. Only the recursive WebAssembly build
        // boundary and site composition are replaced: each child still runs the real JS build or
        // verify command selected by the parent. No Rust/Emscripten installation is needed.
        const driver = path.join(root, "scripts/build-browser.sh");
        fs.writeFileSync(driver, `#!/bin/bash
set -euo pipefail
fixture_root="$(cd "$(dirname "$0")/.." && pwd)"
for argument in "$@"; do
  if [ "$argument" = --both ]; then
    source "$fixture_root/scripts/build-browser-actual.sh"
    exit
  fi
done
printf '%s\\n' "$*" >> "$fixture_root/children.log"
action=build
for argument in "$@"; do
  if [ "$argument" = --webapp-prebuilt ]; then action=verify; fi
done
exec node "$fixture_root/scripts/browser/build-webapp.mjs" "$action"
`, { mode: 0o755 });
        fs.writeFileSync(path.join(root, "scripts/browser/combine-site.mjs"),
            'import fs from "node:fs"; fs.writeFileSync(process.env.FERROUI_TEST_COMBINED, "combined");\n');
        fs.writeFileSync(path.join(root, "bin/npm"), `#!${process.execPath}
const fs = require("node:fs"), path = require("node:path");
const args = process.argv.slice(2), cwd = process.cwd();
fs.appendFileSync(process.env.FERROUI_TEST_NPM_LOG, JSON.stringify(args) + "\\n");
if (args[0] === "ci") {
    fs.mkdirSync(path.join(cwd, "node_modules/dependency"), {recursive: true});
    fs.writeFileSync(path.join(cwd, "node_modules/dependency/index.js"), "module.exports = 1;");
}
if (args[1] === "build") {
    const dist = path.resolve(cwd, "../dist");
    fs.mkdirSync(dist, {recursive: true});
    for (const name of ${JSON.stringify(emitted)}) fs.writeFileSync(path.join(dist, name), "output " + name);
}
`, { mode: 0o755 });
        const log = path.join(root, "npm.log");
        const combined = path.join(root, "combined");
        const env = { ...process.env, PATH: `${path.join(root, "bin")}${path.delimiter}${process.env.PATH}`,
            CARGO_TARGET_DIR: path.join(root, "target"), FERROUI_TEST_NPM_LOG: log, FERROUI_TEST_COMBINED: combined };
        const result = spawnSync("bash", [driver, "example", "--both"], { env, encoding: "utf8" });
        assert.equal(result.status, 0, result.stderr + result.stdout);
        assert.equal(fs.readFileSync(combined, "utf8"), "combined");
        const children = fs.readFileSync(path.join(root, "children.log"), "utf8").trim().split("\n");
        assert.equal(children.length, 2);
        assert.equal(children.every(child => child.includes("--webapp-prebuilt")), reusable);
        const calls = fs.readFileSync(log, "utf8").trim().split("\n").map(line => JSON.parse(line));
        assert.equal(calls.length, reusable ? 4 : 12);
        for (let offset = 0; offset < calls.length; offset += 4) {
            assert.deepEqual(calls.slice(offset, offset + 4), [["ci", "--no-audit", "--no-fund"], ["run", "typecheck"], ["run", "lint"], ["run", "build"]]);
        }
        if (!reusable) {
            const previousLog = fs.readFileSync(log, "utf8");
            const explicit = spawnSync("bash", [driver, "example", "--both", "--webapp-prebuilt"], { env, encoding: "utf8" });
            assert.notEqual(explicit.status, 0);
            assert.match(explicit.stderr + explicit.stdout, /absent or stale/, JSON.stringify(explicit));
            assert.equal(fs.readFileSync(log, "utf8"), previousLog, "explicit prebuilt mode must not silently run a new build");
        }
    });
}

test("directory symlink cycles are rejected instead of reusing incomplete input coverage", t => {
    const f = fixture(t);
    if (process.platform === "win32") return t.skip("creating symlinks may require Windows privileges");
    fs.symlinkSync(f.webapp, path.join(f.webapp, "loop"));
    assert.throws(() => treeDigest(f.webapp), /cyclic input/);
});

test("cached lengths equal actual gzip9 and Brotli11 for binary and empty files", async t => {
    const f = fixture(t);
    const directory = path.join(f.root, "cache");
    const inputs = [Buffer.alloc(0), Buffer.from(Array.from({ length: 8192 }, (_, i) => (i * 37) % 256))];
    for (const input of inputs) {
        const expected = [input.length, zlib.gzipSync(input, { level: 9 }).length, zlib.brotliCompressSync(input, {
            params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11, [zlib.constants.BROTLI_PARAM_SIZE_HINT]: input.length },
        }).length];
        const cold = new CompressionCache(directory);
        assert.deepEqual(await cold.measure(input), expected);
        assert.equal(cold.misses, 1);
        const warm = new CompressionCache(directory);
        assert.deepEqual(await warm.measure(input), expected);
        assert.equal(warm.hits, 1);
        assert.equal(warm.misses, 0);
    }
});

test("equal-length changed contents miss the compression cache", async t => {
    const f = fixture(t);
    const directory = path.join(f.root, "cache");
    await new CompressionCache(directory).measure(Buffer.from("aaaaaa"));
    const cache = new CompressionCache(directory);
    const sizes = await cache.measure(Buffer.from("abcdef"));
    assert.equal(cache.hits, 0);
    assert.equal(cache.misses, 1);
    assert.deepEqual(sizes, await new CompressionCache().measure(Buffer.from("abcdef")));
});

test("malformed, foreign-key and inaccessible caches fall back to actual compression", async t => {
    const f = fixture(t);
    const directory = path.join(f.root, "cache");
    const content = Buffer.from("a repeated measurement\n".repeat(100));
    const expected = await new CompressionCache(directory).measure(content);
    const file = path.join(directory, fs.readdirSync(directory)[0]);
    const numericCorruption = JSON.parse(fs.readFileSync(file, "utf8"));
    numericCorruption.sizes[1]++;
    for (const corrupt of ["not JSON", JSON.stringify({ schema: 1, key: "foreign", sizes: [content.length, 0, 0] }), JSON.stringify({ schema: 1, sizes: [0, -1, null] }), JSON.stringify(numericCorruption)]) {
        fs.writeFileSync(file, corrupt);
        const cache = new CompressionCache(directory);
        assert.deepEqual(await cache.measure(content), expected);
        assert.equal(cache.misses, 1);
    }
    const notDirectory = path.join(f.root, "not-directory");
    fs.writeFileSync(notDirectory, "file");
    assert.deepEqual(await new CompressionCache(notDirectory).measure(content), expected);
});

test("simultaneous equal files share one measurement and atomic writers leave usable records", async t => {
    const f = fixture(t);
    const directory = path.join(f.root, "cache");
    const content = Buffer.from("shared contents".repeat(1000));
    const cache = new CompressionCache(directory);
    const sizes = await Promise.all([cache.measure(content), cache.measure(content), new CompressionCache(directory).measure(content)]);
    assert.deepEqual(sizes[0], sizes[1]);
    assert.deepEqual(sizes[0], sizes[2]);
    assert.equal(cache.misses, 1);
    assert.equal(cache.hits, 1);
    assert.ok(fs.readdirSync(directory).every(name => name.endsWith(".json")));
    const restored = new CompressionCache(directory);
    assert.deepEqual(await restored.measure(content), sizes[0]);
    assert.equal(restored.hits, 1);
});

test("cached reports and GitHub outputs are byte-for-byte identical to uncached reports", t => {
    const f = fixture(t);
    const site = path.join(f.root, "site");
    fs.mkdirSync(path.join(site, "assets/Example"), { recursive: true });
    fs.writeFileSync(path.join(site, "module.wasm"), Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]));
    fs.writeFileSync(path.join(site, "module.js"), "export default {};\n");
    fs.writeFileSync(path.join(site, "module.js.map"), "must not count");
    fs.writeFileSync(path.join(site, "assets/Example/hello.txt"), "a fixture asset\n");
    fs.writeFileSync(path.join(site, "assets/Example.json"), JSON.stringify({
        directory: "assets/Example", files: { "/hello.txt": {} }, startup: ["/hello.txt"], pages: { Home: ["/hello.txt"] },
    }));
    function report(name, flags) {
        const githubOutput = path.join(f.root, `${name}.out`);
        const stdoutPath = path.join(f.root, `${name}.stdout`);
        const stderrPath = path.join(f.root, `${name}.stderr`);
        const stdout = fs.openSync(stdoutPath, "w");
        const stderr = fs.openSync(stderrPath, "w");
        let result;
        try {
            result = spawnSync(process.execPath, [path.join(scripts, "module-sizes.mjs"), site, "--github-output", githubOutput, ...flags], { stdio: ["ignore", stdout, stderr] });
        } finally {
            fs.closeSync(stdout);
            fs.closeSync(stderr);
        }
        const errorText = fs.readFileSync(stderrPath, "utf8");
        assert.equal(result.status, 0, errorText);
        return { stdout: fs.readFileSync(stdoutPath, "utf8"), output: fs.readFileSync(githubOutput, "utf8"), stderr: errorText };
    }
    const reference = report("reference", ["--no-cache"]);
    assert.match(reference.stdout, /brotli -11/);
    const cold = report("cold", ["--cache-dir", path.join(f.root, "cache")]);
    const warm = report("warm", ["--cache-dir", path.join(f.root, "cache")]);
    for (const actual of [cold, warm]) {
        assert.equal(actual.stdout, reference.stdout);
        assert.equal(actual.output, reference.output);
    }
    assert.match(warm.stderr, /0 computed/);
    assert.doesNotMatch(warm.stdout, /module\.js\.map/);
});
