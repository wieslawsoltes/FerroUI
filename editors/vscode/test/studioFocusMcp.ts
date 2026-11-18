import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CanvasFocusOrder, CanvasState } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyFocusOverlay({ invoke, content }: McpFixture): Promise<void> {
    const path = 'CustomerFocusOrder.xaml';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const document = await vscode.workspace.openTextDocument(uri);
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const { nodes, revision } = content<{ revision: number; nodes: { id: number; name: string }[] }>(
        await invoke('designer_outline', { session }));
    const node = (name: string) => nodes.find(node => node.name === name)!.id;
    const read = async () => content<{ model: string; overlay: CanvasFocusOrder }>(await invoke('designer_studio_query', {
        session, tool: 'designer_focus_order', arguments: { overlay: true },
    }));
    const setVisible = async (focus_order: boolean) => content<CanvasState>(await invoke('designer_canvas', {
        session, tab: 'preview', mode: 'select', focus_order,
    }));
    assert.equal((await setVisible(true)).focusOrder, true);
    const { model, overlay } = await read();
    const email = overlay.badges.findIndex(badge => badge.node === node('CustomerEmail'));
    const save = overlay.badges.findIndex(badge => badge.node === node('SaveContact'));
    assert.ok(email >= 0 && save >= 0 && overlay.links.some(([from, to]) => from === email && to === save));
    assert.deepEqual(overlay.badges[email].group, overlay.badges[save].group);
    assert.ok(overlay.badges[email].bounds.width > 0);
    const excluded = node('DiscardContact');
    assert.ok(!overlay.badges.some(badge => badge.node === excluded));
    const channels = ['EmailChannel', 'PhoneChannel'].map(name => overlay.badges.findIndex(badge => badge.node === node(name)));
    assert.ok(channels.every(index => index >= 0 && overlay.badges[index].group !== overlay.badges[email].group));
    assert.ok(!overlay.links.some(([a, b]) => channels.includes(a) && channels.includes(b)));
    const link = overlay.labels.find(item => item.node === node('EmailLabel'));
    assert.equal(link?.target, node('CustomerEmail'));
    assert.ok(link && link.to.y > link.from.y);
    assert.equal(document.getText(), original);
    const edit = { session, expected_revision: revision, tool: 'designer_focus_edit',
        arguments: { action: 'tab_stop', node: node('SaveContact'), value: false, expected_model: model } };
    content(await invoke('designer_studio_edit', edit));
    assert.match(document.getText(), /Name="SaveContact"[^>]*IsTabStop="False"/i);
    assert.ok((await invoke('designer_studio_edit', edit)).isError);
    assert.ok(!(await read()).overlay.badges.some(badge => badge.node === node('SaveContact')));
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    assert.equal((await setVisible(false)).focusOrder, false);
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`));
    console.log('VS Code mapped native focus ranks, Once group behavior and label links to the accepted frame, and undid a guarded focus edit');
}
