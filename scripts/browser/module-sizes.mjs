// Sizes of the files of a browser site: raw, with gzip and with brotli (both at their highest level),
// as a Markdown table.
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

const files = fs.readdirSync(site, { recursive: true })
    .map((name) => path.join(site, name))
    .filter((file) => fs.statSync(file).isFile() && !file.endsWith(".map"))
    .sort((a, b) => fs.statSync(b).size - fs.statSync(a).size);

const megabytes = (bytes) => `${(bytes / 1_000_000).toFixed(2)} MB`;
const size = (bytes) => bytes >= 100_000 ? megabytes(bytes) : `${(bytes / 1000).toFixed(1)} kB`;

console.log("| File | Raw | gzip -9 | brotli -11 |");
console.log("|---|---:|---:|---:|");
const total = [0, 0, 0];
const wasm = [0, 0];
for (const file of files) {
    const content = fs.readFileSync(file);
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
    if (file.endsWith(".wasm")) { wasm[0] += sizes[0]; wasm[1] += sizes[1]; }
    console.log(`| \`${path.relative(site, file)}\` | ${sizes.map(size).join(" | ")} |`);
}
console.log(`| total (without source maps) | ${total.map(size).join(" | ")} |`);

if (githubOutput !== undefined) {
    fs.appendFileSync(githubOutput, `wasm_raw_bytes=${wasm[0]}\nwasm_gzip_bytes=${wasm[1]}\n`);
}
