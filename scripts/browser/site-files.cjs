const { createReadStream } = require("node:fs");
const { realpath, stat } = require("node:fs/promises");
const path = require("node:path");

const types = new Map(Object.entries({
    ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm",
    ".css": "text/css", ".map": "application/json", ".json": "application/json", ".png": "image/png",
    ".ico": "image/x-icon", ".svg": "image/svg+xml", ".br": "application/octet-stream", ".gz": "application/gzip",
    ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".webp": "image/webp", ".woff2": "font/woff2",
    ".mp3": "audio/mpeg", ".wav": "audio/wav", ".mp4": "video/mp4", ".webm": "video/webm",
}));

function rangeOf(header, size) {
    const match = /^bytes=(\d*)-(\d*)$/.exec(header);
    if (!match || !match[1] && !match[2]) return;
    const start = match[1] ? Number(match[1]) : Math.max(0, size - Number(match[2]));
    const end = Math.min(size - 1, match[1] && match[2] ? Number(match[2]) : size - 1);
    if (Number.isSafeInteger(start) && Number.isSafeInteger(end) && start <= end && start < size) return { start, end };
}

function end(response, status) {
    response.statusCode = status;
    response.end();
}

/** Serve a GET or HEAD from a physical site root. Both the browser harness and Live App use this
 * path, including the single-range responses needed by native media. */
async function serveSiteFile(root, pathname, request, response) {
    if (request.method !== "GET" && request.method !== "HEAD") return end(response, 405);
    let name;
    try { name = decodeURIComponent(pathname); }
    catch { return end(response, 400); }
    if (!name.startsWith("/") || name.includes("\\") || name.includes("\0") || name.split("/").includes("..")) return end(response, 404);
    const candidate = path.resolve(root, "." + name + (name.endsWith("/") ? "index.html" : ""));
    const file = await realpath(candidate).catch(() => undefined);
    if (!file) return end(response, pathname === "/favicon.ico" ? 204 : 404);
    const relative = path.relative(root, file);
    if (!relative || relative === ".." || relative.startsWith(".." + path.sep) || path.isAbsolute(relative)) return end(response, 404);
    const info = await stat(file).catch(() => undefined);
    if (!info?.isFile()) return end(response, 404);
    if (response.destroyed) return;

    const size = info.size;
    response.setHeader("content-type", types.get(path.extname(file).toLowerCase()) ?? "application/octet-stream");
    response.setHeader("accept-ranges", "bytes");
    let range;
    if (request.headers.range) {
        range = rangeOf(request.headers.range, size);
        if (!range) {
            response.setHeader("content-range", "bytes */" + size);
            return end(response, 416);
        }
        response.statusCode = 206;
        response.setHeader("content-range", "bytes " + range.start + "-" + range.end + "/" + size);
    }
    response.setHeader("content-length", range ? range.end - range.start + 1 : size);
    if (request.method === "HEAD" || size === 0) { response.end(); return; }
    const stream = createReadStream(file, range);
    stream.on("error", error => response.destroy(error));
    response.on("close", () => stream.destroy());
    stream.pipe(response);
}

module.exports = { serveSiteFile };
