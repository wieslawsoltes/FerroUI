// The preceding unmodified vendor module exports the official scanner.
let initialization;
let initialized = false;
let cachedText;
let cachedOnigString;

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
    if (!initialized) throw new Error("initialize_browser_oniguruma must finish before tokenization");
    return new module.exports.OnigScanner([pattern]);
}

export function ferroSearchOniguruma(scanner, text, start) {
    if (cachedText !== text) {
        if (cachedOnigString) cachedOnigString.dispose();
        cachedText = text;
        cachedOnigString = new module.exports.OnigString(text);
    }
    const match = scanner.findNextMatchSync(cachedOnigString, start);
    if (!match) return null;
    return match.captureIndices.map(capture => [capture.start, capture.end]);
}

export function ferroDisposeOniguruma(scanner) { scanner.dispose(); }
