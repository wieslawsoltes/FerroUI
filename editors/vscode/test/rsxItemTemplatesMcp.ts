import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { OutlineNode, Range } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface Slot {
    property: string;
    range: Range | null;
    rustBody: Range | null;
    canCreate: boolean;
    canReset: boolean;
}

export async function verifyRsxItemTemplates({ invoke, content }: McpFixture): Promise<void> {
    const path = 'template_workbench.rs';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const tool = async <T>(name: string, arguments_: object) => content<T>(await invoke(name, arguments_));
    const { session } = await tool<{ session: number }>('designer_open', { path });
    const outline = async () => {
        const ready = await tool<{ renderError: string | null }>('designer_wait', { session });
        assert.ok(!ready.renderError, ready.renderError ?? undefined);
        return tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
    };
    const studio = <T>(name: string, arguments_: object) => tool<T>('designer_studio_query', {
        session, tool: name, arguments: arguments_,
    });
    const read = async (name: string, column?: number) => {
        const frame = await outline();
        const selected = frame.nodes.find(node => node.name === name);
        assert.ok(selected);
        const node = column === undefined ? selected.id
            : (await studio<TableColumns>('designer_columns', { node: selected.id })).columns[column].node;
        const { model, slots } = await studio<{ model: string; slots: Slot[] }>('designer_item_templates', { node });
        assert.ok(slots.length);
        return { frame, node, model, slot: slots[0] };
    };
    const run = (target: Awaited<ReturnType<typeof read>>, action: string, extra: object = {}) =>
        invoke(action === 'preview' ? 'designer_studio_present' : 'designer_studio_edit', {
            session, expected_revision: target.frame.revision, tool: 'designer_item_template',
            arguments: { node: target.node, property: target.slot.property, expected_model: target.model, action, ...extra },
        });
    const undo = async (expected: string) => {
        await tool('designer_history', { session, expected_version: document.version, action: 'undo' });
        await until(() => document.getText() === expected);
    };
    const back = async () => {
        const { revision } = await outline();
        await tool('designer_studio_present', { session, expected_revision: revision, tool: 'designer_definition', arguments: { action: 'exit' } });
        assert.ok((await outline()).nodes.some(node => node.name === 'Directory'));
    };

    const { edits } = await tool<{ edits: { name: string }[] }>('designer_studio_tools', { session });
    assert.ok(edits.some(edit => edit.name === 'designer_item_template'));
    const directory = await read('Directory');
    assert.ok(directory.slot.rustBody && directory.slot.canReset);
    assert.match(document.getText(asRange(directory.slot.rustBody)), /^rsx! \{\s*Border/);
    assert.equal((await tool<{ itemTemplates: boolean }>('designer_inspect', { session, node: directory.node })).itemTemplates, true);
    const entered = content<{ rust_template: string; opened_template: { range: Range } }>(await run(directory, 'preview'));
    assert.equal(entered.rust_template, 'ItemTemplate');
    assert.deepEqual(entered.opened_template.range, directory.slot.rustBody);

    const body = await outline();
    const title = body.nodes.find(node => node.name === 'CustomerTitle');
    assert.ok(title?.bounds, 'The exact template invocation has a rendered native control');
    await tool('designer_edit_property', { session, expected_revision: body.revision, node: title.id,
        property: 'Text', value: 'Account profile 🧭', action: 'set' });
    assert.equal(document.getText(), original.replace('Customer profile 🧭', 'Account profile 🧭'));
    await undo(original);
    await back();
    assert.ok((await run(directory, 'preview')).isError, 'Retired authoring frame cannot open the old range');

    const computed = await read('Computed');
    assert.equal(document.getText(asRange(computed.slot.range!)), 'computed_template()');
    assert.ok(!computed.slot.rustBody && !computed.slot.canReset);
    assert.ok((await run(computed, 'reset')).isError);
    assert.equal(document.getText(), original);

    const blank = await read('Accounts', 0);
    assert.equal(blank.slot.property, 'CellTemplate');
    assert.ok(blank.slot.canCreate);
    await tool('designer_select', { session, expected_revision: blank.frame.revision,
        node: blank.frame.nodes.find(node => node.name === 'Accounts')!.id });
    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    await designer.revealDesign(panel, 'columns');
    await until(async () => (await designer.columns.getChildren()).some(row => row.column.label === 'Account'));
    const column = (await designer.columns.getChildren()).find(row => row.column.label === 'Account')!;
    const field = (await designer.columns.getChildren(column)).find(row => row.field?.name === 'CellTemplate');
    assert.ok(field && designer.columns.getTreeItem(field).command);

    content(await run(await read('Accounts', 0), 'create_inline', { rust_type: 'Customer' }));
    const created = document.getText();
    assert.match(created, /\.cell_template\(::ferroui_view::template::<Customer, _>/);
    const cell = await read('Accounts', 0);
    assert.ok(cell.slot.rustBody && cell.slot.canReset);
    assert.equal(content<{ rust_template: string }>(await run(cell, 'preview')).rust_template, 'CellTemplate');
    assert.ok((await outline()).nodes[0].bounds);
    await back();
    content(await run(await read('Accounts', 0), 'reset'));
    assert.equal(document.getText(), original);
    await undo(created);
    await undo(original);

    panel.panel.dispose();
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path);
    await vscode.window.tabGroups.close(tabs);
    console.log('RSX Items and Columns used exact native template scopes, retained Rust expressions, and checked grouped source edits and Undo');
}
