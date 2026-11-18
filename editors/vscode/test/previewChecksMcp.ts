import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
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
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code native preview checks linked clipping, contrast and names to authored controls and cleared a repaired finding');
}
