// Exact compression measurements shared by reports of the individual and combined sites.
// Content, algorithm settings and implementation versions are all part of the key. No source
// timestamp or pathname is used as evidence of content identity.
import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import util from "node:util";
import zlib from "node:zlib";

const gzip = util.promisify(zlib.gzip);
const brotli = util.promisify(zlib.brotliCompress);
const schema = 1;
const implementation = JSON.stringify({
    schema,
    node: process.version,
    zlib: process.versions.zlib,
    brotli: process.versions.brotli,
    platform: process.platform,
    arch: process.arch,
    gzipLevel: 9,
    brotliQuality: 11,
    brotliSizeHint: "input-length",
});

function recordChecksum(key, sizes) {
    return crypto.createHash("sha256").update(JSON.stringify({ schema, key, sizes })).digest("hex");
}

export class CompressionCache {
    #directory;
    #pending = new Map();
    hits = 0;
    misses = 0;

    constructor(directory) { this.#directory = directory; }

    measure(content) {
        const key = crypto.createHash("sha256").update(implementation).update("\0").update(content).digest("hex");
        if (this.#pending.has(key)) {
            this.hits++;
            return this.#pending.get(key);
        }
        const pending = this.#measure(content, key);
        this.#pending.set(key, pending);
        return pending;
    }

    async #measure(content, key) {
        const file = this.#directory && path.join(this.#directory, `${key}.json`);
        if (file) {
            try {
                const record = JSON.parse(await fs.readFile(file, "utf8"));
                if (record.schema === schema && record.key === key && Array.isArray(record.sizes)
                    && record.sizes.length === 3 && record.sizes[0] === content.length
                    && record.sizes.every(value => Number.isSafeInteger(value) && value >= 0)
                    && record.checksum === recordChecksum(key, record.sizes)) {
                    this.hits++;
                    return record.sizes;
                }
            } catch { /* Missing, damaged or inaccessible cache: compute the actual result. */ }
        }
        this.misses++;
        const [gzipped, brotlied] = await Promise.all([
            gzip(content, { level: 9 }),
            brotli(content, {
                params: {
                    [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
                    [zlib.constants.BROTLI_PARAM_SIZE_HINT]: content.length,
                },
            }),
        ]);
        const sizes = [content.length, gzipped.length, brotlied.length];
        if (file) {
            const temporary = `${file}.${process.pid}.${crypto.randomUUID()}.tmp`;
            try {
                await fs.mkdir(this.#directory, { recursive: true });
                await fs.writeFile(temporary, `${JSON.stringify({ schema, key, sizes, checksum: recordChecksum(key, sizes) })}\n`, { flag: "wx" });
                await fs.rename(temporary, file);
            } catch {
                // Measurements remain valid when the optional cache cannot be written.
            } finally {
                await fs.rm(temporary, { force: true }).catch(() => {});
            }
        }
        return sizes;
    }
}
