// Writes precompressed copies next to the files of a browser site: <file>.gz (gzip -9) and
// <file>.br (brotli -11), for static web servers that serve them with Content-Encoding.
//
//   node scripts/browser/compress.mjs <site directory>
//
// Compresses the .wasm, .js, .html, .css and .map files of the directory. Uses the zlib of Node:
// gzip at level 9, brotli at quality 11 with the default window. No dependencies.
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const dir = process.argv[2];
if (!dir) { console.error("usage: node scripts/browser/compress.mjs <site directory>"); process.exit(2); }

for (const file of fs.readdirSync(dir).filter((f) => /\.(wasm|js|html|css|map)$/.test(f)).sort()) {
    const full = path.join(dir, file);
    const data = fs.readFileSync(full);
    const gzip = zlib.gzipSync(data, { level: 9 });
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
