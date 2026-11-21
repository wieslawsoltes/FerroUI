import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { StyleCatalog, StyleDefinition } from '../src/stylesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyRsxSharedColorsMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'rust_shared_colors.rs';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { line, character } = document.positionAt(original.indexOf('rsx!', original.indexOf('fn review')));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path, line, character }));
    const ready = async () => {
        const frame = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(frame.status, 'Ready');
        return frame.revision;
    };
    const read = <T>(tool: string, arguments_: object = {}) => invoke('designer_studio_query', { session, tool, arguments: arguments_ }).then(content<T>);
    const model = async () => {
        await ready();
        const { definitions } = await read<StyleCatalog>('designer_rsx_styles');
        return read<StyleDefinition>('designer_rsx_styles', { node: definitions[0].node });
    };
    const color = async () => {
        await ready();
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === 'SharedStatus');
        assert.ok(node, 'The shared edit keeps the second RSX invocation active');
        return (await read<{ effective: string }>('designer_property_trace', { node: node.id, property: 'Foreground' })).effective;
    };
    const originalColor = 'Rgb::from_rgb(37u8, /* brand 🧭 */ 74, 112)';
    const initial = await model();
    const entry = initial.color_resources?.find(color => color.layer === 'default');
    assert.ok(entry);
    assert.equal(entry.shared_constant, 'DEFAULT_INK');
    assert.equal(document.getText(asRange(entry.range)), originalColor);
    assert.equal(initial.resource_comparison?.[0].dark?.scope, 'Dark');
    assert.equal(await color(), '#FF254A70');
    const request = { session, expected_revision: await ready(), tool: 'designer_rsx_style_edit',
        arguments: { action: 'color_resource', style: initial.node, resource: entry.node, value: '#EB177254' } };
    assert.equal((await invoke('designer_studio_edit', request)).isError, true, 'A shared edit must have an explicit mode');
    content(await invoke('designer_studio_edit', { ...request, arguments: { ...request.arguments, mode: 'declaration' } }));
    assert.equal(document.getText(), original.replace(originalColor, 'Rgb::from_argb(235, 23u8, /* brand 🧭 */ 114, 84)'));
    assert.equal(await color(), '#EB177254');
    assert.equal((await invoke('designer_studio_edit', { ...request, arguments: { ...request.arguments, mode: 'declaration' } })).isError, true);
    const undo = () => invoke('designer_history', { session, action: 'undo', expected_version: document.version }).then(content);
    await undo();
    assert.equal(document.getText(), original);
    assert.equal(await color(), '#FF254A70');

    const current = await model();
    const next = current.color_resources?.find(color => color.layer === 'default');
    assert.ok(next);
    content(await invoke('designer_studio_edit', { ...request, expected_revision: await ready(),
        arguments: { ...request.arguments, style: current.node, resource: next.node, mode: 'local', value: '#FF246A4F' } }));
    assert.equal(await color(), '#FF246A4F');
    assert.ok(document.getText().includes(originalColor));
    assert.ok(document.getText().includes('/* account palette */'));
    assert.equal((await model()).color_resources?.find(color => color.layer === 'default')?.shared_constant, null);
    await undo();
    assert.equal(document.getText(), original);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code and MCP followed Rust color aliases and undid both shared and local edits in the active invocation');
}
