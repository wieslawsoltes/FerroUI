import assert from "node:assert/strict";

// Inspect the real input only after both presses have finished. The native
// mouse device owns click classification; DOM click detail is not a substitute.
export function assertGradientDoubleClickInput({ before, after, requested, observation }) {
    assert.equal(observation.droppedEvents, 0, "Gradient insertion input evidence must be complete");
    assert.equal(observation.events.filter(event => event.type === "pointercancel").length, 0,
        "Gradient insertion cannot contain a cancelled pointer");
    const pointerEvents = phase => observation.events.filter(event => event.phase === phase &&
        (event.type === "pointerdown" || event.type === "pointerup"));
    const events = pointerEvents("capture"), bubble = pointerEvents("bubble");
    const sequence = ["pointerdown", "pointerup", "pointerdown", "pointerup"];
    assert.deepEqual(events.map(event => event.type), sequence, "One ordered browser double-click is required");
    assert.deepEqual(bubble.map(event => event.type), sequence, "Both presses and releases must traverse the native input target");
    assert.ok(Number.isSafeInteger(events[0].pointerId) && events[0].pointerId > 0, "The actual mouse pointer identity is required");
    for (const [index, event] of events.entries()) {
        assert.ok(event.trusted && event.pointerType === "mouse", "Gradient insertion must use trusted mouse input");
        assert.equal(event.button, 0);
        assert.equal(event.buttons, index % 2 === 0 ? 1 : 0);
        assert.equal(event.pointerId, events[0].pointerId);
        for (const key of ["x", "y", "offsetX", "offsetY"]) {
            assert.ok(Number.isFinite(event[key]));
            assert.equal(event[key], events[0][key], "Every double-click event must use the same " + key);
        }
        assert.ok(Number.isFinite(event.eventTimeMs) && Number.isFinite(event.observedAtMs) &&
            event.observedAtMs >= event.eventTimeMs, "Browser creation and observation times must be retained");
        if (index > 0) assert.ok(event.eventTimeMs >= events[index - 1].eventTimeMs, "Browser input timestamps must be ordered");
        for (const key of ["type", "trusted", "pointerType", "pointerId", "button", "buttons", "x", "y", "offsetX", "offsetY", "eventTimeMs"]) {
            assert.equal(bubble[index][key], event[key], "Capture and bubble must describe the same actual event");
        }
    }
    for (const coordinate of ["x", "y"]) {
        // Chromium may quantize the requested CSS coordinate to float32. This
        // permits only that conversion, never a hit-point tolerance or retry.
        assert.ok(events[0][coordinate] === requested.page[coordinate] ||
            events[0][coordinate] === Math.fround(requested.page[coordinate]),
        "The delivered double-click must match the requested " + coordinate);
    }
    const domIntervalMs = events[2].eventTimeMs - events[0].eventTimeMs;
    assert.ok(domIntervalMs >= 0 && domIntervalMs <= 500,
        "Browser press creation interval must remain within 500 ms: " + domIntervalMs);
    const presses = after.pointerPresses.entries.filter(press => press.serial > before.pointerPresses.serial);
    assert.equal(after.pointerPresses.serial, before.pointerPresses.serial + 2, "Exactly two new presses must reach the native canvas");
    assert.deepEqual(presses.map(press => press.serial), [before.pointerPresses.serial + 1, before.pointerPresses.serial + 2]);
    assert.deepEqual(presses.map(press => press.clickCount), [1, 2], "The native mouse device must recognize one double-click");
    const nativeIntervalMs = presses[1].timestamp - presses[0].timestamp;
    assert.ok(presses.every(press => Number.isSafeInteger(press.timestamp) && press.timestamp >= 0) &&
        nativeIntervalMs >= 0 && nativeIntervalMs <= 500,
    "Native press interval must remain within the unchanged 500 ms platform limit: " + nativeIntervalMs);
    for (const key of ["x", "y", "width", "height"]) {
        assert.equal(after.chrome.CanvasHost[key], before.chrome.CanvasHost[key], "The native canvas bounds stay fixed during insertion");
    }
    assert.equal(after.zoom, before.zoom);
    assert.deepEqual(after.pan, before.pan);
    for (const press of presses) {
        assert.equal(press.buttons, 1);
        assert.equal(press.zoom, before.zoom);
        assert.deepEqual(press.pan, before.pan);
        assert.deepEqual(press.screen, presses[0].screen, "Both native presses must hit the same canvas point");
        assert.deepEqual(press.world, presses[0].world, "Both native presses must hit the same world point");
        for (const coordinate of ["x", "y"]) {
            const screen = events[0][coordinate] - before.chrome.CanvasHost[coordinate];
            assert.ok(Math.abs(press.screen[coordinate] - screen) < 1e-6, "Native press must match the delivered canvas " + coordinate);
            assert.ok(Math.abs(press.world[coordinate] - (screen - press.pan[coordinate]) / press.zoom) < 1e-6,
                "Native press must match the delivered world " + coordinate);
        }
    }
    return { event: events[2], press: presses[1], domIntervalMs, nativeIntervalMs };
}

