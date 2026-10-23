import assert from "node:assert/strict";

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
