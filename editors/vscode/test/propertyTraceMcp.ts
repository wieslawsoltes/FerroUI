import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { DesignTraceSource, PropertyTrace } from '../src/propertyTrace';
import type { OutlineNode, PropertyField } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyPropertyTraceMcp({ invoke, content }: McpFixture): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const open = async (path: string, anchor?: string) => {
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, path));
        const original = document.getText();
        const position = anchor ? document.positionAt(original.indexOf(anchor)) : undefined;
        const { session } = content<{ session: number }>(await invoke('designer_open', {
            path, ...(position && { line: position.line, character: position.character }),
        }));
        const { status } = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(status, 'Ready');
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const read = async (name: string, property: string) => {
            const node = nodes.find(node => node.name === name);
            assert.ok(node?.bounds);
            return content<PropertyTrace<DesignTraceSource>>(await invoke('designer_studio_query', {
                session, tool: 'designer_property_trace', arguments: { node: node.id, property },
            }));
        };
        const finish = async () => {
            assert.equal(document.getText(), original);
            const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
                .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
            await vscode.window.tabGroups.close(tabs);
        };
        return { document, session, nodes, read, finish };
    };

    const xaml = await open('PropertyTraceWorkbench.xaml');
    const style = await xaml.read('StyleCard', 'Opacity');
    assert.equal(style.effective, '0.72');
    assert.equal(style.authored, null);
    assert.deepEqual(style.contributors.filter(row => row.state === 'effective').map(row => row.label), ['Style Border.priority']);
    const winner = style.contributors[0];
    assert.equal(winner.priority, 'StyleTrigger');
    assert.ok(winner.source && !winner.source.included);
    assert.equal(xaml.document.getText(asRange(winner.source.range)), '0.72');
    assert.match(xaml.document.lineAt(winner.source.range.start.line).text, /Border\.priority/);
    assert.ok(style.contributors.some(row => row.label === 'Style Border.account' && row.value === '0.72' && row.state === 'overridden'));
    assert.ok(style.contributors.some(row => row.label === 'Style Border.attention' && row.state === 'inactive'));

    const local = await xaml.read('LocalCard', 'Opacity');
    assert.equal(local.contributors[0].label, 'Local value');
    assert.equal(local.effective, '0.9');
    assert.equal(local.authored?.text, '0.9');
    const background = await xaml.read('LocalCard', 'Background');
    assert.equal(background.effective, '#FFEAF4FA');
    assert.equal(background.authored?.text, '{DynamicResource AccountSurface}');
    assert.ok(background.contributors[0].binding);
    assert.ok(background.resource && !background.resource.included);
    assert.match(xaml.document.lineAt(background.resource.range.start.line).text, /x:Key="AccountSurface"/);
    const card = xaml.nodes.find(node => node.name === 'LocalCard')!;
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session: xaml.session, node: card.id }));
    assert.equal(properties.find(row => row.name === 'Opacity')?.canTrace, true);
    assert.ok((await invoke('designer_studio_query', {
        session: xaml.session, tool: 'designer_property_trace',
        arguments: { node: card.id, runtime_node: card.id, property: 'Opacity' },
    })).isError);
    await xaml.finish();

    const rust = await open('views.rs', 'saveButton');
    const contentValue = await rust.read('saveButton', 'Content');
    assert.equal(contentValue.effective, 'Save changes');
    assert.ok(contentValue.authored && !contentValue.authored.included);
    assert.equal(rust.document.getText(asRange(contentValue.authored.range)), '"Save changes"');
    assert.equal(contentValue.contributors[0].source?.text, '"Save changes"');
    await rust.finish();
    console.log('VS Code traced native winners, inactive selectors, scoped resources, and mapped RSX source without authoring edits');
}