// Independent expected geometry for rectangular browser fixtures. Inputs are
// passive PointerEvent samples or the actual routed native press coordinates;
// none of these functions reads an edited gradient value.
const wrapDegrees = angle => {
    const remainder = angle % 360;
    return remainder < 0 ? remainder + 360 : remainder;
};

function matrix(node) {
    const m = node.affine_transform || { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
    const angle = node.rotation * Math.PI / 180, cosine = Math.cos(angle), sine = Math.sin(angle);
    const sx = node.flip_x ? -1 : 1, sy = node.flip_y ? -1 : 1;
    return {
        a: cosine * sx * m.a - sine * sy * m.b, b: sine * sx * m.a + cosine * sy * m.b,
        c: cosine * sx * m.c - sine * sy * m.d, d: sine * sx * m.c + cosine * sy * m.d,
    };
}

function localVector(node, point) {
    const m = matrix(node), determinant = m.a * m.d - m.b * m.c;
    assert.ok(Number.isFinite(determinant) && determinant !== 0);
    assert.ok(Number.isFinite(point.x) && Number.isFinite(point.y));
    return { x: (m.d * point.x - m.c * point.y) / determinant,
        y: (m.a * point.y - m.b * point.x) / determinant };
}

function worldPoint(node, point) {
    const m = node.affine_transform || { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
    const angle = node.rotation * Math.PI / 180, cosine = Math.cos(angle), sine = Math.sin(angle);
    const x = (m.a * point.x + m.c * point.y + (m.e ?? 0) - node.width / 2) * (node.flip_x ? -1 : 1);
    const y = (m.b * point.x + m.d * point.y + (m.f ?? 0) - node.height / 2) * (node.flip_y ? -1 : 1);
    return { x: node.x + node.width / 2 + cosine * x - sine * y,
        y: node.y + node.height / 2 + sine * x + cosine * y };
}

export function angularPaintFromPointerInput({ node, paint, part, zoom, down, moves }) {
    assert.equal(paint.type, "angular_gradient");
    assert.ok(part.kind === "end" || part.kind === "stop");
    assert.ok(Number.isFinite(zoom) && zoom > 0);
    const offset = part.kind === "stop" ? paint.stops[part.stop].offset : 0;
    const radians = (wrapDegrees(paint.angle) + offset * 360) * Math.PI / 180;
    const radius = Math.min(node.width, node.height) / 2;
    assert.ok(Number.isFinite(radius) && radius > 0);
    const base = { x: Math.sin(radians) * radius, y: -Math.cos(radians) * radius };
    let previous = base, change = 0;
    for (const move of moves) {
        const x = move.offsetX - down.offsetX, y = move.offsetY - down.offsetY;
        assert.ok(Number.isFinite(x) && Number.isFinite(y));
        if (Math.hypot(x, y) <= 1e-6) {
            previous = base; change = 0;
            continue;
        }
        const delta = localVector(node, { x: x / zoom, y: y / zoom });
        const next = { x: base.x + delta.x, y: base.y + delta.y };
        if (Math.hypot(next.x, next.y) <= 1e-12) continue;
        // Direction differences are wrapped independently of the native
        // normalized dot/cross implementation, then accumulated across seams.
        const step = Math.atan2(next.y, next.x) - Math.atan2(previous.y, previous.x);
        change += Math.atan2(Math.sin(step), Math.cos(step));
        previous = next;
    }
    const result = structuredClone(paint);
    if (part.kind === "end") result.angle = Math.max(-360000, Math.min(360000, paint.angle + change * 180 / Math.PI));
    else result.stops[part.stop].offset = Math.max(0, Math.min(1, offset + change / (2 * Math.PI)));
    return result;
}

export function gradientColorAtOffset(stops, offset) {
    const ordered = [...stops].sort((a, b) => a.offset - b.offset);
    const right = ordered.findIndex(stop => stop.offset > offset);
    if (right === 0) return structuredClone(ordered[0].color);
    if (right < 0) return structuredClone(ordered.at(-1).color);
    const a = ordered[right - 1], b = ordered[right], t = (offset - a.offset) / (b.offset - a.offset);
    return Object.fromEntries(["r", "g", "b", "a"].map(channel =>
        [channel, Math.round(a.color[channel] + (b.color[channel] - a.color[channel]) * t)]));
}

export function angularInsertionFromPointerInput({ node, paint, nativeWorld }) {
    assert.equal(paint.type, "angular_gradient");
    const center = worldPoint(node, { x: node.width * paint.center.x, y: node.height * paint.center.y });
    const local = localVector(node, { x: nativeWorld.x - center.x, y: nativeWorld.y - center.y });
    assert.ok(Math.hypot(local.x, local.y) > 1e-12);
    const offset = wrapDegrees(Math.atan2(local.x, -local.y) * 180 / Math.PI - wrapDegrees(paint.angle)) / 360;
    const result = structuredClone(paint);
    result.stops.push({ offset, color: gradientColorAtOffset(paint.stops, offset) });
    return result;
}
