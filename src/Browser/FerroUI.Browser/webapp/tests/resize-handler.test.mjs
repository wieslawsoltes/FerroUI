import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transform } from "esbuild";

const source = await readFile(new URL("../modules/ferroui/rendering/resizeHandler.ts", import.meta.url), "utf8");
const { code } = await transform(source, { loader: "ts", format: "esm", target: "es2020" });
const moduleUrl = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
let sequence = 0;

class Events extends EventTarget {
    listeners = new Set();
    addEventListener(type, callback) { super.addEventListener(type, callback); this.listeners.add(callback); }
    removeEventListener(type, callback) { super.removeEventListener(type, callback); this.listeners.delete(callback); }
}

async function fixture(t, ratio = 1) {
    const globals = ["window", "ResizeObserver", "getComputedStyle"].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)]);
    const window = new Events(), queries = [], stops = [];
    let observer;
    window.devicePixelRatio = ratio;
    window.matchMedia = query => {
        const result = new Events(); result.media = query; queries.push(result); return result;
    };
    globalThis.window = window;
    globalThis.getComputedStyle = element => element.style;
    globalThis.ResizeObserver = class {
        observed = new Set();
        constructor(callback) { observer = this; this.deliver = entries => callback(entries, this); }
        observe(element) { this.observed.add(element); }
        unobserve(element) { this.observed.delete(element); }
    };
    const { ResizeHandler } = await import(`${moduleUrl}#fixture-${sequence++}`);
    t.after(() => {
        stops.forEach(stop => stop());
        for (const [name, descriptor] of globals) {
            if (descriptor) Object.defineProperty(globalThis, name, descriptor);
            else delete globalThis[name];
        }
    });
    return {
        window, queries, ResizeHandler,
        get observer() { return observer; },
        observe(element) {
            const values = [], stop = ResizeHandler.observeSize(element, (...size) => values.push(size));
            stops.push(stop); return { values, stop };
        },
        deliver(element, width, height, deviceBox, contentBox = [{ inlineSize: width, blockSize: height }]) {
            observer.deliver([{ target: element, contentRect: { width, height }, contentBoxSize: contentBox, devicePixelContentBoxSize: deviceBox }]);
        }
    };
}

function element(width = 320, height = 844, { hidden = false, border = 0, padding = 0, borderBox = false } = {}) {
    const inset = borderBox ? 2 * (border + padding) : 0;
    return {
        getClientRects: () => hidden ? [] : [{}],
        style: {
            width: `${width + inset}px`, height: `${height + inset}px`, boxSizing: borderBox ? "border-box" : "content-box",
            paddingLeft: `${padding}px`, paddingRight: `${padding}px`, paddingTop: `${padding}px`, paddingBottom: `${padding}px`,
            borderLeftWidth: `${border}px`, borderRightWidth: `${border}px`, borderTopWidth: `${border}px`, borderBottomWidth: `${border}px`
        }
    };
}
const pixels = (width, height) => [{ inlineSize: width, blockSize: height }];

test("a Retina host's device box cannot double a DPR=1 emulated viewport", async t => {
    const f = await fixture(t), target = element(), { values } = f.observe(target);
    f.deliver(target, 320, 844, pixels(640, 1688));
    assert.deepEqual(values, [[320, 844, 1]]);
});

test("a stale 1x delivery cannot halve layout after switching to DPR=2", async t => {
    const f = await fixture(t, 2), target = element(), { values } = f.observe(target);
    f.deliver(target, 320, 844, pixels(320, 844));
    assert.deepEqual(values, [[640, 1688, 2]]);
});

test("consistent device snapping is retained at integer and fractional ratios", async t => {
    const f = await fixture(t), target = element(), { values } = f.observe(target);
    for (const ratio of [1, 1.25, 1.5, 2, 3]) {
        f.window.devicePixelRatio = ratio;
        const width = Math.ceil(333.125 * ratio), height = Math.floor(211.75 * ratio);
        f.deliver(target, 333.125, 211.75, pixels(width, height));
        assert.deepEqual(values.at(-1), [width, height, ratio]);
    }
});

