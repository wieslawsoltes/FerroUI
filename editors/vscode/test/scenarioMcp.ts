import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { ScenarioReport } from '../src/scenarioRunner';
import type { McpFixture } from './mcp';

export async function verifyScenarioMcp({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const uri = vscode.Uri.joinPath(root, 'ScenarioView.xaml');
    const baseline = vscode.Uri.joinPath(root, 'Baselines', 'customer-search.png');
    const original = await vscode.workspace.fs.readFile(uri);
    const path = 'customer-search.ferroui-scenario.json';
    const run = async (accept_baselines = false) =>
        content<ScenarioReport>(await invoke('ferroui_scenario_run', { path, accept_baselines }));
    const accepted = await run(true);
    assert.equal(accepted.passed, true, JSON.stringify(accepted.steps));
    assert.equal(accepted.steps.length, 5);
    const baselineImage = await vscode.workspace.fs.readFile(baseline);
    assert.equal(Buffer.from(baselineImage).subarray(1, 4).toString(), 'PNG');
    const matched = await run();
    assert.equal(matched.passed, true, JSON.stringify(matched.steps));

    try {
        const changed = new TextDecoder().decode(original).replace('#E4EEE8', '#823E40');
        await vscode.workspace.fs.writeFile(uri, Buffer.from(changed));
        const mismatch = await run();
        assert.equal(mismatch.passed, false);
        const failed = mismatch.steps.at(-1);
        assert.equal(failed?.step, 5);
        assert.match(failed?.message ?? '', /pixels changed/);
        const diff = failed?.artifacts.find(file => file.endsWith('-diff.png'));
        assert.ok(diff);
        assert.equal((await readFile(diff)).subarray(1, 4).toString(), 'PNG');
        assert.deepEqual(await vscode.workspace.fs.readFile(baseline), baselineImage);
    } finally {
        await vscode.workspace.fs.writeFile(uri, original);
    }
    const prototype = 'customer-review.ferroui-scenario.json';
    const runPrototype = async (accept_baselines = false) => content<ScenarioReport>(
        await invoke('ferroui_scenario_run', { path: prototype, accept_baselines }));
    const recorded = await runPrototype(true);
    assert.equal(recorded.passed, true, JSON.stringify(recorded.steps));
    assert.equal(recorded.steps.length, 11);
    assert.match(recorded.steps[4].message, /Showing overlay "Notes"/);
    assert.match(recorded.steps[6].message, /Focused Button "Close notes"/);
    assert.match(recorded.steps[9].message, /No overlay is open/);
    const replayed = await runPrototype();
    assert.equal(replayed.passed, true, JSON.stringify(replayed.steps));
    console.log('VS Code ran native input and accessibility assertions, accepted a PNG baseline and detected a real visual diff');
    console.log('VS Code replayed a saved native prototype across two screens and an overlay, then matched its screenshot baseline');
}
