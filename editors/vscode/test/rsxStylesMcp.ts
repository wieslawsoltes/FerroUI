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
    const trace = async (property: string) => {
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const target = nodes.find(node => node.name === 'Review');
        assert.ok(target);
        return read<{ effective: string; contributors: { kind: string; state: string; source?: { text?: string } }[] }>(
            'designer_property_trace', { node: target.id, property });
    };
    const sizeTrace = () => trace('TextElement.FontSize');
    assert.equal((await sizeTrace()).effective, '24');
    const args = { action: 'setter', style: style.node, setter: size.node, value: '31.5' };
    assert.ok((await edit(await ready(), { ...args, value: 'Invalid' })).isError);
    assert.equal(document.getText(), original);
    const revision = await ready();
    content(await edit(revision, args));
    await ready();
    assert.equal((await sizeTrace()).effective, '31.5');
    assert.equal(document.getText(), original.replace('24.0f64', '31.5f64'));
    assert.ok((await edit(revision, args)).isError);
    const undo = () => invoke('designer_history', { session, expected_version: document.version, action: 'undo' }).then(content);
    await undo();
    assert.equal(document.getText(), original);
    await ready();
    assert.equal((await sizeTrace()).effective, '24');

    const choice = definition.resource_choices?.find(entry => entry.key === 'ReviewInk');
    assert.equal(choice?.color, '#FF224E7C');
    const foreground = definition.setters.find(setter => setter.property === 'Foreground')!;
    assert.equal(foreground.field.resources, true);
    const expected = await ready();
    assert.ok((await edit(expected, { action: 'set_resource', style: style.node, setter: size.node, key: 'ReviewInk' })).isError);
    content(await edit(await ready(), { action: 'set_resource', style: style.node, setter: foreground.node, key: 'ReviewInk' }));
    await ready();
    const linked = original.replace('foreground_property(), "#1C3554"', 'foreground_property(), "ReviewInk"')
        .replace('.set(ferroui_controls::TextBlock::foreground_property()', '.set_resource(ferroui_controls::TextBlock::foreground_property()');
    assert.equal(document.getText(), linked);
    const inkTrace = () => trace('TextElement.Foreground');
    const active = await inkTrace();
    assert.equal(active.effective, '#FF224E7C');
    assert.equal(active.contributors.find(entry => entry.kind === 'style' && entry.state === 'effective')?.source?.text, '"ReviewInk"');
    const bound = await read<StyleDefinition>('designer_rsx_styles', { node: style.node });
    const resource = bound.color_resources?.[0];
    assert.ok(resource && bound.setters.find(setter => setter.resource?.key === 'ReviewInk')?.resource?.declaration);
    content(await edit(await ready(), { action: 'color_resource', style: style.node, resource: resource.node, value: '#FF17674D' }));
    await ready();
    assert.equal((await inkTrace()).effective, '#FF17674D');
    assert.equal(document.getText(), linked.replace('34, /* palette */ 78, 124', '23, /* palette */ 103, 77'));
    await undo();
    assert.equal(document.getText(), linked);
    await ready();
    assert.equal((await inkTrace()).effective, '#FF224E7C');
    await undo();
    assert.equal(document.getText(), original);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code and MCP edited and traced Rust style setters and resources, preserved nested styles and native clips, and restored exact source with Undo');
}
