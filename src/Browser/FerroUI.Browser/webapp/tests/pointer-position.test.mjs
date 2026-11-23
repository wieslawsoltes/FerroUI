// The position of a pointer in the view (modules/ferroui/pointerPosition.ts): measured on the canvas's
// content box and converted with the scale of what is drawn there, at fractional device pixel ratios.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

const source = await readFile(new URL("../modules/ferroui/pointerPosition.ts", import.meta.url), "utf8");
const { code } = await transform(source, { loader: "ts", format: "esm", target: "es2020" });
const { measureViewBox, viewPoint, setViewCanvas, clearViewCanvas, viewBoxOf } =
    await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

const near = (actual, expected, message) =>
    assert.ok(Math.abs(actual - expected) < 1e-9, `${message}: ${actual}, expected ${expected}`);

// The computed style of a canvas: `width` and `height` as `box-sizing` reports them.
function style({ width, height, border = 0, padding = 0, boxSizing = "content-box" }) {
    const outer = boxSizing === "border-box" ? 2 * (border + padding) : 0;
    return {
        boxSizing,
        width: `${width + outer}px`,
        height: `${height + outer}px`,
        borderLeftWidth: `${border}px`,
        borderRightWidth: `${border}px`,
        borderTopWidth: `${border}px`,
        borderBottomWidth: `${border}px`,
        paddingLeft: `${padding}px`,
        paddingRight: `${padding}px`,
        paddingTop: `${padding}px`,
        paddingBottom: `${padding}px`
    };
}

// A canvas of `width` x `height` CSS pixels at (`left`, `top`) in the client area, scaled by `scale` (a
// transform), whose backing store the browser snapped to device pixels at `ratio`, rendered at `scaling`.
function canvas({ left, top, width, height, border = 0, padding = 0, boxSizing, scale = 1, ratio, scaling = ratio }) {
    const outer = 2 * (border + padding);
    const rect = { left, top, width: (width + outer) * scale, height: (height + outer) * scale };
    const render = { width: Math.round(width * ratio), height: Math.round(height * ratio), scaling };
    const box = measureViewBox(rect, style({ width, height, border, padding, boxSizing }), render);
    // The corner of the content box in client coordinates, and the client size of the view.
    const inset = (border + padding) * scale;
    return { box, corner: [left + inset, top + inset], extent: [width * scale, height * scale], render };
}

const fixtures = [
    { name: "plain", left: 0, top: 0, width: 400.25, height: 300.5 },
    { name: "at a fractional corner", left: 10.4, top: 20.6, width: 333.33, height: 211.7 },
    { name: "with a border", left: 7.5, top: 3.25, width: 400.25, height: 300.5, border: 3 },
    { name: "with a border and padding", left: 0.3, top: 0.7, width: 401, height: 299, border: 2, padding: 4 },
    { name: "border-box", left: 12.125, top: 8.875, width: 400.25, height: 300.5, border: 3, padding: 4, boxSizing: "border-box" },
    { name: "transformed", left: 5.5, top: 6.5, width: 400.25, height: 300.5, border: 1, scale: 0.5 }
];

for (const ratio of [1, 1.25, 1.5, 1.75, 2]) {
    for (const fixture of fixtures) {
        test(`the content box maps onto the view at ${ratio}x: ${fixture.name}`, () => {
            const { box, corner, extent, render } = canvas({ ...fixture, ratio });
            const client = [render.width / render.scaling, render.height / render.scaling];
            for (const [fx, fy] of [[0, 0], [1, 1], [0.5, 0.25], [0.1, 0.9]]) {
                const [x, y] = viewPoint(box, corner[0] + fx * extent[0], corner[1] + fy * extent[1]);
                near(x, fx * client[0], `x at ${fx}`);
                near(y, fy * client[1], `y at ${fy}`);
            }
        });
    }
}

test("where the backing store and the scaling agree, the position is in CSS pixels of the content box", () => {
    // What `offsetX` and `offsetY` of an event on the canvas give, to within the rounding of the
    // backing store (less than one device pixel over the whole canvas).
    for (const ratio of [1.25, 1.5, 1.75]) {
        const { box, corner } = canvas({ left: 10.4, top: 20.6, width: 333.33, height: 211.7, ratio });
        const [x, y] = viewPoint(box, corner[0] + 200.5, corner[1] + 100.25);
        assert.ok(Math.abs(x - 200.5) < 1 / ratio, `x ${x} at ${ratio}x`);
        assert.ok(Math.abs(y - 100.25) < 1 / ratio, `y ${y} at ${ratio}x`);
    }
});

test("a scaling that differs from the ratio of the backing store follows the picture, not the CSS pixels", () => {
    // The backing store is 1.5 device pixels per CSS pixel and the view is rendered at 2: the client
    // size of the view is three quarters of the CSS size, and the picture is drawn over all of it.
    const { box, corner, extent } = canvas({ left: 0, top: 0, width: 800, height: 600, ratio: 1.5, scaling: 2 });
    const [x, y] = viewPoint(box, corner[0] + extent[0], corner[1] + extent[1]);
    near(x, 600, "far corner x");
    near(y, 450, "far corner y");
    const [cx, cy] = viewPoint(box, 400, 300);
    near(cx, 300, "centre x");
    near(cy, 225, "centre y");
});

test("a canvas without a size or a scaling yet maps one unit per CSS pixel", () => {
    const hidden = measureViewBox({ left: 0, top: 0, width: 0, height: 0 },
        { ...style({ width: 0, height: 0 }), width: "auto", height: "auto" }, { width: 0, height: 0, scaling: 0 });
    assert.deepEqual(viewPoint(hidden, 12.5, 7.25), [12.5, 7.25]);
});

test("a view measures on its registered canvas and on its element without one", () => {
    const fixtureStyle = style({ width: 400, height: 300, border: 2 });
    globalThis.getComputedStyle = () => fixtureStyle;
    try {
        const container = { getBoundingClientRect: () => ({ left: 50, top: 60, width: 404, height: 304 }) };
        const surface = {
            canvas: { getBoundingClientRect: () => ({ left: 50.5, top: 60.5, width: 404, height: 304 }) },
            width: 600,
            height: 450,
            scaling: 1.5
        };
        assert.deepEqual(viewPoint(viewBoxOf(container), 60, 70), [10, 10]);
        setViewCanvas(container, surface);
        const [x, y] = viewPoint(viewBoxOf(container), 52.5 + 100, 62.5 + 50);
        near(x, 100, "x");
        near(y, 50, "y");
        // Another surface of the same element is not cleared by the old one.
        clearViewCanvas(container, { ...surface });
        near(viewPoint(viewBoxOf(container), 52.5, 62.5)[0], 0, "still the canvas");
        clearViewCanvas(container, surface);
        assert.deepEqual(viewPoint(viewBoxOf(container), 60, 70), [10, 10]);
    } finally {
        delete globalThis.getComputedStyle;
    }
});
