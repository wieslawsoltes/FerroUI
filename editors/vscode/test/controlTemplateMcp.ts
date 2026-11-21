import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

interface Model {
    model: string;
    slots: {
        property: string;
        value: { kind: string };
        canCopyInline: boolean;
        localDefinition?: number;
        includedDefinition?: { uri: string; property?: string; key?: string };
    }[];
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
        return { revision, node, model };
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
    content(await invoke('designer_wait', { session }));
    const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    const shared = nodes.find(node => node.name === 'SharedAction');
    assert.ok(shared);
    const inspection = content<{ controlTemplate: boolean }>(await invoke('designer_inspect', { session, node: shared.id }));
    assert.equal(inspection.controlTemplate, true);
    const ready = await read('SharedAction');
    assert.ok(!ready.model.resources.some(resource => resource.key === 'InputOnly'));
    assert.ok(ready.model.includedResources.some(resource => resource.key === 'SharedButton'));
    assert.equal(ready.model.slots[0].includedDefinition?.property, 'Template');
    assert.ok(ready.model.slots[0].canCopyInline);
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
    const nativeImage = async () => {
        const frame = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(frame.status, 'Ready');
        const image = (await invoke('designer_preview_image', { session })).content
            .find(item => item.type === 'image')?.data;
        assert.ok(image);
        return image;
    };
    const local = await read('LocalAction');
    assert.equal(local.model.slots[0].canCopyInline, true);
    const before = await nativeImage();
    content(await apply(local, { action: 'copy_inline' }));
    assert.match(source.getText(), /x:Key="LocalButton_LocalInk" Color="#FF20657B"/);
    assert.match(source.getText(), /StaticResource 'LocalButton_LocalInk'/);
    assert.match(source.getText(), /Name="SecondLocalAction"[^>]+Template="\{StaticResource LocalButton\}"/);
    assert.equal(await nativeImage(), before);
    await undo();

    const inherited = await read('InheritedAction');
    assert.equal(inherited.model.slots[0].value.kind, 'unset');
    assert.ok(inherited.model.slots[0].localDefinition);
    assert.equal(inherited.model.slots[0].canCopyInline, true);
    content(await apply(inherited, { action: 'preview' }, true));
    const scoped = content<{ revision: number }>(await invoke('designer_wait', { session }));
    content(await invoke('designer_studio_present', { session, expected_revision: scoped.revision,
        tool: 'designer_definition', arguments: { action: 'exit' } }));
    const inheritedBefore = await nativeImage();
    content(await apply(await read('InheritedAction'), { action: 'copy_inline' }));
    assert.match(source.getText(), /x:Key="ControlTemplate_LocalInk" Color="#FF20657B"/);
    assert.match(source.getText(), /StaticResource 'ControlTemplate_LocalInk'/);
    assert.equal(await nativeImage(), inheritedBefore);
    await undo();

    const themed = await read('ThemedAction');
    assert.equal(themed.model.slots[0].value.kind, 'unset');
    assert.ok(themed.model.slots[0].includedDefinition?.uri.endsWith('/Templates/ControlTemplates.xaml'));
    assert.equal(themed.model.slots[0].includedDefinition?.key, undefined);
    assert.equal(themed.model.slots[0].canCopyInline, true);
    const themeCanvas = content<{ opened_session: number }>(await apply(themed, { action: 'preview' }, true));
    assert.notEqual(themeCanvas.opened_session, session);
    const themeDefinition = content<{ revision: number }>(await invoke('designer_wait', { session: themeCanvas.opened_session }));
    content(await invoke('designer_studio_present', { session: themeCanvas.opened_session, expected_revision: themeDefinition.revision,
        tool: 'designer_definition', arguments: { action: 'exit' } }));
    const themedBefore = await nativeImage();
    content(await apply(await read('ThemedAction'), { action: 'copy_inline' }));
    assert.match(source.getText(), /Name="ThemedAction"[^>]*Theme="\{StaticResource SharedButtonTheme\}"[^>]*>\s*<Button\.Template>/);
    assert.equal(await nativeImage(), themedBefore);
    await undo();

    const unset = await read('PlainAction');
    content(await apply(unset, { action: 'create_inline' }));
    assert.match(source.getText(), /<ControlTemplate TargetType="Button"><Border \/><\/ControlTemplate>/);
    assert.ok((await apply(unset, { action: 'reset' })).isError);
    await undo();
    console.log('Control template authoring opened and copied assigned and inherited templates, preserved native frames, and restored source with Undo');
}
