import { runTests } from '@vscode/test-electron';
import { strict as assert } from 'node:assert';
import { constants } from 'node:fs';
import { access, copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { Writable } from 'node:stream';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
if (args.length < 4 || args.length > 5) {
    throw new Error('Usage: node test/workbenchBenchmark.mjs <vscode-executable> <baseline-extension> <candidate-extension> <report.json> [rounds]');
}
const [executable, baseline, candidate, report] = args.slice(0, 4).map(path => resolve(path));
const rounds = args[4] === undefined ? 4 : Number(args[4]);
if (!Number.isSafeInteger(rounds) || rounds < 1) throw new Error('Choose a positive number of paired rounds.');
await access(executable, constants.X_OK);
const extensionTestsPath = fileURLToPath(new URL('./workbenchBenchmarkDriver.cjs', import.meta.url));
const fixture = new URL('./fixtures/SettingsView.xaml', import.meta.url);
const workspace = await mkdtemp(join(tmpdir(), 'ferroui-workbench-benchmark-'));
const entries = [[baseline, 'baseline'], [candidate, 'candidate']];
const reports = { baseline: [], candidate: [] };

async function run(name, extensionDevelopmentPath) {
    const sandbox = await mkdtemp(join(workspace, `${name}-`));
    const root = join(sandbox, 'workspace');
    const profile = join(sandbox, 'profile');
    const extensions = join(sandbox, 'extensions');
    const result = join(sandbox, 'result.json');
    await mkdir(root);
    await mkdir(join(profile, 'User'), { recursive: true });
    await copyFile(fixture, join(root, 'SettingsView.xaml'));
    await writeFile(join(profile, 'User', 'settings.json'), JSON.stringify({
        'workbench.startupEditor': 'none',
        'workbench.editor.restoreViewState': false,
        'telemetry.telemetryLevel': 'off',
        'extensions.autoUpdate': false,
    }));
    let recentOutput = '';
    const capture = new Writable({ write(chunk, _encoding, done) {
        recentOutput = (recentOutput + chunk.toString()).slice(-8192);
        done();
    } });
    try {
        await runTests({
            vscodeExecutablePath: executable, extensionDevelopmentPath, extensionTestsPath,
            extensionTestsEnv: { FERROUI_BENCHMARK_RESULT: result }, stdout: capture, stderr: capture,
            launchArgs: [root, '--user-data-dir', profile, '--extensions-dir', extensions, '--skip-welcome', '--skip-release-notes',
                '--disable-extension', 'GitHub.copilot-chat'],
        });
    } catch (error) {
        console.error(recentOutput);
        throw new Error(`The ${name} VS Code benchmark failed`, { cause: error });
    } finally { capture.destroy(); }
    return JSON.parse(await readFile(result, 'utf8'));
}

function summarize(runs) {
    const samples = runs.flatMap(run => run.samples);
    const sorted = [...samples].sort((a, b) => a - b);
    const percentile = fraction => sorted[Math.ceil(fraction * sorted.length) - 1];
    return { p50: percentile(0.5), p95: percentile(0.95), runs };
}

try {
    for (let round = 0; round < rounds; round++) {
        for (const [extension, role] of round % 2 === 0 ? entries : [...entries].reverse()) {
            reports[role].push(await run(role, extension));
        }
    }
    const [reference] = reports.baseline;
    for (const sample of [...reports.baseline, ...reports.candidate]) {
        assert.equal(sample.vscode, reference.vscode);
        assert.deepEqual(sample.viewport, reference.viewport, 'Measure the same preview dimensions in both packages');
    }
    await writeFile(report, JSON.stringify({ rounds, baseline: summarize(reports.baseline), candidate: summarize(reports.candidate) }, null, 2) + '\n');
} finally { await rm(workspace, { recursive: true, force: true, maxRetries: 3 }); }
