import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { StageChecks, StageModel } from '../src/stagesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyPreviewChecks({ invoke, content }: McpFixture): Promise<void> {
    const path = 'PreviewChecks.xaml';
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const args = { session, tool: 'designer_preview_checks', arguments: {} };
    const read = async () => content<{ findings: { name: string; code: string; message: string }[] }>(
        await invoke('designer_studio_query', args));
    const { findings } = await read();
    const codes = findings.map(finding => [finding.name, finding.code]);
    for (const expected of [['UnlabeledInput', 'missingAccessibleName'], ['FadedNote', 'lowTextContrast'], ['CutOffAction', 'clippedLayout']]) {
        assert.ok(codes.some(([name, code]) => name === expected[0] && code === expected[1]), JSON.stringify(findings));
    }
    assert.ok(!findings.some(finding => ['LabeledInput', 'ReadableNote', 'ScrollableAction'].includes(finding.name)), JSON.stringify(findings));
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    content(await invoke('designer_source', { session, expected_version: document.version,
        source: original.replace('Name="UnlabeledInput"', 'Name="UnlabeledInput" AutomationProperties.Name="Primary contact"') }));
    assert.ok(!(await read()).findings.some(finding => finding.name === 'UnlabeledInput'));
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    const board = content<StageModel>(await invoke('designer_stages_open', { session }));
    const stage = board.stages[0];
    assert.ok(stage);
    const target = { session, stage: stage.id, expected_epoch: stage.epoch };
    const stageReport = content<StageChecks>(await invoke('designer_stage_checks', target));
    const clipped = stageReport.findings.find(finding => finding.code === 'clippedLayout' && finding.name === 'CutOffAction');
    assert.ok(clipped?.range);
    assert.equal(document.getText(asRange(clipped.range)), 'Button');
    content(await invoke('designer_stage_change', { session, expected_epoch: board.epoch, arguments: {
        action: 'update', id: stage.id, configuration: { ...stage.configuration,
            environment: { ...stage.configuration.environment, width: stage.configuration.environment.width + 24 } },
    } }));
    assert.ok((await invoke('designer_stage_checks', target)).isError);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code native checks linked clipping, contrast and names to authored controls, cleared a repaired finding and rejected a retired stage');
}
