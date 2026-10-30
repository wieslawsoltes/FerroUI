// Build the shared browser script once, or verify the exact successful build used by a batch.
//
//   node scripts/browser/build-webapp.mjs build
//   node scripts/browser/build-webapp.mjs verify
//
// `verify` is an explicit batch contract, not an implicit npm cache. It refuses changes to any
// source, installed dependency or emitted file. Ordinary build-browser.sh calls still use build.
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repository = path.resolve(path.dirname(scriptPath), "../..");
const webapp = path.join(repository, "src/Browser/FerroUI.Browser/webapp");
const receiptName = ".ferroui-validated-webapp.json";
const outputs = ["ferroui.js", "ferroui.js.map", "storage.js", "storage.js.map", "ferroui-sw.js", "ferroui-sw.js.map"];
const schema = 2;

// A receipt is deliberately narrower than running the tools. An inherited executable config
// can import arbitrary files or consult external state; hashing its entry point is insufficient.
// Such a setup still gets the ordinary install/typecheck/lint/build, but no reusable receipt.
class UnsupportedReuse extends Error {}

/** Hash names and contents, including additions/deletions and targets of file symlinks. */
export function treeDigest(directory, excluded = new Set()) {
    const hash = crypto.createHash("sha256");
    const ancestors = new Set();
    function visit(absolute, relative) {
        const link = fs.lstatSync(absolute);
        if (link.isSymbolicLink()) hash.update(`link\0${relative}\0${fs.readlinkSync(absolute)}\0`);
        const stat = fs.statSync(absolute);
        if (stat.isDirectory()) {
            const real = fs.realpathSync(absolute);
            if (ancestors.has(real)) throw new Error(`cyclic input directory: ${relative}`);
            ancestors.add(real);
            hash.update(`directory\0${relative}\0`);
            for (const name of fs.readdirSync(absolute).sort()) {
                if (!relative && excluded.has(name)) continue;
                visit(path.join(absolute, name), relative ? `${relative}/${name}` : name);
            }
            ancestors.delete(real);
        } else if (stat.isFile()) {
            const bytes = fs.readFileSync(absolute);
            hash.update(`file\0${relative}\0${stat.mode & 0o777}\0${bytes.length}\0`);
            hash.update(bytes);
        } else {
            throw new Error(`unsupported input file: ${relative}`);
        }
    }
    visit(directory, "");
    return hash.digest("hex");
}

