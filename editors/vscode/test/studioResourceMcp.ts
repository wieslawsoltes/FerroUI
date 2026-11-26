import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { PropertyField } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyResourcePromotion({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ResourceWorkbench.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(
        await invoke('designer_outline', { session }));
    const node = nodes.find(item => item.name === 'CustomerCard')!.id;
    const property = 'Background';
    const inspect = async (target: number) => content<{ properties: PropertyField[] }>(
        await invoke('designer_inspect', { session, node: target })).properties.find(field => field.name === property);
    assert.ok((await inspect(node))?.canPromote);
    const promotion = { session, tool: 'designer_component_promotion', arguments: { node, property } };
    const { owners } = content<{ owners: { id: number; root: boolean }[] }>(await invoke('designer_studio_query', promotion));
    assert.equal(owners[0]?.id, node);
    const owner = owners.find(scope => scope.root)?.id;
    assert.ok(owner);
    const edit = { session, expected_revision: revision, tool: 'designer_component', arguments: {
        action: 'extract_property', node, property, owner, key: 'CustomerAccent',
    } };
    content(await invoke('designer_studio_edit', edit));
    assert.match(document.getText(), /SolidColorBrush[^>]*\bColor="#FF2D6A51"/);
    assert.match(document.getText(), /x:Key="CustomerAccent"/);
    assert.match(document.getText(), /Background="\{StaticResource 'CustomerAccent'\}"/);
    assert.match(document.getText(), /<!-- Preserve the card's authored content\. -->/);
    assert.ok((await invoke('designer_studio_edit', edit)).isError);
    const next = content<{ nodes: { name: string | null; id: number }[] }>(await invoke('designer_outline', { session }));
    const current = next.nodes.find(item => item.name === 'CustomerCard');
    assert.ok(current);
    const background = await inspect(current.id);
    assert.equal(background?.effective, '#FF2D6A51');
    assert.equal(background?.canPromote, false);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code Properties and Studio MCP promoted a local brush to a scoped resource and restored the source in one undo step');
}
