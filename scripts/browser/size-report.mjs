// Size of the files of a browser site: raw, gzip -9 and brotli -11.
//
//   node scripts/browser/size-report.mjs <site directory> [--json]
//
// Lists the .wasm and .js files of the directory (target/browser/<example> as
// scripts/build-browser.sh writes it). gzip runs the `gzip -9` command of the system; brotli uses
// the zlib of Node at quality 11 with the default window, the setting of `brotli -11`.
// Precompressed .gz and .br files next to the files are not listed. No dependencies.
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import { execFileSync } from "node:child_process";

const args = process.argv.slice(2);
const json = args.includes("--json");
const dir = args.find((a) => !a.startsWith("--"));
if (!dir) { console.error("usage: node scripts/browser/size-report.mjs <site directory> [--json]"); process.exit(2); }

const files = fs.readdirSync(dir).filter((f) => /\.(wasm|js)$/.test(f)).sort();
const rows = files.map((file) => {
    const data = fs.readFileSync(path.join(dir, file));
    const gzip = execFileSync("gzip", ["-9", "-c", "-n"], { input: data, maxBuffer: 1 << 30 }).length;
    const brotli = zlib.brotliCompressSync(data, {
        params: {
            [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
            [zlib.constants.BROTLI_PARAM_SIZE_HINT]: data.length
        }
    }).length;
    return { file, raw: data.length, gzip, brotli };
});

if (json) {
    console.log(JSON.stringify(rows, null, 2));
} else {
    const mb = (n) => (n / 1e6).toFixed(2).padStart(8);
    console.log(`${"file".padEnd(24)} ${"raw".padStart(12)} ${"gzip -9".padStart(12)} ${"brotli -11".padStart(12)}   (MB: raw, gzip, brotli)`);
    for (const r of rows) {
        console.log(`${r.file.padEnd(24)} ${String(r.raw).padStart(12)} ${String(r.gzip).padStart(12)} ${String(r.brotli).padStart(12)}   ${mb(r.raw)} ${mb(r.gzip)} ${mb(r.brotli)}`);
    }
}