/** Cover the bounded inputs tools can discover outside the webapp and its dependencies. */
function externalInputs(directory) {
    const hash = crypto.createHash("sha256");
    const checkedProjects = new Set();
    function input(file) {
        hash.update(`${file}\0`);
        let stat;
        try { stat = fs.lstatSync(file); }
        catch (error) {
            if (error.code !== "ENOENT") throw error;
            hash.update("absent\0");
            return false;
        }
        if (!stat.isFile() && !stat.isSymbolicLink()) {
            throw new UnsupportedReuse(`external build input is not a file: ${file}`);
        }
        if (stat.isSymbolicLink()) {
            try { fs.statSync(file); }
            catch (error) {
                if (error.code !== "ENOENT") throw error;
                hash.update(`dangling-link\0${fs.readlinkSync(file)}\0`);
                return false;
            }
        }
        // Content, mode and symlink target identity; equal-size edits with restored mtimes count.
        hash.update(`present\0${treeDigest(file)}\0`);
        return true;
    }
    function staticInheritedConfig(config, file) {
        // These fields are data consumed by the already fingerprinted ESLint implementation.
        // In particular, extends/parser/plugins/settings/parserOptions can introduce file or
        // module dependencies outside the webapp, which this bounded contract cannot cover.
        const fields = new Set(["root", "env", "globals", "rules", "ignorePatterns", "overrides", "files", "excludedFiles"]);
        if (!config || typeof config !== "object" || Array.isArray(config)
            || Object.keys(config).some(key => !fields.has(key))) {
            throw new UnsupportedReuse(`inherited ESLint config has external dependency settings: ${file}`);
        }
        if (Object.keys(config.rules ?? {}).some(name => name.includes("/"))) {
            // Plugin rule options can name files, e.g. import/no-extraneous-dependencies'
            // packageDir. The already installed plugin is hashed, but those external files are not.
            throw new UnsupportedReuse(`inherited ESLint plugin rules require full validation: ${file}`);
        }
        if (config.overrides !== undefined) {
            if (!Array.isArray(config.overrides)) throw new UnsupportedReuse(`unsupported ESLint overrides: ${file}`);
            for (const override of config.overrides) staticInheritedConfig(override, file);
        }
    }
    function typeScriptProject(file) {
        if (checkedProjects.has(file)) return;
        checkedProjects.add(file);
        let config;
        try {
            // JSONC: recognize quoted strings first, so comment delimiters inside a string are
            // never removed. Strip comments and trailing commas in separate passes; a comment
            // may occur between a trailing comma and its closing bracket. Unknown syntax stays
            // a full-validation-only configuration, rather than guessing TypeScript semantics.
            const text = fs.readFileSync(file, "utf8").replace(/^\uFEFF/, "");
            const withoutComments = text.replace(/("(?:\\.|[^"\\])*")|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
                (match, quoted) => quoted ?? match.replace(/[^\r\n]/g, " "));
            const json = withoutComments.replace(/("(?:\\.|[^"\\])*")|,\s*(?=[}\]])/g,
                (match, quoted) => quoted ?? match.slice(1));
            config = JSON.parse(json);
        } catch {
            throw new UnsupportedReuse(`TypeScript project configuration requires full validation: ${file}`);
        }
        // This receipt supports the browser's self-contained leaf project. Extends, references,
        // type acquisition and custom path/resolver options can introduce transitive inputs.
        // tsc still handles its complete language during every ordinary full build.
        const fields = new Set(["compilerOptions", "files", "include", "exclude", "compileOnSave"]);
        const options = new Set(["target", "module", "moduleResolution", "lib", "strict", "sourceMap",
            "noEmit", "noEmitOnError", "skipLibCheck", "isolatedModules", "esModuleInterop",
            "allowSyntheticDefaultImports", "forceConsistentCasingInFileNames", "noUnusedLocals",
            "noUnusedParameters", "strictNullChecks", "strictFunctionTypes", "noImplicitAny",
            "noImplicitReturns", "noFallthroughCasesInSwitch", "useUnknownInCatchVariables"]);
        if (!config || typeof config !== "object" || Array.isArray(config)
            || Object.keys(config).some(name => !fields.has(name))
            || (config.compilerOptions !== undefined && (!config.compilerOptions
                || typeof config.compilerOptions !== "object" || Array.isArray(config.compilerOptions)
                || Object.keys(config.compilerOptions).some(name => !options.has(name))))) {
            throw new UnsupportedReuse(`transitive or custom TypeScript project inputs require full validation: ${file}`);
        }
        const root = fs.realpathSync(directory);
        function withinRoot(candidate) {
            const relative = path.relative(root, candidate);
            return relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
        }
        for (const name of ["files", "include", "exclude"]) {
            if (config[name] === undefined) continue;
            if (!Array.isArray(config[name])) throw new UnsupportedReuse(`unsupported TypeScript ${name}: ${file}`);
            for (const value of config[name]) {
                if (typeof value !== "string" || path.win32.isAbsolute(value) || value.replaceAll("\\", "/").split("/").includes("..")) {
                    throw new UnsupportedReuse(`external TypeScript ${name} requires full validation: ${file}`);
                }
                const normalized = value.replaceAll("\\", "/");
                const resolved = path.resolve(path.dirname(file), normalized);
                // Follow existing literal path prefixes too; a local-looking glob can name an
                // external directory through a symlink. Do not enumerate arbitrary glob roots.
                let existing = path.resolve(path.dirname(file), normalized.split(/[*?{[\]]/, 1)[0] || ".");
                while (!fs.existsSync(existing) && path.dirname(existing) !== existing) existing = path.dirname(existing);
                if (!withinRoot(resolved) || !withinRoot(fs.realpathSync(existing))) {
                    throw new UnsupportedReuse(`external TypeScript ${name} requires full validation: ${file}`);
                }
            }
        }
        // Without explicit typeRoots, TypeScript includes visible @types packages from enclosing
        // directories. A newly created ancestor type root must invalidate an old receipt too.
        for (let current = path.dirname(root); ; current = path.dirname(current)) {
            if (fs.existsSync(path.join(current, "node_modules/@types"))) {
                throw new UnsupportedReuse(`inherited TypeScript type roots require full validation: ${file}`);
            }
            if (path.dirname(current) === current) break;
        }
    }
    function localConfig(config, file) {
        const root = fs.realpathSync(directory);
        const require = createRequire(path.join(root, "package.json"));
        function withinRoot(candidate) {
            const relative = path.relative(root, candidate);
            return relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
        }
        function installedModule(name) {
            let resolved;
            try { resolved = fs.realpathSync(require.resolve(name)); }
            catch { throw new UnsupportedReuse(`ESLint dependency cannot be fingerprinted: ${name}`); }
            if (!withinRoot(resolved)) throw new UnsupportedReuse(`ESLint dependency resolves outside the webapp: ${name}`);
        }
        function packageName(name, prefix) {
            if (name.startsWith("@")) {
                const [scope, item = ""] = name.split("/");
                return `${scope}/${item.startsWith(prefix) ? item : prefix + (item ? `-${item}` : "")}`;
            }
            return name.startsWith(prefix) ? name : `${prefix}-${name}`;
        }
        if (!config || typeof config !== "object" || Array.isArray(config)) {
            throw new UnsupportedReuse(`unsupported ESLint configuration: ${file}`);
        }
        if (config.extends !== undefined) {
            for (const name of Array.isArray(config.extends) ? config.extends : [config.extends]) {
                if (typeof name !== "string" || name.startsWith(".") || path.isAbsolute(name)) {
                    throw new UnsupportedReuse(`file-based ESLint extends requires full validation: ${file}`);
                }
                if (name === "eslint:recommended" || name === "eslint:all") continue;
                if (name.startsWith("plugin:")) {
                    installedModule(packageName(name.slice(7, name.lastIndexOf("/")), "eslint-plugin"));
                } else installedModule(packageName(name, "eslint-config"));
            }
        }
        if (config.parser !== undefined) installedModule(config.parser);
        if (config.plugins !== undefined) {
            if (!Array.isArray(config.plugins)) throw new UnsupportedReuse(`unsupported ESLint plugins: ${file}`);
            for (const name of config.plugins) installedModule(packageName(name, "eslint-plugin"));
        }
        if (config.settings !== undefined) throw new UnsupportedReuse(`ESLint resolver settings require full validation: ${file}`);
        if (Object.keys(config.rules ?? {}).some(name => name.includes("/") && !name.startsWith("@typescript-eslint/"))) {
            throw new UnsupportedReuse(`custom ESLint plugin rule options require full validation: ${file}`);
        }
        if (config.parserOptions !== undefined) {
            const options = config.parserOptions;
            const allowed = new Set(["ecmaVersion", "sourceType", "ecmaFeatures", "project", "tsconfigRootDir"]);
            if (!options || typeof options !== "object" || Object.keys(options).some(key => !allowed.has(key))) {
                throw new UnsupportedReuse(`custom ESLint parser settings require full validation: ${file}`);
            }
            const projectRoot = options.tsconfigRootDir === undefined ? root : path.resolve(root, options.tsconfigRootDir);
            if (!withinRoot(projectRoot)) throw new UnsupportedReuse(`ESLint project root is outside the webapp: ${file}`);
            if (options.project !== undefined && options.project !== false) {
                for (const project of Array.isArray(options.project) ? options.project : [options.project]) {
                    if (typeof project !== "string" || /[*?{}[\]]/.test(project)) {
                        throw new UnsupportedReuse(`automatic/glob ESLint projects require full validation: ${file}`);
                    }
                    const resolved = path.resolve(projectRoot, project);
                    if (!withinRoot(resolved) || (fs.existsSync(resolved) && !withinRoot(fs.realpathSync(resolved)))) {
                        throw new UnsupportedReuse(`ESLint project is outside the webapp: ${file}`);
                    }
                    typeScriptProject(resolved);
                }
            }
        }
        if (config.overrides !== undefined) {
            if (!Array.isArray(config.overrides)) throw new UnsupportedReuse(`unsupported ESLint overrides: ${file}`);
            for (const override of config.overrides) localConfig(override, file);
        }
    }
    for (const name of ["NODE_PATH", "NODE_OPTIONS"]) {
        if (process.env[name]?.trim()) {
            throw new UnsupportedReuse(`${name} can load external Node inputs; full validation is required for every build`);
        }
    }
    if (process.env.ESLINT_USE_FLAT_CONFIG === "true") {
        throw new UnsupportedReuse("flat ESLint configuration requires full validation for every build");
    }
    const legacyNames = [".eslintrc.js", ".eslintrc.cjs", ".eslintrc.yaml", ".eslintrc.yml", ".eslintrc.json", ".eslintrc", "package.json"];
    let inspectLegacy = true;
    const start = fs.realpathSync(directory);
    // The recipe invokes tsc -p tsconfig.json independently of ESLint's parser project selection.
    if (fs.existsSync(path.join(start, "tsconfig.json"))) typeScriptProject(path.join(start, "tsconfig.json"));
    for (let current = start; ; current = path.dirname(current)) {
        // ESLint 8 can auto-discover flat configuration independently of legacy root:true.
        if (process.env.ESLINT_USE_FLAT_CONFIG !== "false") {
            for (const name of ["eslint.config.js", "eslint.config.mjs", "eslint.config.cjs"]) {
                const file = path.join(current, name);
                if (input(file)) throw new UnsupportedReuse(`executable ESLint configuration requires full validation: ${file}`);
            }
        }
        if (inspectLegacy) {
            let selected;
            for (const name of legacyNames) {
                const file = path.join(current, name);
                if (!input(file) || selected) continue;
                if (name !== ".eslintrc.json" && name !== "package.json") {
                    throw new UnsupportedReuse(`non-JSON ESLint configuration requires full validation: ${file}`);
                }
                let document;
                try { document = JSON.parse(fs.readFileSync(file, "utf8")); }
                catch {
                    // ESLint also accepts JSON comments and other legacy formats. Preserve full
                    // validation without claiming that this small reader resolved their inputs.
                    throw new UnsupportedReuse(`ESLint configuration requires full validation: ${file}`);
                }
                const config = name === "package.json" ? document.eslintConfig : document;
                if (config !== undefined) selected = { file, config };
            }
            if (selected) {
                if (current === start) localConfig(selected.config, selected.file);
                else staticInheritedConfig(selected.config, selected.file);
                inspectLegacy = selected.config?.root !== true;
            }
        }
        if (path.dirname(current) === current) break;
    }
    // esbuild resolves relative overrides in the child's cwd, not this helper's cwd. Absence
    // also matters: esbuild can fall back to its installed binary until the override is created.
    if (process.env.ESBUILD_BINARY_PATH) input(path.resolve(start, process.env.ESBUILD_BINARY_PATH));
    return hash.digest("hex");
}