test("both device axes must agree before either precise dimension is trusted", async t => {
    const f = await fixture(t), target = element(), { values } = f.observe(target);
    f.deliver(target, 400.2, 300.2, pixels(401, 600));
    assert.deepEqual(values, [[400, 300, 1]]);
});

test("missing, empty, and invalid device boxes fall back to the CSS content rectangle", async t => {
    const f = await fixture(t, 1.5), target = element(), { values } = f.observe(target);
    for (const box of [undefined, [], pixels(NaN, 154), pixels(-1, 154)]) {
        f.deliver(target, 88.25, 102.5, box, { inlineSize: 88.25, blockSize: 102.5 });
        assert.deepEqual(values.at(-1), [132, 154, 1.5]);
    }
});

test("vertical writing-mode logical dimensions do not swap the canvas's physical axes", async t => {
    const f = await fixture(t, 2), target = element(), { values } = f.observe(target);
    f.deliver(target, 320, 844, pixels(1688, 640), [{ inlineSize: 844, blockSize: 320 }]);
    assert.deepEqual(values, [[640, 1688, 2]]);
});

test("initial and DPR-only measurements use content size with borders, padding, and hidden elements", async t => {
    const f = await fixture(t, 2), target = element(320, 844, { border: 2, padding: 10, borderBox: true });
    assert.deepEqual(f.ResizeHandler.getInitialSize(target), [640, 1688, 2]);
    assert.deepEqual(f.ResizeHandler.getInitialSize(element(320, 844, { hidden: true })), [0, 0, 2]);
});

test("DPR-only monitor changes remeasure all surfaces, rearm queries, and release disposed listeners", async t => {
    const f = await fixture(t), target = element(), hidden = element(320, 844, { hidden: true });
    const first = f.observe(target), second = f.observe(hidden), initialQuery = f.queries.at(-1);
    f.window.devicePixelRatio = 2;
    initialQuery.dispatchEvent(new Event("change"));
    assert.deepEqual(first.values, [[640, 1688, 2]]);
    assert.deepEqual(second.values, [[0, 0, 2]]);
    assert.equal(initialQuery.listeners.size, 0);
    assert.equal(f.queries.at(-1).media, "(resolution: 2dppx)");

    // A window resize may precede the queued media-query/observer delivery.
    f.window.devicePixelRatio = 1.25;
    f.window.dispatchEvent(new Event("resize"));
    assert.deepEqual(first.values.at(-1), [400, 1055, 1.25]);
    assert.equal(f.queries.at(-1).media, "(resolution: 1.25dppx)");
    const count = first.values.length;
    f.window.dispatchEvent(new Event("resize"));
    assert.equal(first.values.length, count);

    // A stale observer entry following that event must retain the new scale.
    f.deliver(target, 320, 844, pixels(640, 1688));
    assert.deepEqual(first.values.at(-1), [400, 1055, 1.25]);
    first.stop();
    assert.equal(f.window.listeners.size, 1);
    second.stop();
    assert.equal(f.window.listeners.size, 0);
    assert.equal(f.queries.at(-1).listeners.size, 0);
    assert.equal(f.observer.observed.size, 0);
    const stoppedCount = first.values.length;
    f.deliver(target, 320, 844, pixels(320, 844));
    assert.equal(first.values.length, stoppedCount);
});

test("disposing a superseded observation does not unsubscribe its replacement", async t => {
    const f = await fixture(t), target = element(), first = f.observe(target), second = f.observe(target);
    first.stop();
    f.deliver(target, 320, 844, pixels(320, 844));
    assert.deepEqual(first.values, []);
    assert.deepEqual(second.values, [[320, 844, 1]]);
    assert.equal(f.observer.observed.size, 1);
    second.stop();
    assert.equal(f.observer.observed.size, 0);
    assert.equal(f.window.listeners.size, 0);
});
