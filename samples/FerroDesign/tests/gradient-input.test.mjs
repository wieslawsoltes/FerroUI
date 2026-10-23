import assert from "node:assert/strict";
import { test } from "node:test";
import { angularInsertionFromPointerInput, angularPaintFromPointerInput, assertGradientDoubleClickInput, gradientColorAtOffset } from "./gradient-input.mjs";

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

// These are validator fixtures, not claims of executed browser input. Browser
// executions must supply the real passive records and routed native presses.
function doubleClickInput() {
    const before = { pointerPresses: { serial: 10, entries: [] }, zoom: 2, pan: { x: 10, y: -6 },
        chrome: { CanvasHost: { x: 120, y: 80, width: 800, height: 600 } } };
    const presses = [1, 2].map((clickCount, index) => ({ serial: 11 + index, clickCount, buttons: 1,
        timestamp: 8000 + index * 175, screen: { x: 70, y: 74 }, world: { x: 30, y: 40 },
        pan: { x: 10, y: -6 }, zoom: 2 }));
    const times = [1000, 1004, 1175, 1179];
    const events = ["pointerdown", "pointerup", "pointerdown", "pointerup"].flatMap((type, index) =>
        ["capture", "bubble"].map((phase, phaseIndex) => ({ type, phase, trusted: true, pointerType: "mouse",
            pointerId: 1, button: 0, buttons: index % 2 === 0 ? 1 : 0,
            x: 190, y: 154, offsetX: 190, offsetY: 154,
            eventTimeMs: times[index], observedAtMs: times[index] + 8 + phaseIndex })));
    return { before, after: { ...structuredClone(before), pointerPresses: { serial: 12, entries: presses } },
        requested: { page: { x: 190, y: 154 }, world: { x: 30, y: 40 } }, observation: { events, droppedEvents: 0 } };
}

test("double-click evidence requires one native 1,2 sequence at the delivered point without changing its records", () => {
    const input = doubleClickInput(), original = structuredClone(input);
    const result = assertGradientDoubleClickInput(input);
    assert.equal(result.domIntervalMs, 175);
    assert.equal(result.nativeIntervalMs, 175);
    assert.equal(result.press.serial, 12);
    assert.deepEqual(result.press.world, { x: 30, y: 40 });
    assert.deepEqual(input, original);
});

test("the unchanged 500 ms bound uses creation and native timestamps, not delayed observer processing", () => {
    const input = doubleClickInput();
    for (const event of input.observation.events.slice(4)) {
        event.eventTimeMs += 325;
        event.observedAtMs += 819;
    }
    input.after.pointerPresses.entries[1].timestamp += 325;
    const result = assertGradientDoubleClickInput(input);
    assert.equal(result.domIntervalMs, 500);
    assert.equal(result.nativeIntervalMs, 500);
    assert.equal(input.observation.events[4].observedAtMs - input.observation.events[0].observedAtMs, 994);
    input.after.pointerPresses.entries[1].timestamp++;
    assert.throws(() => assertGradientDoubleClickInput(input), /unchanged 500 ms platform limit: 501/);
    input.after.pointerPresses.entries[1].timestamp--;
    for (const event of input.observation.events.slice(4)) event.eventTimeMs++;
    assert.throws(() => assertGradientDoubleClickInput(input), /creation interval must remain within 500 ms: 501/);
});

test("two native singles cannot pass merely because the browser requested a double-click", () => {
    const input = doubleClickInput();
    input.after.pointerPresses.entries[1].clickCount = 1;
    input.after.pointerPresses.entries[1].timestamp = input.after.pointerPresses.entries[0].timestamp + 994;
    assert.throws(() => assertGradientDoubleClickInput(input), /native mouse device must recognize one double-click/);
});

test("missing, extra, cancelled, untrusted or shifted browser presses cannot establish insertion input", () => {
    const cases = [
        input => { input.observation.droppedEvents = 1; },
        input => { input.observation.events.splice(4, 1); },
        input => { input.observation.events.splice(5, 1); },
        input => { input.observation.events.push(...structuredClone(input.observation.events.slice(4))); },
        input => { input.observation.events.push({ type: "pointercancel", phase: "capture" }); },
        input => { input.observation.events[4].trusted = false; },
        input => { input.observation.events[4].x++; },
        input => { input.observation.events[4].offsetX++; },
        input => { input.observation.events[4].pointerId++; },
        input => { input.observation.events[4].buttons = 2; },
    ];
    for (const change of cases) {
        const input = doubleClickInput(); change(input);
        assert.throws(() => assertGradientDoubleClickInput(input));
    }
});

test("native counts alone cannot hide another hit point, missing press or changing viewport", () => {
    const cases = [
        input => { input.after.pointerPresses.serial++; },
        input => { input.after.pointerPresses.entries.shift(); },
        input => { input.after.pointerPresses.entries[1].screen.x++; },
        input => { input.after.pointerPresses.entries[1].world.y++; },
        input => { input.after.chrome.CanvasHost.x++; },
        input => { input.after.pan.x++; },
        input => { input.after.pointerPresses.entries.forEach(press => { press.screen.x += 2; press.world.x++; }); },
    ];
    for (const change of cases) {
        const input = doubleClickInput(); change(input);
        assert.throws(() => assertGradientDoubleClickInput(input));
    }
});

test("the requested point permits only exact or Chromium float32 delivery before matching native coordinates", () => {
    const input = doubleClickInput();
    input.requested = { page: { x: 190.1, y: 154.2 }, world: { x: 30.05, y: 40.1 } };
    const x = Math.fround(190.1), y = Math.fround(154.2);
    for (const event of input.observation.events) Object.assign(event, { x, y, offsetX: x, offsetY: y });
    for (const press of input.after.pointerPresses.entries) {
        press.screen = { x: x - 120, y: y - 80 };
        press.world = { x: (x - 130) / 2, y: (y - 74) / 2 };
    }
    assertGradientDoubleClickInput(input);
    input.requested.page.x += 0.01;
    assert.throws(() => assertGradientDoubleClickInput(input), /delivered double-click must match the requested x/);
});