function identity(directory) {
    // Values may include private registry configuration. Record only their digest, never the
    // configuration itself. Rust target/profile changes do not affect this shared JS bundle.
    const environment = Object.entries(process.env).filter(([name]) =>
        ["NODE_OPTIONS", "NODE_PATH", "NODE_ENV", "ESBUILD_BINARY_PATH", "ESLINT_USE_FLAT_CONFIG"].includes(name)
        || /^npm_config_/i.test(name)).sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0);
    return {
        schema,
        directory: fs.realpathSync(directory),
        node: process.version,
        platform: process.platform,
        arch: process.arch,
        environment: crypto.createHash("sha256").update(JSON.stringify(environment)).digest("hex"),
        // Changing the validation recipe invalidates an old receipt as well.
        recipe: crypto.createHash("sha256").update(fs.readFileSync(scriptPath)).digest("hex"),
        external: externalInputs(directory),
        sources: treeDigest(directory, new Set(["node_modules"])),
        dependencies: treeDigest(path.join(directory, "node_modules")),
    };
}

function outputDigest(directory) {
    const dist = path.resolve(directory, "../dist");
    for (const name of outputs) {
        if (!fs.statSync(path.join(dist, name)).isFile()) throw new Error(`missing script output: ${name}`);
    }
    return treeDigest(dist, new Set([receiptName]));
}

