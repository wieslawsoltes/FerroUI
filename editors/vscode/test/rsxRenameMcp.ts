import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { reviewStateSource } from './rsxStatesMcp';
import { until } from './wait';

export async function verifyRsxControlRename({ invoke, content }: McpFixture): Promise<void> {
    const path = 'review_names.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const declaration = [
        '            styles: [ferroui_view::style("Border#Card")',
        '                .animation(ferroui_view::animation(ferroui_view::ms(1000))',
        '                    .key_frame(0.0, |key| key.set(ferroui_base::Visual::opacity_property(), 0.8))',
        '                    .key_frame(1.0, |key| key.set(ferroui_base::Visual::opacity_property(), 1.0)))],',
        '',
    ].join('\r\n');
    const source = reviewStateSource.replace('            visual_states:', declaration + '            visual_states:');
    const renamed = source.replace('#Card', '#ClientCard').replaceAll('"Card"', '"ClientCard"');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    assert.equal(document.eol, vscode.EndOfLine.CRLF);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const outline = async (name: string) => {
        content(await invoke('designer_wait', { session }));
        const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(
            await invoke('designer_outline', { session }));
        const control = nodes.find(node => node.name === name);
        assert.ok(control, `Expected ${name} in the accepted native frame`);
        return { expected_revision: revision, node: control.id };
    };
    const history = async (action: 'undo' | 'redo', expected: string) => {
        const version = document.version;
        content(await invoke('designer_history', { session, expected_version: version, action }));
        await until(() => document.version !== version);
        assert.equal(document.getText(), expected);
    };
    const original = await outline('Card');
    content(await invoke('designer_rename_rsx_control', { session, ...original, value: 'ClientCard' }));
    assert.equal(document.getText(), renamed);
    await outline('ClientCard');
    assert.ok((await invoke('designer_rename_rsx_control', { session, ...original, value: 'Stale' })).isError);
    await history('undo', source);
    await history('redo', renamed);
    await history('undo', source);
    const current = await outline('Card');
    content(await invoke('designer_studio_edit', { session, expected_revision: current.expected_revision,
        tool: 'designer_property', arguments: { action: 'set', property: 'Name', nodes: [current.node], value: 'ClientCard' } }));
    assert.equal(document.getText(), renamed);
    const target = await outline('ClientCard');
    assert.ok((await invoke('designer_studio_edit', { session, expected_revision: target.expected_revision,
        tool: 'designer_property', arguments: { action: 'reset', property: 'Name', nodes: [target.node] } })).isError);
    assert.equal(document.getText(), renamed);
    await history('undo', source);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === 'Design · review_names.rs' || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    await vscode.workspace.fs.delete(uri, { useTrash: false });
    console.log('Renamed an RSX control, style selector and state targets in one VS Code edit and exercised Undo, Redo and stale-frame protection');
}
