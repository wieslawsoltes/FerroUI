// Pure coverage math shared by Node regressions and browser evaluation. Keep
// areaAverageAlpha self-contained: Playwright executes this exact function in
// the page with a JSHandle to its RGBA buffer, without transferring that buffer.
export function areaAverageAlpha({ pixels, width, height, scale }) {
    if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 ||
        width * height > 28000 || !Number.isInteger(scale) || scale < 1 || scale > 16) {
        throw new RangeError("Coverage requires 1..28000 output pixels and an integer scale from 1 to 16.");
    }
    const sourceWidth = width * scale, sourceHeight = height * scale;
    if (!(pixels instanceof Uint8Array || pixels instanceof Uint8ClampedArray) ||
        pixels.length !== sourceWidth * sourceHeight * 4) {
        throw new RangeError("Coverage requires an exact, tightly packed RGBA8 buffer.");
    }
    const alpha = new Array(width * height), area = scale * scale;
    for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
            let sum = 0;
            for (let sy = 0; sy < scale; sy++) {
                const start = ((y * scale + sy) * sourceWidth + x * scale) * 4 + 3;
                for (let sx = 0; sx < scale; sx++) sum += pixels[start + sx * 4];
            }
            alpha[y * width + x] = sum / area;
        }
    }
    return alpha;
}

// A predeclared numerical stability gate, not a proof of absolute raster error.
// Keep fractional averages until after this comparison; round only the final
// reference used by the original byte-alpha image comparison.
export function coverageConvergence(coarse, fine) {
    if (!Array.isArray(coarse) || !Array.isArray(fine) || coarse.length !== fine.length ||
        coarse.length < 1 || coarse.length > 28000) {
        throw new RangeError("Coverage references must have the same bounded pixel count.");
    }
    let maxAlphaDifference = 0, absoluteAlphaAreaDifference = 0, oppositeOpaqueTransparent = 0;
    for (let i = 0; i < coarse.length; i++) {
        const a = coarse[i], b = fine[i];
        if (!Number.isFinite(a) || !Number.isFinite(b) || a < 0 || a > 255 || b < 0 || b > 255) {
            throw new RangeError("Coverage values must be finite alpha from 0 to 255.");
        }
        const delta = Math.abs(a - b);
        maxAlphaDifference = Math.max(maxAlphaDifference, delta);
        absoluteAlphaAreaDifference += delta / 255;
        oppositeOpaqueTransparent += Number((a === 0 && b === 255) || (a === 255 && b === 0));
    }
    return {
        maxAlphaDifference,
        absoluteAlphaAreaDifference: Number(absoluteAlphaAreaDifference.toFixed(6)),
        oppositeOpaqueTransparent,
        maxAllowedAlphaDifference: 4,
        converged: maxAlphaDifference <= 4 && oppositeOpaqueTransparent === 0,
    };
}