/** Write only after every command succeeded; tests can exercise this contract on a fixture. */
export function recordBuild(directory, validatedIdentity) {
    let currentIdentity;
    try { currentIdentity = identity(directory); }
    catch (error) {
        if (!(error instanceof UnsupportedReuse)) throw error;
        throw new Error(`browser script inputs changed during validation (${error.message}); build again`);
    }
    if (JSON.stringify(currentIdentity) !== JSON.stringify(validatedIdentity)) {
        throw new Error("browser script inputs changed during validation; build again");
    }
    const receipt = path.resolve(directory, "../dist", receiptName);
    const temporary = `${receipt}.${process.pid}.${crypto.randomUUID()}.tmp`;
    const data = { identity: currentIdentity, outputs: outputDigest(directory) };
    try {
        fs.writeFileSync(temporary, `${JSON.stringify(data)}\n`, { flag: "wx" });
        fs.renameSync(temporary, receipt);
    } finally {
        fs.rmSync(temporary, { force: true });
    }
}

export function verifyBuild(directory = webapp) {
    try {
        const receipt = JSON.parse(fs.readFileSync(path.resolve(directory, "../dist", receiptName), "utf8"));
        if (JSON.stringify(receipt.identity) !== JSON.stringify(identity(directory)) || receipt.outputs !== outputDigest(directory)) {
            throw new Error("an input or output changed");
        }
    } catch (error) {
        throw new Error(`the prebuilt browser script is absent or stale (${error.message}); run node scripts/browser/build-webapp.mjs build`);
    }
}

