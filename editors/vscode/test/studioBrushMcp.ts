import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { PropertyField } from '../src/protocol';
import type { McpFixture } from './mcp';

interface BrushDraft { source: string; brush: { kind: string; node: number }; stops: { node: number }[] }

export async function verifyBrushMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'CustomerGradient.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const { revision, nodes } = content<{ revision: number; nodes: { name?: string; id: number }[] }>(await invoke('designer_outline', { session }));
    const node = nodes.find(item => item.name === 'CustomerCard')?.id;
    assert.ok(node);
    const property = 'Background';
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node }));
    assert.ok(properties.find(field => field.name === property)?.canEditBrush);
    const query = (arguments_: object) => invoke('designer_studio_query', { session, tool: 'designer_brush_draft', arguments: arguments_ });
    let draft = content<BrushDraft>(await query({ action: 'open', node, property }));
    assert.equal(draft.brush.kind, 'LinearGradientBrush');
    assert.equal(draft.stops.length, 2);
    const before = (await invoke('designer_preview_image', { session })).content.find(item => item.type === 'image')?.data;
    assert.ok(before);
    const update = async (change: object) => { draft = content<BrushDraft>(await query({ action: 'change', source: draft.source, change })); };
    await update({ edit: 'insert_stop' });
    let stop = draft.stops[2].node;
    assert.ok((await query({ action: 'change', source: draft.source, change: { edit: 'set', node: stop, property: 'Offset', value: '1.2' } })).isError);
    await update({ edit: 'set', node: stop, property: 'Color', value: '#675DF5' });
    stop = draft.stops[2].node;
    await update({ edit: 'move_stop', node: stop, direction: -1 });
    assert.match(draft.source, /Color="#675DF5" Offset="0\.5"[^\n]*\n\s*<GradientStop Color="#2D6A51"/);
    assert.equal(document.getText(), original);
    const commit = { session, expected_revision: revision, tool: 'designer_component', arguments: {
        action: 'set_object_property', node, property, source: draft.source,
    } };
    content(await invoke('designer_studio_edit', commit));
    assert.match(document.getText(), /Keep the authored stops and their order/);
    assert.match(document.getText(), /Northwind Studio · Account review/);
    assert.ok((await invoke('designer_studio_edit', commit)).isError);
    const frame = await invoke('designer_preview_image', { session });
    const after = frame.content.find(item => item.type === 'image')?.data;
    assert.ok(after, JSON.stringify(frame.content[0]));
    assert.notEqual(after, before);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code edited typed gradient stops in a detached Studio draft, rendered the committed brush and undid the source patch');
}
