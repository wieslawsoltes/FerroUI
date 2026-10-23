import assert from "node:assert/strict";
import { test } from "node:test";
import { radiusFromPointerInput } from "./corner-radius-handles-browser.mjs";

const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-12, `${actual} != ${expected}`);

test("the b7 Alt-radius failure matches delivered float32 input without weakening the authored-radius assertion", () => {
    // Run 39869066118's two browser modes exported this exact authored result.
    // The press coordinates are in its native pointer evidence; float32 of the
    // deterministic move request reproduces the browser's coordinate rounding.
    const requestedDown = { x: 1170.674381805361, y: 334.2480288296134 };
    const requestedMove = { x: 1144.8886999335487, y: 352.0560462636321 };
    const down = { offsetX: 1170.6744384765625, offsetY: 334.2480163574219 };
    const move = { offsetX: Math.fround(requestedMove.x), offsetY: Math.fround(requestedMove.y) };
    assert.equal(down.offsetX, Math.fround(requestedDown.x));
    assert.equal(down.offsetY, Math.fround(requestedDown.y));
    const actual = radiusFromPointerInput({
        node: { rotation: 11, flip_x: false, flip_y: false,
            affine_transform: { a: 1, b: 0.08, c: 0.1, d: 1 } },
        localPerRadius: { x: -1, y: 1 }, authoredRadius: 18.499999964629964,
        zoom: 3.8958215738438757, down, move,
    });
    near(actual, 24.750016939075536);
    assert.ok(Math.abs(actual - 24.75) > 1e-6,
        "Comparing real input with the ideal requested target caused the historical timeout");
});

test("the pointer oracle preserves local radius units under reflection, shear, rotation and viewport zoom", () => {
    // This fixture's world basis is (-0.25, -2), (-1.5, -0.4).
    // A local radius direction (-1, 1) therefore becomes (-1.25, 1.6).
    const input = {
        node: { rotation: 90, flip_x: true, flip_y: false,
            affine_transform: { a: 2, b: 0.25, c: 0.4, d: 1.5 } },
        localPerRadius: { x: -1, y: 1 }, authoredRadius: 8.5, zoom: 1.75,
        down: { offsetX: 123.5, offsetY: -44.25 },
        move: { offsetX: 123.5 - 1.25 * 4.25 * 1.75, offsetY: -44.25 + 1.6 * 4.25 * 1.75 },
    };
    near(radiusFromPointerInput(input), 12.75);
    // Local motion (3, 3) is perpendicular to the radius direction even though
    // the sheared world displacement is not perpendicular to its world axis.
    near(radiusFromPointerInput({ ...input,
        move: { offsetX: 123.5 - 5.25 * 1.75, offsetY: -44.25 - 7.2 * 1.75 } }), 8.5);
});

test("invalid geometry fails the oracle and negative delivered radius clamps to the authored lower bound", () => {
    const input = {
        node: { rotation: 0 }, localPerRadius: { x: 1, y: 1 }, authoredRadius: 8, zoom: 1,
        down: { offsetX: 100, offsetY: 100 }, move: { offsetX: 80, offsetY: 80 },
    };
    assert.equal(radiusFromPointerInput(input), 0);
    assert.throws(() => radiusFromPointerInput({ ...input, zoom: 0 }));
    assert.throws(() => radiusFromPointerInput({ ...input,
        node: { rotation: 0, affine_transform: { a: 0, b: 0, c: 0, d: 1 } } }));
});