/** Full validation always installs from the lockfile. No successful receipt survives a failure. */
export function build(directory = webapp, run = runNpm) {
    fs.rmSync(path.resolve(directory, "../dist", receiptName), { force: true });
    const beforeInstall = treeDigest(directory, new Set(["node_modules"]));
    run(["ci", "--no-audit", "--no-fund"], directory);
    let validated;
    let unavailable;
    try { validated = identity(directory); }
    catch (error) {
        if (!(error instanceof UnsupportedReuse)) throw error;
        unavailable = error.message;
    }
    if (beforeInstall !== (validated?.sources ?? treeDigest(directory, new Set(["node_modules"])))) {
        throw new Error("browser script sources changed during installation; build again");
    }
    for (const command of ["typecheck", "lint", "build"]) run(["run", command], directory);
    if (unavailable) return { reusable: false, reason: unavailable };
    recordBuild(directory, validated);
    return { reusable: true };
}

function runNpm(args, directory) {
    const npm = process.platform === "win32" ? "npm.cmd" : "npm";
    // These command names are fixed above; no shell text comes from a document or a caller.
    const result = spawnSync(npm, args, { cwd: directory, stdio: "inherit", shell: process.platform === "win32" });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`npm ${args.join(" ")} failed (${result.signal ?? result.status})`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
    try {
        if (process.argv.length !== 3 || !["build", "verify"].includes(process.argv[2])) {
            throw new Error("usage: node scripts/browser/build-webapp.mjs <build|verify>");
        }
        if (process.argv[2] === "build") {
            const result = build();
            console.log("Browser script built and validated.");
            if (!result.reusable) console.log(`Batch reuse unavailable: ${result.reason}.`);
        } else {
            verifyBuild();
            console.log("Browser script verified: exact validated inputs and outputs.");
        }
    } catch (error) {
        console.error(error.message);
        process.exitCode = 1;
    }
}
