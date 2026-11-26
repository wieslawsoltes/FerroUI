import { strict as assert } from 'node:assert';
import { execFile } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import type { ScenarioReport } from '../src/scenarioRunner';
import type { McpFixture } from './mcp';

const run = promisify(execFile);

export async function verifyBranchReview({ invoke, content }: Pick<McpFixture, 'invoke' | 'content'>,
    root: vscode.Uri, source: Uint8Array, accepted: Uint8Array): Promise<void> {
    const folder = path.join(root.fsPath, 'Branch review fixture');
    const suite = 'customer-search.ferroui-scenario.json';
    const scenario = path.join(folder, suite);
    const view = path.join(folder, 'ScenarioView.xaml');
    const baseline = path.join(folder, 'Baselines', 'customer-search.png');
    await mkdir(path.dirname(baseline), { recursive: true });
    await writeFile(baseline, accepted);
    await writeFile(view, source);
    const definition = await readFile(path.join(root.fsPath, suite));
    await writeFile(scenario, definition);
    const git = async (...args: string[]) => (await run('git', [
        '-c', 'user.name=FerroUI Test', '-c', 'user.email=tests@example.invalid',
        '-c', 'commit.gpgSign=false', '-c', 'core.autocrlf=false', '-c', `core.hooksPath=${path.join(folder, 'no-hooks')}`, ...args,
    ], { cwd: folder, windowsHide: true })).stdout.trim();
    await git('init', '-q', '--initial-branch=main');
    await git('add', '.');
    await git('commit', '-qm', 'Accept the initial image');
    const fork = await git('rev-parse', 'HEAD');
    await git('branch', 'review');
    await git('commit', '-q', '--allow-empty', '-m', 'Advance main');
    await git('switch', '-q', 'review');

    const update = new TextDecoder().decode(source).replace('#E4EEE8', '#823E40');
    await writeFile(view, update);
    const inputs = { path: `Branch review fixture/${suite}`, baseline_ref: 'main' };
    const review = async () => content<ScenarioReport>(await invoke('ferroui_scenario_review', inputs));
    const changed = await review();
    assert.equal(changed.passed, false);
    assert.equal(changed.reference?.commit, fork);
    const capture = changed.visuals?.[0];
    assert.ok(capture?.approval && capture.expected && capture.difference);
    assert.equal(capture.comparison.status, 'changed');
    assert.equal(capture.workspace.status, 'changed');
    assert.deepEqual(await readFile(capture.expected), Buffer.from(accepted));
    assert.deepEqual(await readFile(baseline), Buffer.from(accepted));

    await writeFile(view, update + '\n<!-- A newer authored view -->\n');
    const stale = await invoke('ferroui_scenario_approve_baseline', { ...inputs, approval: capture.approval });
    assert.equal(stale.isError, true);
    assert.deepEqual(await readFile(baseline), Buffer.from(accepted));
    await writeFile(view, update);

    const broken = JSON.parse(definition.toString()) as { steps: unknown[] };
    broken.steps.push({ action: 'expectAccessible', name: 'Unavailable control', role: 'Button' });
    await writeFile(scenario, JSON.stringify(broken));
    const failure = await review();
    assert.equal(failure.visuals?.[0].approval, null);
    assert.match(failure.steps.at(-1)?.message ?? '', /Unavailable control/);
    await writeFile(scenario, definition);

    const approved = content<ScenarioReport>(await invoke('ferroui_scenario_approve_baseline', { ...inputs, approval: capture.approval }));
    assert.equal(approved.approved && path.basename(approved.approved), 'customer-search.png');
    assert.equal(approved.visuals?.[0].approval, null);
    const verified = await review();
    assert.ok(verified.passed);
    assert.equal(verified.visuals?.[0].workspace.status, 'match');
    assert.equal(verified.visuals?.[0].comparison.status, 'changed');
    console.log('VS Code reviewed a real Git merge base, rejected stale and failing runs, and approved one matching capture');
}
