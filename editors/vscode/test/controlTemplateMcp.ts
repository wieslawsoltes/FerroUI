import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

interface Model {
    model: string;
    slots: { property: string; canCopyInline: boolean; includedDefinition?: { property?: string; key: string } }[];
    resources: { key: string }[];
    includedResources: { key: string }[];
}

export async function verifyControlTemplate({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const path = 'ControlTemplateWorkbench.xaml';
    const source = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, path));
    const original = source.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const read = async (name: string) => {
        content(await invoke('designer_wait', { session }));
        const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        const model = content<Model>(await invoke('designer_studio_query', { session, tool: 'designer_item_templates',
            arguments: { node, property: 'Template' }, selection: [node] }));
        assert.equal(model.slots[0]?.property, 'Template');
        return { revision, node, nodes, model };
    };
    const apply = (report: Awaited<ReturnType<typeof read>>, action: object, present = false) =>
        invoke(present ? 'designer_studio_present' : 'designer_studio_edit', { session, expected_revision: report.revision,
            tool: 'designer_item_template', selection: [report.node], arguments: {
                node: report.node, property: 'Template', expected_model: report.model.model, ...action,
            } });
    const undo = async () => {
        content(await invoke('designer_history', { session, expected_version: source.version, action: 'undo' }));
        assert.equal(source.getText(), original);
    };
    const ready = await read('SharedAction');
    assert.ok(!ready.model.resources.some(resource => resource.key === 'InputOnly'));
    assert.ok(ready.model.includedResources.some(resource => resource.key === 'SharedButton'));
    assert.equal(ready.model.slots[0].includedDefinition?.property, 'Template');
    assert.ok(ready.model.slots[0].canCopyInline);
    const inspection = content<{ controlTemplate: boolean }>(await invoke('designer_inspect', { session, node: ready.node }));
    assert.equal(inspection.controlTemplate, true);
    const opened = content<{ opened_session: number }>(await apply(ready, { action: 'preview' }, true));
    assert.notEqual(opened.opened_session, session);
    const definition = content<{ revision: number }>(await invoke('designer_wait', { session: opened.opened_session }));
    const { scope } = content<{ scope: { definition: { title: string } } }>(await invoke('designer_studio_query', {
        session: opened.opened_session, tool: 'designer_definition_scope', arguments: {},
    }));
    assert.equal(scope.definition.title, 'SharedButton');
    content(await invoke('designer_studio_present', { session: opened.opened_session, expected_revision: definition.revision,
        tool: 'designer_definition', arguments: { action: 'exit' } }));

    content(await apply(await read('SharedAction'), { action: 'copy_inline' }));
    assert.match(source.getText(), /<Button\.Template>\s*<(?:\w+:)?ControlTemplate[^>]*TargetType="Button"/);
    assert.match(source.getText(), /TemplateBinding Content/);
    assert.ok(!source.getText().includes('Template="{StaticResource SharedButton}"'));
    await undo();
    const unset = await read('PlainAction');
    content(await apply(unset, { action: 'create_inline' }));
    assert.match(source.getText(), /<ControlTemplate TargetType="Button"><Border \/><\/ControlTemplate>/);
    assert.ok((await apply(unset, { action: 'reset' })).isError);
    await undo();
    console.log('Control template authoring opened the dictionary scope, copied a shared template, and restored source with Undo');
}
