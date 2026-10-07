// Sizes of the files of a browser site: raw, with gzip and with brotli (both at their highest level),
// as a Markdown table. When the site has a list of its asset files (a JSON file with `assembly`,
// `directory`, `files`, `startup` and `pages`, which the build script of the ControlCatalog sample
// writes to `assets/ControlCatalog.json`), the files of its directory are one row of the first table,
// and a second table gives the asset files by what they are loaded for: the start-up, each page of the
// list that has files of its own (a file several pages use counts for each of them), the files of the
// pages each counted once, and the files no page waits for.
//
//   node scripts/browser/module-sizes.mjs <site directory> [--github-output <file>]
//
// Megabytes are 1,000,000 bytes. With --github-output the raw and the gzip size in bytes of the
// WebAssembly modules of the site (summed, when there are several) are appended to the file as the
// outputs `wasm_raw_bytes` and `wasm_gzip_bytes` of a GitHub Actions step (pass "$GITHUB_OUTPUT").
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const argv = process.argv.slice(2);
let site;
let githubOutput;
for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--github-output") githubOutput = argv[++i];
    else site = argv[i];
}
if (!site || !fs.existsSync(site) || (githubOutput !== undefined && !githubOutput)) {
    console.error("usage: node scripts/browser/module-sizes.mjs <site directory> [--github-output <file>]");
    process.exit(2);
}

/** Paths relative to the site, with `/`. */
const files = fs.readdirSync(site, { recursive: true })
    .map((name) => name.split(path.sep).join("/"))
    .filter((name) => fs.statSync(path.join(site, name)).isFile() && !name.endsWith(".map"))
    .sort((a, b) => fs.statSync(path.join(site, b)).size - fs.statSync(path.join(site, a)).size);

const megabytes = (bytes) => `${(bytes / 1_000_000).toFixed(2)} MB`;
const size = (bytes) => bytes >= 100_000 ? megabytes(bytes) : `${(bytes / 1000).toFixed(1)} kB`;

/** The lists of asset files of the site. */
const manifests = [];
for (const name of files.filter((name) => name.endsWith(".json"))) {
    let manifest;
    try { manifest = JSON.parse(fs.readFileSync(path.join(site, name), "utf8")); } catch { continue; }
    if (typeof manifest?.directory !== "string" || typeof manifest.files !== "object" || !Array.isArray(manifest.startup)
        || typeof manifest.pages !== "object") continue;
    manifests.push({ name, manifest });
}
const assetDirectoryOf = (name) => manifests.map(({ manifest }) => manifest.directory).find((directory) => name.startsWith(`${directory}/`));

console.log("| File | Raw | gzip -9 | brotli -11 |");
console.log("|---|---:|---:|---:|");
const total = [0, 0, 0];
const assetDirectories = new Map();
const wasm = [0, 0];
const sizesOf = new Map();
for (const name of files) {
    const content = fs.readFileSync(path.join(site, name));
    const sizes = [
        content.length,
        zlib.gzipSync(content, { level: 9 }).length,
        zlib.brotliCompressSync(content, {
            params: {
                [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
                [zlib.constants.BROTLI_PARAM_SIZE_HINT]: content.length
            }
        }).length
    ];
    sizes.forEach((value, i) => { total[i] += value; });
    sizesOf.set(name, sizes);
    if (name.endsWith(".wasm")) { wasm[0] += sizes[0]; wasm[1] += sizes[1]; }
    const directory = assetDirectoryOf(name);
    if (directory === undefined) {
        console.log(`| \`${name}\` | ${sizes.map(size).join(" | ")} |`);
        continue;
    }
    const sum = assetDirectories.get(directory) ?? { count: 0, sizes: [0, 0, 0] };
    sum.count++;
    sizes.forEach((value, i) => { sum.sizes[i] += value; });
    assetDirectories.set(directory, sum);
}
for (const [directory, sum] of assetDirectories) {
    console.log(`| \`${directory}/\` (${sum.count} asset files) | ${sum.sizes.map(size).join(" | ")} |`);
}
console.log(`| total (without source maps) | ${total.map(size).join(" | ")} |`);

for (const { name, manifest } of manifests) {
    /** Raw and gzip size of the files of `assets` (rooted asset paths). */
    const sum = (assets) => {
        const result = [0, 0];
        for (const asset of assets) {
            const sizes = sizesOf.get(`${manifest.directory}${asset}`);
            if (!sizes) throw new Error(`${name} lists ${asset}, which is not a file of the site`);
            result[0] += sizes[0]; result[1] += sizes[1];
        }
        return result;
    };
    const row = (label, assets) => {
        const [raw, gzip] = sum(assets);
        console.log(`| ${label} | ${assets.length} | ${size(raw)} | ${size(gzip)} |`);
    };
    const ofPages = new Set(Object.values(manifest.pages).flat());
    const unreached = Object.keys(manifest.files).filter((asset) => !manifest.startup.includes(asset) && !ofPages.has(asset));
    const withFiles = Object.entries(manifest.pages).filter(([, assets]) => assets.length > 0)
        .sort(([a, x], [b, y]) => sum(y)[0] - sum(x)[0] || a.localeCompare(b));
    console.log("");
    console.log(`Asset files listed in \`${name}\` (${Object.keys(manifest.pages).length} pages, ${withFiles.length} of them with files of their own):`);
    console.log("");
    console.log("| Loaded for | Files | Raw | gzip -9 |");
    console.log("|---|---:|---:|---:|");
    row("start-up", manifest.startup);
    for (const [page, assets] of withFiles) row(`page ${page}`, assets);
    row("the files of the pages, each once", [...ofPages]);
    if (unreached.length > 0) row("no page (prefetched only)", unreached);
    row("all", Object.keys(manifest.files));
}

if (githubOutput !== undefined) {
    fs.appendFileSync(githubOutput, `wasm_raw_bytes=${wasm[0]}\nwasm_gzip_bytes=${wasm[1]}\n`);
}
