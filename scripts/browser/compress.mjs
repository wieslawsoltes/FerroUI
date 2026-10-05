// Writes precompressed copies next to the files of a browser site: <file>.gz (gzip -9) and
// <file>.br (brotli -11), for static web servers that serve them with Content-Encoding.
//
//   node scripts/browser/compress.mjs <site directory>
//
// Compresses the .wasm, .js, .html, .css and .map files of the directory: gzip with the `gzip -9`
// command of the system (it compresses the module about 1% better than the zlib of Node), brotli
// with the zlib of Node at quality 11 and the default window, the setting of `brotli -11`.
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";
import { execFileSync } from "node:child_process";

const dir = process.argv[2];
if (!dir) { console.error("usage: node scripts/browser/compress.mjs <site directory>"); process.exit(2); }

for (const file of fs.readdirSync(dir).filter((f) => /\.(wasm|js|html|css|map)$/.test(f)).sort()) {
    const full = path.join(dir, file);
    const data = fs.readFileSync(full);
    const gzip = execFileSync("gzip", ["-9", "-c", "-n"], { input: data, maxBuffer: 1 << 30 });
    const brotli = zlib.brotliCompressSync(data, {
        params: {
            [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
            [zlib.constants.BROTLI_PARAM_SIZE_HINT]: data.length
        }
    });
    fs.writeFileSync(full + ".gz", gzip);
    fs.writeFileSync(full + ".br", brotli);
    console.log(`${file}: ${data.length} bytes, ${gzip.length} gzip, ${brotli.length} brotli`);
}
