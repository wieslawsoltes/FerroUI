// Sizes of the files of a browser site: raw, with gzip and with brotli (both at their highest level),
// as a Markdown table.
//
//   node scripts/browser/module-sizes.mjs <site directory>
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const site = process.argv[2];
if (!site || !fs.existsSync(site)) {
    console.error("usage: node scripts/browser/module-sizes.mjs <site directory>");
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
    console.log(`| \`${path.relative(site, file)}\` | ${sizes.map(size).join(" | ")} |`);
}
console.log(`| total (without source maps) | ${total.map(size).join(" | ")} |`);
