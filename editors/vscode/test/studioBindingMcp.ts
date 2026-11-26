import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';

export async function verifyBindingMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'BindingWorkbench.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(
        await invoke('designer_outline', { session }));
    const output = nodes.find(node => node.name === 'Output');
    assert.ok(output);
    const inspector = content<{ properties: { name: string; editable: boolean; canBind?: boolean }[] }>(
        await invoke('designer_inspect', { session, node: output.id }));
    const text = inspector.properties.find(field => field.name === 'Text');
    assert.deepEqual([text?.editable, text?.canBind], [false, true]);
    const arguments_ = { node: output.id, property: 'Text' };
    const binding = content<{ extension: string; fields: { path: string }; elementNames: string[] }>(
        await invoke('designer_studio_query', { session, tool: 'designer_binding', arguments: arguments_ }));
    assert.equal(binding.extension, 'f:Binding');
    assert.equal(binding.fields.path, 'Text');
    assert.ok(binding.elementNames.includes('Input'));
    const edit = { session, expected_revision: revision, tool: 'designer_binding_edit',
        arguments: { ...arguments_, changes: [{ field: 'mode', value: 'OneTime' }, { field: 'delay', value: 30 }] } };
    const invalid = { ...edit, arguments: { ...arguments_, changes: [{ field: 'delay', value: -1 }] } };
    assert.ok((await invoke('designer_studio_edit', invalid)).isError);
    assert.equal(document.getText(), original);
    edit.expected_revision = content<{ revision: number }>(await invoke('designer_wait', { session })).revision;
    content(await invoke('designer_studio_edit', edit));
    assert.equal(document.getText(), original.replace("FallbackValue='No value'}", "FallbackValue='No value', Mode='OneTime', Delay='30'}"));
    assert.ok((await invoke('designer_studio_edit', edit)).isError);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    const opened = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(opened);
    console.log('VS Code Properties identified an authored binding and MCP edited typed fields with one undo');
}
