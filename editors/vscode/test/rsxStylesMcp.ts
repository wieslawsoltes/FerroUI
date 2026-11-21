import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { StyleCatalog, StyleDefinition } from '../src/stylesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyRustStylesMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'rust_style_workbench.rs';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const read = async <T>(tool: string, args: object = {}) =>
        content<T>(await invoke('designer_studio_query', { session, tool, arguments: args }));
    const ready = async () => {
        const frame = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(frame.status, 'Ready', 'The mixed style and animation must render in the native design');
        return frame.revision;
    };
    const edit = (revision: number, args: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_rsx_style_edit', arguments: args,
    });
    await ready();
    content(await invoke('designer_view', { session, view: 'styles' }));
    const catalog = await read<StyleCatalog>('designer_rsx_styles');
    assert.equal(catalog.source_kind, 'rust');
    assert.ok(catalog.definitions.some(entry => entry.name === '^.muted'));
    assert.ok(catalog.issues?.some(issue => document.getText(asRange(issue.range)) === 'preferred_note_size()'));
    const style = catalog.definitions.find(entry => entry.name === '#Review')!;
    const definition = await read<StyleDefinition>('designer_rsx_styles', { node: style.node });
    const size = definition.setters.find(setter => setter.property === 'FontSize')!;
    assert.equal(document.getText(asRange(size.range)), '24.0f64');
    const effective = async () => {
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const target = nodes.find(node => node.name === 'Review');
        assert.ok(target);
        return (await read<{ effective: string }>('designer_property_trace', { node: target.id, property: 'TextElement.FontSize' })).effective;
    };
    assert.equal(await effective(), '24');
    const args = { action: 'setter', style: style.node, setter: size.node, value: '31.5' };
    assert.ok((await edit(await ready(), { ...args, value: 'Invalid' })).isError);
    assert.equal(document.getText(), original);
    const revision = await ready();
    content(await edit(revision, args));
    await ready();
    assert.equal(await effective(), '31.5');
    assert.equal(document.getText(), original.replace('24.0f64', '31.5f64'));
    assert.ok((await edit(revision, args)).isError);
    await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.One });
    await vscode.commands.executeCommand('undo');
    assert.equal(document.getText(), original);
    await ready();
    assert.equal(await effective(), '24');
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code and MCP edited a Rust style beside its nested rule and native clip, then undid the exact registered setter change');
}
