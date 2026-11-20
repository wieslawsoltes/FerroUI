import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyIncludedTemplateCopiesMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'IncludedTemplateCopies.xaml';
    const root = vscode.workspace.workspaceFolders![0].uri;
    const open = (path: string) => vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, ...path.split('/')));
    const [consumer, dictionary, ink] = await Promise.all([path, 'Templates/Copy Rows.xaml', 'Templates/Shared/Ink.xaml'].map(open));
    const original = consumer.getText();
    const originalDictionary = dictionary.getText();
    const originalInk = ink.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const frame = async () => {
        const accepted = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(accepted.status, 'Ready');
        return accepted.revision;
    };
    const read = async (canCopy = true) => {
        const revision = await frame();
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === 'Directory')?.id;
        assert.ok(node);
        const result = content<{ model: string; slots: { canCopyInline: boolean; copyReason: string | null }[] }>(await invoke('designer_studio_query', {
            session, tool: 'designer_item_templates', arguments: { node }, selection: [node],
        }));
        assert.equal(result.slots[0]?.canCopyInline, canCopy, result.slots[0]?.copyReason ?? undefined);
        return { revision, node, model: result.model };
    };
    const copy = (current: Awaited<ReturnType<typeof read>>, model = current.model) => invoke('designer_studio_edit', {
        session, expected_revision: current.revision, tool: 'designer_item_template', selection: [current.node],
        arguments: { action: 'copy_inline', node: current.node, property: 'ItemTemplate', expected_model: model },
    });
    const image = async (dark: boolean) => {
        content(await invoke('designer_view', { session, dark }));
        await frame();
        const result = await invoke('designer_preview_image', { session });
        assert.equal(result.isError, false);
        const image = result.content.find(item => item.type === 'image')?.data;
        assert.ok(image);
        return image;
    };

    const retired = await read();
    const inkEditor = await vscode.window.showTextDocument(ink);
    const offset = originalInk.indexOf('#FF207991');
    assert.ok(offset >= 0);
    assert.ok(await inkEditor.edit(edit => edit.replace(
        new vscode.Range(ink.positionAt(offset), ink.positionAt(offset + 9)), '#FF305A84')));
    const draftInk = ink.getText();
    const current = await read();
    assert.notEqual(current.model, retired.model);
    const obsolete = await copy(current, retired.model);
    assert.ok(obsolete.isError);
    assert.match(obsolete.content[0]?.text ?? '', /authoring model changed/);
    assert.equal(consumer.getText(), original);

    const light = await image(false);
    const dark = await image(true);
    assert.notEqual(light, dark);
    content(await copy(await read()));
    assert.match(consumer.getText(), /<ListBox\.ItemTemplate>/);
    assert.match(consumer.getText(), /CustomerRow_InkColor">#FF305A84/);
    assert.match(consumer.getText(), /StaticResource 'CustomerRow_CustomerInk2'/);
    assert.match(consumer.getText(), /Background="\{DynamicResource RowSurface\}"/);
    assert.match(consumer.getText(), /Name="ReviewQueue"[^>]+ItemTemplate="\{StaticResource CustomerRow\}"/);
    assert.equal(dictionary.getText(), originalDictionary);
    assert.equal(ink.getText(), draftInk);
    assert.equal(await image(false), light, 'the copied template retains the native Light frame');
    assert.equal(await image(true), dark, 'the copied template retains the native Dark frame');

    const copySource = consumer.getText();
    const inline = await read(false);
    content(await invoke('designer_studio_present', {
        session, expected_revision: inline.revision, tool: 'designer_item_template', selection: [inline.node],
        arguments: { action: 'preview', node: inline.node, property: 'ItemTemplate', expected_model: inline.model },
    }));
    const revision = await frame();
    const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    const name = nodes.find(node => node.name === 'CustomerName');
    assert.ok(name?.bounds, 'the inline copy renders in its template editing scope');
    content(await invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_component', selection: [name.id],
        arguments: { action: 'set_property', node: name.id, property: 'FontSize', value: '22' },
    }));
    assert.match(consumer.getText(), /Name="CustomerName"[^>]+FontSize="22"/);
    assert.equal(dictionary.getText(), originalDictionary);
    content(await invoke('designer_studio_present', {
        session, expected_revision: await frame(), tool: 'designer_definition', arguments: { action: 'exit' },
    }));
    assert.notEqual(await image(true), dark, 'editing the inline template updates its consumer');
    content(await invoke('designer_history', { session, expected_version: consumer.version, action: 'undo' }));
    assert.equal(consumer.getText(), copySource);
    content(await invoke('designer_history', { session, expected_version: consumer.version, action: 'undo' }));
    assert.equal(consumer.getText(), original);
    assert.equal(await image(true), dark);
    await vscode.window.showTextDocument(ink, { viewColumn: inkEditor.viewColumn });
    await vscode.commands.executeCommand('undo');
    assert.equal(ink.getText(), originalInk);
    const labels = new Set([path, 'Copy Rows.xaml', 'Ink.xaml']);
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => labels.has(tab.label.replace(/^Design · /, ''))));
    console.log('VS Code and MCP made a private included template copy, preserved both native themes and the original resources, rejected a stale draft, and undid the copy in one step.');
}
