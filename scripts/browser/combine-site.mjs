// Composes one site that carries both modules of an application: the one built without threads
// and the one built with threads (`scripts/build-browser.sh <application> --threads`). The host page
// of such a site chooses between them before it requests either
// (scripts/browser/threads/ferroui-loader.js; docs/porting/browser-render-worker.md, "B2.8").
//
//   node scripts/browser/combine-site.mjs <site without threads> <site with threads> <output directory>
//
// `scripts/build-browser.sh <application> --both` builds the two sites and calls this.
//
// The layout of the output:
//
//   index.html, main.js, ...          the host page: the files of the site without threads
//   ferroui.js, storage.js,           the script modules of the platform, the service worker and the
//   ferroui-sw.js, ferroui-loader.js  loader, once
//   assets/...                        the files the build scripts of the application wrote, once
//   <application>.js, <name>.wasm     the module without threads, where a site with one module has it
//   threads/<application>.js          the module with threads, under the same two names: its script
//   threads/<name>.wasm               names the WebAssembly file and itself (for the web workers of
//                                     its threads) relative to its own address
//   threads/ferroui.js                one line that re-exports ../ferroui.js: the script of the module
//                                     imports `./ferroui.js`, and the host page and the module have
//                                     to share one instance of that script
//
// Everything but the two files of the module is shared, and shared files have to be identical: every
// file the two sites both have, other than the script of the module and the WebAssembly file, is
// compared byte for byte, and a file only one of them has is an error (except ferroui-threads.js of
// the site with threads, which is left out: the loader isolates the page of a site with both modules
// itself, and falls back to the other module where that check shows a message). The composition
// fails with the list of what differs; nothing is written then.
//
// The host page is changed in two places: the elements that preload the script of the module and
// the WebAssembly file are removed (the loader adds the one of the module it chose: a preload of
// the module that is not used would download it for nothing), and
// `<meta name="ferroui-modules" content='{"threads":"./threads/","wasm":"<name>.wasm"}'>` is added,
// which is how the loader knows the site has both.
//
// The output directory is deleted first. It has to be empty, absent or an earlier output of this
// script.
import fs from "node:fs";
import path from "node:path";

const THREADS_DIRECTORY = "threads";
const CHECK = "ferroui-threads.js";
const LOADER = "ferroui-loader.js";
const PLATFORM = "ferroui.js";
const MARKER = "ferroui-modules";

function fail(message) {
    console.error(`combine-site: ${message}`);
    process.exit(1);
}

const argv = process.argv.slice(2);
if (argv.length !== 3 || argv.some((argument) => argument.startsWith("-"))) {
    console.error("usage: node scripts/browser/combine-site.mjs <site without threads> <site with threads> <output directory>");
    process.exit(2);
}
const [plain, threaded, out] = argv.map((directory) => path.resolve(directory));

/** The files of a site, as paths relative to it with `/`. */
function filesOf(site) {
    return fs.readdirSync(site, { recursive: true })
        .map((name) => name.split(path.sep).join("/"))
        .filter((name) => fs.statSync(path.join(site, name)).isFile())
        .sort();
}

for (const [site, what] of [[plain, "without threads"], [threaded, "with threads"]]) {
    if (!fs.existsSync(path.join(site, "index.html"))) { fail(`no site ${what} at ${site}`); }
}
if (fs.existsSync(path.join(plain, CHECK))) { fail(`${plain} is a site built with threads (it has ${CHECK})`); }
if (!fs.existsSync(path.join(threaded, CHECK))) { fail(`${threaded} is not a site built with threads (it has no ${CHECK})`); }
if (fs.existsSync(path.join(plain, THREADS_DIRECTORY))) { fail(`${plain} has a directory ${THREADS_DIRECTORY}/ of its own`); }
if (!fs.existsSync(path.join(plain, LOADER))) { fail(`${plain} has no ${LOADER}: its host page cannot choose a module`); }
for (const site of [plain, threaded]) {
    if (out === site || out.startsWith(site + path.sep) || site.startsWith(out + path.sep)) { fail(`the output directory ${out} and the site ${site} contain each other`); }
}
if (fs.existsSync(out)) {
    const entries = fs.statSync(out).isDirectory() ? fs.readdirSync(out) : null;
    const earlier = entries !== null && entries.includes("index.html") && entries.includes(THREADS_DIRECTORY);
    if (entries === null || (entries.length > 0 && !earlier)) { fail(`refusing to replace ${out}: it is neither empty nor an earlier output of this script`); }
}

const plainFiles = filesOf(plain);
const threadedFiles = filesOf(threaded);

// The module: one WebAssembly file at the root of each site, of the same name, and the one script
// at the root that names it.
function moduleOf(site, files) {
    const wasm = files.filter((name) => !name.includes("/") && name.endsWith(".wasm"));
    if (wasm.length !== 1) { fail(`${site} has ${wasm.length} WebAssembly files at its root, expected one`); }
    const scripts = files.filter((name) => !name.includes("/") && name.endsWith(".js")
        && fs.readFileSync(path.join(site, name), "utf8").includes(`"${wasm[0]}"`));
    if (scripts.length !== 1) { fail(`${site}: ${scripts.length} scripts name ${wasm[0]} (${scripts.join(", ")}), expected the script of the module alone`); }
    return { wasm: wasm[0], script: scripts[0] };
}
const module = moduleOf(plain, plainFiles);
const threadedModule = moduleOf(threaded, threadedFiles);
if (module.wasm !== threadedModule.wasm || module.script !== threadedModule.script) {
    fail(`the sites are of different applications: ${module.script} with ${module.wasm}, and ${threadedModule.script} with ${threadedModule.wasm}`);
}
const ofModule = new Set([module.script, module.wasm]);

