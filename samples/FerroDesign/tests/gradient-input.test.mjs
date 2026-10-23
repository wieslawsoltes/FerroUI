import assert from "node:assert/strict";
import { test } from "node:test";
import { angularInsertionFromPointerInput, angularPaintFromPointerInput, gradientColorAtOffset } from "./gradient-input.mjs";

const near = (actual, expected, tolerance = 1e-10) => assert.ok(Math.abs(actual - expected) < tolerance, `${actual} != ${expected}`);
const node = { x: 240, y: 130, width: 240, height: 160, rotation: 17, flip_x: true, flip_y: false,
    affine_transform: { a: 1.1, b: 0.15, c: 0.2, d: 0.9, e: 3, f: -5 } };
const plain = { x: 80, y: 80, width: 240, height: 160, rotation: 0 };
const paint = { type: "angular_gradient", center: { x: 0.5, y: 0.5 }, angle: 359, stops: [
    { offset: 0, color: { r: 255, g: 0, b: 0, a: 255 } },
    { offset: 0.5, color: { r: 0, g: 255, b: 0, a: 192 } },
    { offset: 0.5, color: { r: 0, g: 0, b: 255, a: 128 } },
    { offset: 1, color: { r: 0, g: 0, b: 255, a: 255 } },
] };

test("quantized affine pointer samples preserve the strict angular oracle across the 360-degree seam", () => {
    // Deterministic float32 delivery at the browser-observed camera scale.
    // These requested logical rotations are 15, 30 and 45 degrees; their real
    // input geometry differs enough to fail a comparison with the ideal path.
    const down = { offsetX: 1170.6744384765625, offsetY: 334.2480163574219 };
    const moves = [
        { offsetX: 1077.9075927734375, offsetY: 327.23419189453125 },
        { offsetX: 981.4068603515625, offsetY: 337.66802978515625 },
        { offsetX: 887.7485961914062, offsetY: 364.8385009765625 },
    ];
    const expected = [374.0000107515067, 389.00001212751073, 404.000006089787];
    for (let count = 1; count <= moves.length; count++) {
        const actual = angularPaintFromPointerInput({ node, paint, part: { kind: "end" },
            zoom: 3.8958215738438757, down, moves: moves.slice(0, count) });
        near(actual.angle, expected[count - 1]);
        assert.ok(Math.abs(actual.angle - (359 + count * 15)) > 1e-6);
        assert.deepEqual({ ...actual, angle: paint.angle }, paint,
            "Actual input changes only the angle, preserving stop identity and exact RGBA values");
    }
});

test("logical local-pixel rays ignore displaced markers and preserve source-stop identity on a nonsquare box", () => {
    const original = { ...structuredClone(paint), angle: 0 };
    const down = { offsetX: 1000, offsetY: -200 };
    const end = angularPaintFromPointerInput({ node: plain, paint: original, part: { kind: "end" }, zoom: 2,
        down, moves: [{ offsetX: 1160, offsetY: -40 }] });
    near(end.angle, 90);
    const stop = angularPaintFromPointerInput({ node: plain, paint: original, part: { kind: "stop", stop: 2 }, zoom: 2,
        down, moves: [{ offsetX: 1000 - 80 * Math.SQRT1_2 * 2, offsetY: -200 + (80 * Math.SQRT1_2 - 80) * 2 }] });
    const expected = structuredClone(original); expected.stops[2].offset = 0.625;
    near(stop.stops[2].offset, 0.625);
    expected.stops[2].offset = stop.stops[2].offset;
    assert.deepEqual(stop, expected);
    assert.equal(original.stops[2].offset, 0.5, "The oracle cannot mutate its pre-gesture fixture");
});

test("angular input accumulates winding and restores the complete original paint on exact drag-back", () => {
    const original = { ...structuredClone(paint), angle: 170 }, down = { offsetX: 400, offsetY: 500 };
    const radians = angle => angle * Math.PI / 180;
    const moves = [30, 120, 210, 300, 390].map(change => ({
        offsetX: down.offsetX + 80 * (Math.sin(radians(170 + change)) - Math.sin(radians(170))),
        offsetY: down.offsetY + 80 * (Math.cos(radians(170)) - Math.cos(radians(170 + change))),
    }));
    near(angularPaintFromPointerInput({ node: plain, paint: original, part: { kind: "end" }, zoom: 1, down, moves }).angle, 560);
    assert.deepEqual(angularPaintFromPointerInput({ node: plain, paint: original, part: { kind: "end" }, zoom: 1,
        down, moves: [...moves, down] }), original);
});

test("insertion derives the actual affine ray before exact RGBA interpolation at a rounding boundary", () => {
    const original = { ...structuredClone(paint), angle: 404 };
    const offset = 0.25 - 1e-8, angle = (original.angle + offset * 360) * Math.PI / 180;
    const x = 120 + Math.sin(angle) * 80, y = 80 - Math.cos(angle) * 80;
    const sx = -(1.1 * x + 0.2 * y + 3 - 120), sy = 0.15 * x + 0.9 * y - 5 - 80;
    const sine = Math.sin(17 * Math.PI / 180), cosine = Math.cos(17 * Math.PI / 180);
    const nativeWorld = { x: 240 + 120 + sx * cosine - sy * sine, y: 130 + 80 + sx * sine + sy * cosine };
    const actual = angularInsertionFromPointerInput({ node, paint: original, nativeWorld });
    near(actual.stops[4].offset, offset, 1e-12);
    assert.deepEqual(actual.stops[4].color, { r: 128, g: 127, b: 0, a: 224 });
    assert.deepEqual(gradientColorAtOffset(original.stops, 0.25), { r: 128, g: 128, b: 0, a: 224 });
    assert.deepEqual(actual.stops.slice(0, 4), original.stops);
    assert.deepEqual({ ...actual, stops: original.stops }, original);
    assert.equal(original.stops.length, 4);
});

test("stable authored tie order supplies the same exact color on and after a repeated offset", () => {
    assert.deepEqual(gradientColorAtOffset(paint.stops, 0.5), paint.stops[2].color);
    assert.deepEqual(gradientColorAtOffset(paint.stops, 0.75), { r: 0, g: 0, b: 255, a: 192 });
    assert.deepEqual(gradientColorAtOffset(paint.stops, 1), paint.stops[3].color);
});
