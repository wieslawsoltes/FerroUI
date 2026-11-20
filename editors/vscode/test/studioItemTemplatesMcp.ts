import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode, PropertyField } from '../src/protocol';
import type { McpFixture } from './mcp';

interface Report {
    model: string;
    slots: { property: string; value: { kind: string; key?: string }; canCopyInline: boolean }[];
    resources: { key: string; node: number }[];
}

export async function verifyItemTemplatesMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ItemTemplatesWorkbench.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));

    const frame = async () => {
        const { status } = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(status, 'Ready');
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const read = async (name: string) => {
        const { revision, nodes } = await frame();
        const node = nodes.find(node => node.name === name);
        assert.ok(node);
        const model = content<Report>(await invoke('designer_studio_query', { session,
            tool: 'designer_item_templates', arguments: { node: node.id }, selection: [node.id] }));
        assert.equal(model.slots[0]?.property, 'ItemTemplate');
        return { revision, node: node.id, model };
    };
    const run = (target: Awaited<ReturnType<typeof read>>, change: object, present = false, model = target.model.model) =>
        invoke(present ? 'designer_studio_present' : 'designer_studio_edit', {
            session, expected_revision: target.revision, tool: 'designer_item_template', selection: [target.node],
            arguments: { node: target.node, property: 'ItemTemplate', expected_model: model, ...change },
        });
    const undo = async (expected: string) => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
    };
    const definition = async () => content<{ scope: { definition: { title: string } | null } }>(
        await invoke('designer_studio_query', { session, tool: 'designer_definition_scope', arguments: {} }));

    const directory = await read('Directory');
    assert.equal(directory.model.slots[0].value.key, 'CustomerRow');
    assert.ok(directory.model.slots[0].canCopyInline);
    const { itemTemplates } = content<{ itemTemplates: boolean }>(
        await invoke('designer_inspect', { session, node: directory.node }));
    assert.equal(itemTemplates, true);
    content(await run(directory, { action: 'preview' }, true));
    assert.equal((await definition()).scope.definition?.title, 'CustomerRow');
    assert.equal(document.getText(), original);

    const active = await frame();
    const name = active.nodes.find(node => node.name === 'CustomerName');
    assert.ok(name?.bounds);
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: name.id }));
    assert.equal(properties.find(property => property.name === 'Text')?.effective, 'Alpine & Co.');
    content(await invoke('designer_studio_edit', { session, expected_revision: active.revision,
        tool: 'designer_component', selection: [name.id],
        arguments: { action: 'set_property', node: name.id, property: 'FontSize', value: '19' } }));
    await frame();
    assert.equal((await definition()).scope.definition?.title, 'CustomerRow');
    assert.match(document.getText(), /Name="CustomerName" FontSize="19"/);
    await undo(original);
    const restored = await frame();
    assert.equal((await definition()).scope.definition?.title, 'CustomerRow');
    content(await invoke('designer_studio_present', { session, expected_revision: restored.revision,
        tool: 'designer_definition', arguments: { action: 'exit' } }));
    assert.equal((await definition()).scope.definition, null);

    const current = await read('Directory');
    const stale = await run(current, { action: 'copy_inline' }, false, directory.model.model);
    assert.ok(stale.isError);
    assert.match(stale.content[0]?.text ?? '', /accepted authoring model changed/i);
    assert.equal(document.getText(), original);
    content(await run(await read('Directory'), { action: 'copy_inline' }));
    assert.match(document.getText(), /<ListBox.ItemTemplate>/);
    assert.match(document.getText(), /<!-- Keep this comment with the directory\. -->/);
    assert.equal(document.getText().match(/x:Key="CustomerRow"/g)?.length, 1);
    await undo(original);

    const queue = await read('ReviewQueue');
    const compact = queue.model.resources.find(resource => resource.key === 'CompactCustomer');
    assert.ok(compact);
    content(await run(queue, { action: 'assign_resource', resource: compact.node }));
    assert.equal((await read('ReviewQueue')).model.slots[0].value.key, 'CompactCustomer');
    await undo(original);
    content(await run(await read('ReviewQueue'), { action: 'create_inline' }));
    const inline = await read('ReviewQueue');
    assert.equal(inline.model.slots[0].value.kind, 'inline');
    const created = document.getText();
    content(await run(inline, { action: 'reset' }));
    assert.equal((await read('ReviewQueue')).model.slots[0].value.kind, 'unset');
    await undo(created);
    await undo(original);

    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code item templates used typed fixture data, retained the scope across a source edit and undo, and applied checked inline and resource edits');
}