// Shared files are identical files.
const problems = [];
const plainSet = new Set(plainFiles);
const threadedSet = new Set(threadedFiles);
let compared = 0;
for (const name of plainFiles) {
    if (ofModule.has(name)) { continue; }
    if (!threadedSet.has(name)) { problems.push(`${name}: only in the site without threads`); continue; }
    compared++;
    if (!fs.readFileSync(path.join(plain, name)).equals(fs.readFileSync(path.join(threaded, name)))) { problems.push(`${name}: differs between the two sites`); }
}
for (const name of threadedFiles) {
    if (!plainSet.has(name) && name !== CHECK) { problems.push(`${name}: only in the site with threads`); }
}
if (problems.length > 0) {
    fail(`the two sites do not share their files (build both from the same sources):\n  ${problems.join("\n  ")}`);
}

// The script of the module with threads moves into a directory of its own: every import it makes
// relative to itself has to be there. It makes one, of the script module of the platform (the
// imports the wasm-bindgen glue and ferroui-worker-import.js write at its top).
const threadedScript = fs.readFileSync(path.join(threaded, module.script), "utf8");
// Import statements start a line there; a comment that quotes one does not.
const imports = new Set();
for (const match of threadedScript.matchAll(/^[ \t]*import\b[^;\n]*?["'](\.{1,2}\/[^"']+)["']/gm)) { imports.add(match[1]); }
const unknown = [...imports].filter((specifier) => specifier !== `./${PLATFORM}`);
if (unknown.length > 0) { fail(`the script of the module with threads imports ${unknown.join(", ")}: only ./${PLATFORM} is provided in ${THREADS_DIRECTORY}/`); }
if (!threadedScript.includes("ferrouiThreads")) { fail(`${path.join(threaded, module.script)} does not say it was built with threads (no export ferrouiThreads)`); }

// The host page: no preload of either file of the module, and the element the loader reads.
let page = fs.readFileSync(path.join(plain, "index.html"), "utf8");
if (page.includes(`name="${MARKER}"`)) { fail(`the host page of ${plain} already has a ${MARKER} element`); }
const escaped = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
let removed = 0;
for (const name of ofModule) {
    const element = new RegExp(`[ \\t]*<link\\b[^>]*\\bhref=["'](?:\\./)?${escaped(name)}["'][^>]*>[ \\t]*\\r?\\n?`, "g");
    page = page.replace(element, () => { removed++; return ""; });
}
const marker = `<meta name="${MARKER}" content='${JSON.stringify({ threads: `./${THREADS_DIRECTORY}/`, wasm: module.wasm })}'>`;
const head = /^([ \t]*)<\/head>/m.exec(page);
if (head === null) { fail(`the host page of ${plain} has no </head> to put the ${MARKER} element before`); }
page = `${page.slice(0, head.index)}${head[1]}    ${marker}\n${page.slice(head.index)}`;

// Written only now that nothing can fail but the file system.
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
fs.cpSync(plain, out, { recursive: true });
fs.writeFileSync(path.join(out, "index.html"), page);
fs.mkdirSync(path.join(out, THREADS_DIRECTORY));
for (const name of ofModule) { fs.copyFileSync(path.join(threaded, name), path.join(out, THREADS_DIRECTORY, name)); }
fs.writeFileSync(path.join(out, THREADS_DIRECTORY, PLATFORM),
    `// The script of the module with threads imports ./${PLATFORM}; the host page imports the one of the site.\n`
    + `// They have to be one instance (scripts/browser/combine-site.mjs).\nexport * from "../${PLATFORM}";\n`);

const megabytes = (bytes) => `${(bytes / 1_000_000).toFixed(2)} MB`;
const sizeOf = (directory, name) => fs.statSync(path.join(directory, name)).size;
const shared = plainFiles.filter((name) => !ofModule.has(name)).reduce((sum, name) => sum + sizeOf(plain, name), 0);
const total = filesOf(out).reduce((sum, name) => sum + sizeOf(out, name), 0);
console.log(`site written to ${out}`);
console.log(`  shared by both modules (${compared} files, identical in the two sites, source maps included): ${megabytes(shared)}`);
console.log(`  the module without threads (${module.script}, ${module.wasm}): ${megabytes(sizeOf(plain, module.script) + sizeOf(plain, module.wasm))}`);
console.log(`  the module with threads (${THREADS_DIRECTORY}/${module.script}, ${THREADS_DIRECTORY}/${module.wasm}): ${megabytes(sizeOf(threaded, module.script) + sizeOf(threaded, module.wasm))}`);
console.log(`  ${removed} preload element${removed === 1 ? "" : "s"} of the module removed from the host page; the site is ${megabytes(total)}`);
