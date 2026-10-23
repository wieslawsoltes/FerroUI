import assert from "node:assert/strict";
import test from "node:test";
import { areaAverageAlpha, coverageConvergence } from "./alpha-coverage.mjs";

function rgba(width, height, alpha) {
    const data = new Uint8ClampedArray(width * height * 4);
    for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
            // Deliberately unrelated RGB must not contribute to coverage.
            data.set([173, 29, 241, alpha(x, y)], (y * width + x) * 4);
        }
    }
    return data;
}

test("nonoverlapping RGBA blocks keep row/column indexing and conserve alpha area", () => {
    const pixels = rgba(4, 4, (x, y) => y * 40 + x * 7);
    const result = areaAverageAlpha({ pixels, width: 2, height: 2, scale: 2 });
    assert.deepEqual(result, [23.5, 37.5, 103.5, 117.5]);
    const sourceSum = Array.from({ length: 16 }, (_, i) => pixels[i * 4 + 3]).reduce((a, b) => a + b, 0);
    assert.equal(result.reduce((a, b) => a + b, 0) * 4, sourceSum);
});

test("opaque and transparent coverage stays exact at both declared reference scales", () => {
    for (const scale of [16, 32]) {
        const pixels = rgba(3 * scale, 2 * scale, (x, y) => x < scale && y < scale ? 255 : 0);
        assert.deepEqual(areaAverageAlpha({ pixels, width: 3, height: 2, scale }), [255, 0, 0, 0, 0, 0]);
    }
});

test("fractionally translated analytic rectangle coverage survives byte rounding and averaging", () => {
    // Rectangle [0.3,0.4]..[2.15,1.6]. Its exact overlap with each unit pixel is
    // separable; no rasterizer, downscaler or copy of the averaging loop is used.
    const overlap = (a, b, c, d) => Math.max(0, Math.min(b, d) - Math.max(a, c));
    const exact = [107.1, 153, 22.95, 107.1, 153, 22.95], references = [];
    for (const scale of [16, 32]) {
        const pixels = rgba(3 * scale, 2 * scale, (x, y) => Math.round(
            overlap(x / scale, (x + 1) / scale, 0.3, 2.15) *
            overlap(y / scale, (y + 1) / scale, 0.4, 1.6) * scale * scale * 255));
        const averaged = areaAverageAlpha({ pixels, width: 3, height: 2, scale });
        averaged.forEach((alpha, index) => assert.ok(Math.abs(alpha - exact[index]) <= 0.5,
            `Output pixel ${index} exceeds the maximum per-source-byte rounding error`));
        const sourceSum = Array.from({ length: pixels.length / 4 }, (_, i) => pixels[i * 4 + 3])
            .reduce((a, b) => a + b, 0);
        assert.equal(averaged.reduce((a, b) => a + b, 0) * scale * scale, sourceSum);
        references.push(averaged);
    }
    assert.equal(coverageConvergence(...references).converged, true);
});

test("sequential strips preserve coverage across row seams and a short final strip", () => {
    for (const scale of [16, 32]) {
        const width = 3, height = 5;
        const pixels = rgba(width * scale, height * scale, (x, y) => (x * 29 + y * 53) % 256);
        const complete = areaAverageAlpha({ pixels, width, height, scale }), strips = [];
        for (let row = 0; row < height; row += 2) {
            const rows = Math.min(2, height - row), stride = width * scale * 4;
            const strip = pixels.subarray(row * scale * stride, (row + rows) * scale * stride);
            strips.push(...areaAverageAlpha({ pixels: strip, width, height: rows, scale }));
        }
        assert.deepEqual(strips, complete, "Integer-row strip boundaries cannot shift or duplicate a sample");
    }
});

test("unstable references are rejected even when the finer raster would match a candidate", () => {
    assert.equal(coverageConvergence([0, 60, 255], [0, 64, 255]).converged, true);
    const unstable = coverageConvergence([0, 59.5, 255], [0, 64, 255]);
    assert.equal(unstable.maxAlphaDifference, 4.5);
    assert.equal(unstable.converged, false, "No rounding may hide a failed convergence gate");
    const opposite = coverageConvergence([0, 255], [255, 0]);
    assert.equal(opposite.oppositeOpaqueTransparent, 2);
    assert.equal(opposite.converged, false);
});

test("invalid dimensions, excessive work and malformed coverage fail explicitly", () => {
    for (const scale of [0, 1.5, 33]) {
        assert.throws(() => areaAverageAlpha({ pixels: new Uint8Array(4), width: 1, height: 1, scale }), RangeError);
    }
    assert.throws(() => areaAverageAlpha({ pixels: new Uint8Array(289 * 100 * 4), width: 289, height: 100, scale: 1 }), RangeError);
    assert.throws(() => areaAverageAlpha({ pixels: new Uint8Array(15), width: 1, height: 1, scale: 2 }), RangeError);
    assert.throws(() => areaAverageAlpha({ pixels: [0, 0, 0, 255], width: 1, height: 1, scale: 1 }), RangeError);
    assert.throws(() => coverageConvergence([0], [0, 0]), RangeError);
    for (const value of [NaN, Infinity, -1, 256]) {
        assert.throws(() => coverageConvergence([value], [0]), RangeError);
    }
});

test("the larger polygon fixture retains the exact work bound for averaging and convergence", () => {
    const pixels = new Uint8Array(180 * 160 * 4).fill(255);
    const alpha = areaAverageAlpha({ pixels, width: 180, height: 160, scale: 1 });
    assert.equal(alpha.length, 28800);
    assert.ok(alpha.every(value => value === 255));
    assert.equal(coverageConvergence(alpha, alpha).converged, true);
    const oversized = [...alpha, 255];
    assert.throws(() => coverageConvergence(oversized, oversized), RangeError);
});
