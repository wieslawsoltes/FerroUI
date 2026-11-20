import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { IncludedTemplateSource } from '../src/includedTemplate';
import type { OutlineNode, PropertyField } from '../src/protocol';
import type { McpFixture } from './mcp';

interface Report {
    model: string;
    slots: { property: string; includedDefinition: IncludedTemplateSource | null }[];
    includedResources: Pick<IncludedTemplateSource, 'uri' | 'key'>[];
}

export async function verifyIncludedItemTemplatesMcp({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const consumerPath = 'IncludedItemTemplates.xaml';
    const dictionaryPath = 'Templates/Customer Rows.xaml';
    const consumer = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, consumerPath));
    const dictionary = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, dictionaryPath));
    const dictionaryUri = vscode.Uri.file(await realpath(dictionary.uri.fsPath));
    const originalConsumer = consumer.getText();
    const originalDictionary = dictionary.getText();
    const source = await vscode.window.showTextDocument(dictionary);
    const offset = originalDictionary.indexOf('FontSize="16"') + 'FontSize="'.length;
    assert.ok(offset > 10);
    assert.ok(await source.edit(edit => edit.replace(new vscode.Range(dictionary.positionAt(offset), dictionary.positionAt(offset + 2)), '18')));
    const draft = dictionary.getText();
    assert.ok(dictionary.isDirty);

    const { session } = content<{ session: number }>(await invoke('designer_open', { path: consumerPath }));
    const frame = async (session: number) => {
        assert.equal(content<{ status: string }>(await invoke('designer_wait', { session })).status, 'Ready');
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const read = async (name: string) => {
        const { revision, nodes } = await frame(session);
        const node = nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        const model = content<Report>(await invoke('designer_studio_query', { session, tool: 'designer_item_templates',
            arguments: { node }, selection: [node] }));
        assert.equal(model.slots[0]?.property, 'ItemTemplate');
        return { revision, node, model };
    };
    const apply = (report: Awaited<ReturnType<typeof read>>, action: object, present = false) =>
        invoke(present ? 'designer_studio_present' : 'designer_studio_edit', {
            session, expected_revision: report.revision, tool: 'designer_item_template', selection: [report.node],
            arguments: { node: report.node, property: 'ItemTemplate', expected_model: report.model.model, ...action },
        });
    const undo = async (session: number, document: vscode.TextDocument, expected: string) => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
    };
    const image = async () => {
        const result = await invoke('designer_preview_image', { session });
        assert.equal(result.isError, false);
        return result.content.find(item => item.type === 'image')!.data;
    };

    const queue = await read('ReviewQueue');
    assert.ok(queue.model.includedResources.some(resource => resource.key === 'CompactCustomer' && resource.uri === dictionaryUri.toString()));
    content(await apply(queue, { action: 'assign_included_resource', key: 'CompactCustomer' }));
    assert.match(consumer.getText(), /Name="ReviewQueue"[^>]*ItemTemplate="\{StaticResource 'CompactCustomer'\}"/);
    await undo(session, consumer, originalConsumer);
    assert.equal(dictionary.getText(), draft);

    const directory = await read('Directory');
    const target = directory.model.slots[0].includedDefinition;
    assert.equal(target?.key, 'CustomerRow');
    assert.equal(target?.uri, dictionaryUri.toString());
    const baseline = await image();
    const opened = content<{ opened_definition: IncludedTemplateSource }>(await apply(directory, { action: 'preview' }, true));
    assert.deepEqual(opened.opened_definition, target);
    const { designs } = content<{ designs: { uri: string; session: number }[] }>(await invoke('ferroui_designs'));
    const owner = designs.find(design => design.uri === dictionary.uri.toString())?.session;
    assert.ok(owner && owner !== session);
    const accepted = await frame(owner);
    const name = accepted.nodes.find(node => node.name === 'CustomerName');
    assert.ok(name?.bounds);
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session: owner, node: name.id }));
    assert.equal(properties.find(property => property.name === 'Text')?.effective, 'Alpine & Co.');
    assert.equal(properties.find(property => property.name === 'FontSize')?.authored, '18');
    content(await invoke('designer_studio_edit', { session: owner, expected_revision: accepted.revision,
        tool: 'designer_component', selection: [name.id],
        arguments: { action: 'set_property', node: name.id, property: 'FontSize', value: '29' } }));
    await frame(owner);
    const scope = content<{ scope: { definition: { title: string } } }>(
        await invoke('designer_studio_query', { session: owner, tool: 'designer_definition_scope', arguments: {} }));
    assert.equal(scope.scope.definition?.title, 'CustomerRow');
    assert.match(dictionary.getText(), /Name="CustomerName" FontSize="29"/);
    assert.equal(consumer.getText(), originalConsumer);
    await frame(session);
    assert.notEqual(await image(), baseline);
    const edited = await frame(owner);
    const stale = await invoke('designer_studio_present', { session: owner, expected_revision: edited.revision,
        tool: 'designer_definition', arguments: { action: 'enter_included_template', target: { source: target, sample: null } } });
    assert.ok(stale.isError);
    assert.match(stale.content[0]?.text ?? '', /included template source changed/i);
    await undo(owner, dictionary, draft);
    await frame(session);
    assert.equal(await image(), baseline);
    await undo(owner, dictionary, originalDictionary);

    const labels = ['IncludedItemTemplates.xaml', 'Customer Rows.xaml'];
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => labels.some(label => tab.label === label || tab.label === `Design · ${label}`));
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code and MCP opened an unsaved included template with its consumer fixture, edited its owning buffer, and refreshed the consumer through Undo');
}
