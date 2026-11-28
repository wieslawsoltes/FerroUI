// The preceding unmodified vendor module exports the official scanner.
let initialization;
let initialized = false;

export function ferroOnigurumaReady() { return initialized; }

export function ferroInitializeOniguruma(data) {
    if (!initialization) {
        // The Rust slice may move or be freed after this call; retain owned bytes.
        initialization = module.exports.loadWASM(data.slice()).then(() => {
            initialized = true;
        });
    }
    return initialization;
}

export function ferroCompileOniguruma(pattern) {
    return ferroCompileOnigurumaGroup([pattern]);
}

export function ferroCompileOnigurumaGroup(patterns) {
    if (!initialized) throw new Error("initialize_browser_oniguruma must finish before tokenization");
    return new module.exports.OnigScanner(patterns);
}

export function ferroCreateOnigurumaLine(text) { return new module.exports.OnigString(text); }

export function ferroSearchOniguruma(scanner, line, start) {
    const match = scanner.findNextMatchSync(line, line.convertUtf8OffsetToUtf16(start));
    if (!match) return null;
    const captures = new Uint32Array(1 + match.captureIndices.length * 2);
    captures[0] = match.index;
    for (let i = 0; i < match.captureIndices.length; i++) {
        const { start, end } = match.captureIndices[i];
        const pair = 1 + i * 2;
        if (start < 0 || end < start || end > line.utf16Length) {
            captures[pair] = captures[pair + 1] = 0xffffffff;
        } else {
            captures[pair] = line.convertUtf16OffsetToUtf8(start);
            captures[pair + 1] = line.convertUtf16OffsetToUtf8(end);
        }
    }
    return captures;
}

export function ferroDisposeOnigurumaLine(line) { line.dispose(); }
export function ferroDisposeOniguruma(scanner) { scanner.dispose(); }
