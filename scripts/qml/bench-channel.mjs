// Exercise the browser worker framing code without Wasm or a network server.
// node scripts/qml/bench-channel.mjs [path-to-browser_channel.js]
import {readFileSync} from 'node:fs';
import {Worker, isMainThread, workerData, parentPort} from 'node:worker_threads';
import {once} from 'node:events';
import {performance} from 'node:perf_hooks';

if (isMainThread) {
    const file = process.argv[2] || new URL('../../src/Markup/FerroUI.Qml.WebScene/browser_channel.js', import.meta.url);
    const source = readFileSync(file, 'utf8');
    const channel = Function('return (' + source + ')')();
    const input = new SharedArrayBuffer(8 + 65536);
    const output = new SharedArrayBuffer(8 + 65536);
    const worker = new Worker(new URL(import.meta.url), {workerData: {source, input, output}});
    try {
        await once(worker, 'message');
        const tx = channel(input), rx = channel(output);
        for (const [name, text, count] of [
            ['small', '{"scope":3,"method":"value","args":[]}', 1000],
            ['unicode 16 KiB', '⬛🧪⛵'.repeat(1639), 200],
            ['multi-chunk 1 MiB', 'x🧪北'.repeat(131072), 12],
        ]) {
            const samples = [];
            for (let sample = 0; sample < 9; sample++) {
                const start = performance.now();
                for (let i = 0; i < count; i++) {
                    tx.write(text, performance.now() + 10000);
                    if (rx.read(performance.now() + 10000) !== text) throw Error('round-trip mismatch');
                }
                if (sample) samples.push((performance.now() - start) * 1000 / count);
            }
            samples.sort((a,b) => a-b);
            console.log(name, ((samples[3] + samples[4]) / 2).toFixed(1), 'us/round trip');
        }
    } finally {
        await worker.terminate();
    }
} else {
    const channel = Function('return (' + workerData.source + ')')();
    const rx = channel(workerData.input), tx = channel(workerData.output);
    parentPort.postMessage(true);
    for (;;) tx.write(rx.read(Infinity), Infinity);
}
