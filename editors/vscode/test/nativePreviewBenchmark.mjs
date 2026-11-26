import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { arch, cpus, platform, release, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { promisify } from 'node:util';
import { withHelper } from './nativeBenchmarkHelper.mjs';

const warmups = 6;
const frames = 30;
const indexRuns = 12;
const indexFiles = 160;
const memoryRuns = 3;
const viewport = { width: 640, height: 420, dark: false };
const stageCounts = [0, 1, 2, 4];
const exec = promisify(execFile);
const fixturePath = new URL('./fixtures/SettingsView.xaml', import.meta.url);

async function benchmark(helpers, directory) {
    const fixture = await readFile(fixturePath, 'utf8');
    const sourcePath = join(directory, 'SettingsView.xaml');
    await writeFile(sourcePath, fixture);
    const source = { language: 'xaml', uri: pathToFileURL(sourcePath).href, source: fixture, caret: { line: 0, character: 0 } };
    const index = await seedIndex(fixture, directory);
    const results = groups(helpers);

    await withHelper(results[0].preview, baseline => withHelper(results[1].preview, candidate =>
        comparePreview([baseline, candidate], results, source)));
    for (const entry of results) entry.firstIndexingMs = await indexTime(entry.languageServer, index);
    for (let round = 0; round < indexRuns; round++) {
        for (const entry of roundOrder(results, round)) entry.indexingMs.push(await indexTime(entry.languageServer, index));
    }
    for (let round = 0; round < memoryRuns; round++) {
        for (const entry of roundOrder(results, round)) {
            if (entry.stagesAvailable) {
                const memory = await withHelper(entry.preview, helper => stageMemory(helper, source));
                entry.stageResidentMiB.push(memory);
            }
        }
    }
    return {
        host: { platform: platform(), arch: arch(), release: release(), cpu: cpus()[0]?.model },
        fixture: { sha256: createHash('sha256').update(fixture).digest('hex'), viewport, warmups, frames, indexRuns, indexFiles, memoryRuns },
        results: await Promise.all(results.map(report)),
    };
}

function groups(helpers) {
    return helpers.map((directory, index) => {
        const suffix = process.platform === 'win32' ? '.exe' : '';
        return { role: index === 0 ? 'baseline' : 'candidate',
            preview: join(directory, `ferroui-designer-preview${suffix}`), languageServer: join(directory, `ferroui-xaml-lsp${suffix}`),
            renderMs: [], placementMs: [], indexingMs: [], stageResidentMiB: [] };
    });
}

function roundOrder(values, round) {
    return round % 2 === 0 ? values : [...values].reverse();
}

async function comparePreview(helpers, results, source) {
    const sessions = [];
    for (const [index, helper] of helpers.entries()) {
        const { capabilities } = await helper.request('initialize');
        results[index].stagesAvailable = capabilities?.stages === true;
        sessions.push({ helper, result: results[index], previousImage: undefined });
    }
    for (let round = 0; round < warmups + frames; round++) {
        const frame = round + 1;
        const name = round % 2 === 0 ? 'Northwind' : 'Southwind';
        const draft = source.source.replace('Northwind', name);
        assert.match(draft, new RegExp(`Text="${name}"`));
        for (const session of roundOrder(sessions, round)) {
            const { helper, result } = session;
            const start = performance.now();
            const snapshot = await helper.request('render', { ...source, source: draft, ...viewport, frame });
            const renderMs = performance.now() - start;
            assertPng(snapshot.image, snapshot.renderError);
            assert.notEqual(snapshot.image.png, session.previousImage, 'The modified source produced a new image');
            session.previousImage = snapshot.image.png;
            const container = snapshot.nodes.find(node => node.label === 'StackPanel' && node.canInsert);
            assert.ok(container?.bounds, 'Find the measured drop target');
            const { x, y, width } = container.bounds;
            const placement = { origin: { type: 'asset', control: 'TextBlock' },
                target: { type: 'canvas', x: x + width / 2, y: y + 2 } };
            const dragStart = performance.now();
            const hint = await helper.request('placement', { frame, placement });
            const placementMs = performance.now() - dragStart;
            assert.ok(hint.bounds && hint.description, 'The native planner returned a marked landing position');
            if (round >= warmups) {
                result.renderMs.push(renderMs);
                result.placementMs.push(placementMs);
            }
        }
    }
    for (const { helper, result } of sessions) result.artboardResidentMiB = await residentMiB(helper.pid);
}

async function seedIndex(source, directory) {
    const root = join(directory, 'index');
    await mkdir(root);
    const name = index => `PreviewBench${index}`;
    assert.ok(source.includes('saveButton'));
    await Promise.all(Array.from({ length: indexFiles }, (_, index) =>
        writeFile(join(root, `${name(index)}.xaml`), source.replace('saveButton', name(index)))));
    const query = name(indexFiles - 1);
    return { rootUri: pathToFileURL(root).href, uri: pathToFileURL(join(root, `${query}.xaml`)).href, query };
}

async function indexTime(executable, { rootUri, uri, query }) {
    const start = performance.now();
    return withHelper(executable, async helper => {
        await helper.request('initialize', { rootUri, capabilities: {} });
        await helper.notify('initialized', {});
        const symbols = await helper.request('workspace/symbol', { query });
        const elapsed = performance.now() - start;
        assert.ok(symbols.some(symbol => symbol.location.uri === uri && symbol.name.includes(query)),
            'Resolve the last unopened file by its authored name');
        await helper.request('shutdown');
        await helper.notify('exit');
        return elapsed;
    });
}

async function stageMemory(helper, source) {
    await helper.request('initialize');
    let model = await helper.request('stagesSync', { source, manifest: { version: 1, stages: [], active: null } });
    const resident = {};
    for (const count of stageCounts) {
        while (model.stages.length < count) {
            model = await helper.request('stagesEdit', { epoch: model.epoch, action: 'add',
                name: `Settings ${model.stages.length + 1}`, ...viewport });
            const stage = model.stages.at(-1);
            assertPng(await helper.request('stagesImage', { id: stage.id, epoch: stage.epoch }));
        }
        assert.equal(model.stages.length, count);
        resident[count] = await residentMiB(helper.pid);
    }
    return resident;
}

function assertPng(image, error) {
    assert.ok(image?.png?.startsWith('iVBORw0KGgo') && image.width > 0 && image.height > 0, error ?? 'Expected a native PNG');
}

async function residentMiB(pid) {
    const windows = process.platform === 'win32';
    const [command, args] = windows
        ? ['powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `(Get-Process -Id ${pid}).WorkingSet64`]]
        : ['ps', ['-o', 'rss=', '-p', String(pid)]];
    const { stdout } = await exec(command, args, { windowsHide: true });
    const value = Number(stdout.trim());
    assert.ok(Number.isFinite(value) && value > 0, 'Read the live preview helper resident set');
    return value / (windows ? 1024 ** 2 : 1024);
}

async function report(entry) {
    const digest = async file => {
        const hash = createHash('sha256');
        for await (const data of createReadStream(file)) hash.update(data);
        return hash.digest('hex');
    };
    return { role: entry.role, previewSha256: await digest(entry.preview), languageServerSha256: await digest(entry.languageServer),
        renderMs: distribution(entry.renderMs), placementMs: distribution(entry.placementMs),
        firstIndexingMs: entry.firstIndexingMs, indexingMs: distribution(entry.indexingMs),
        artboardResidentMiB: entry.artboardResidentMiB, stagesAvailable: entry.stagesAvailable, stageResidentMiB: entry.stageResidentMiB };
}

function distribution(samples) {
    const sorted = [...samples].sort((a, b) => a - b);
    const quantile = fraction => sorted[Math.ceil(fraction * sorted.length) - 1];
    return { p50: quantile(0.5), p95: quantile(0.95), samples };
}

const paths = process.argv.slice(2);
if (paths.length !== 2) throw new Error('Usage: node test/nativePreviewBenchmark.mjs <baseline-bin> <candidate-bin>');
const scratch = await mkdtemp(join(tmpdir(), 'ferroui-native-benchmark-'));
try { console.log(JSON.stringify(await benchmark(paths.map(path => resolve(path)), await realpath(scratch)), null, 2)); }
finally { await rm(scratch, { recursive: true, force: true, maxRetries: 3 }); }
