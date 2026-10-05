// Sizes of the files of a browser site: raw, with gzip and with brotli (both at their highest level),
// as a Markdown table.
//
//   node scripts/browser/module-sizes.mjs <site directory> [--gzip-budget-mb <megabytes>]
//
// With --gzip-budget-mb the script fails (exit code 1, after the table) when a WebAssembly module of the
// site is larger than the budget with gzip; megabytes are 1,000,000 bytes, as in the table.
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const argv = process.argv.slice(2);
let site;
let budget;
for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--gzip-budget-mb") budget = Number(argv[++i]);
    else site = argv[i];
}
if (!site || !fs.existsSync(site) || (budget !== undefined && !(budget > 0))) {
    console.error("usage: node scripts/browser/module-sizes.mjs <site directory> [--gzip-budget-mb <megabytes>]");
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
const overBudget = [];
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
    if (budget !== undefined && file.endsWith(".wasm") && sizes[1] > budget * 1_000_000) overBudget.push([file, sizes[1]]);
    console.log(`| \`${path.relative(site, file)}\` | ${sizes.map(size).join(" | ")} |`);
}
console.log(`| total (without source maps) | ${total.map(size).join(" | ")} |`);

if (budget !== undefined) {
    console.log();
    for (const [file, gzip] of overBudget) {
        console.log(`**Over budget:** \`${path.relative(site, file)}\` is ${megabytes(gzip)} with gzip; the budget is ${megabytes(budget * 1_000_000)}.`);
    }
    if (overBudget.length === 0) console.log(`Every WebAssembly module is within the gzip budget of ${megabytes(budget * 1_000_000)}.`);
    process.exit(overBudget.length === 0 ? 0 : 1);
}
