import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { OutlineNode, Range } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxTreeDataGrid({ invoke, content }: McpFixture): Promise<void> {
    const path = 'tree_grid_workbench.rs';
    const file = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const document = await vscode.workspace.openTextDocument(file);
    const original = document.getText();
    const tool = async <T>(name: string, arguments_: object) => content<T>(await invoke(name, arguments_));
    const { session } = await tool<{ session: number }>('designer_open', { path });
    const outline = async () => {
        const { renderError } = await tool<{ renderError: string | null }>('designer_wait', { session });
        assert.ok(!renderError, renderError ?? undefined);
        return tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
    };
    const query = <T>(name: string, arguments_: object) => tool<T>('designer_studio_query', {
        session, tool: name, arguments: arguments_,
    });
    const read = async (name = 'Accounts') => {
        const frame = await outline();
        const node = frame.nodes.find(node => node.name === name);
        assert.ok(node);
        const model = await query<TableColumns>('designer_columns', { node: node.id });
        assert.equal(model.table, node.id);
        return { revision: frame.revision, node, model };
    };
    const edit = (grid: Awaited<ReturnType<typeof read>>, action: object) => invoke('designer_studio_edit', {
        session, expected_revision: grid.revision, tool: 'designer_columns_edit',
        arguments: { table: grid.node.id, expected_model: grid.model.model, ...action },
    });
    const undo = async () => {
        await tool('designer_history', { session, expected_version: document.version, action: 'undo' });
        await until(() => document.getText() === original);
    };
    const image = async () => {
        await outline();
        const reply = await invoke('designer_preview_image', { session });
        assert.equal(reply.isError, false);
        const data = reply.content.find(item => item.type === 'image')?.data;
        assert.ok(data);
        return data;
    };

    const application = await read('ApplicationModel');
    assert.match(application.model.reason ?? '', /defined by Rust/);
    assert.equal(document.getText(asRange(application.model.sourceRange!)), 'assigned_model()');
    const initial = await read();
    assert.equal(initial.model.reason, null);
    assert.deepEqual(initial.model.columns.map(column => column.kind),
        ['rowHeader', 'expander', 'text', 'checkBox', 'template']);
    const [rowHeader, hierarchy, , , status] = initial.model.columns;
    assert.equal(hierarchy.label, 'Account 🧭');
    assert.equal(hierarchy.children[0].fields.find(field => field.name === 'Binding')?.value, 'Name');
    const before = await image();
    content(await edit(initial, { action: 'set', column: hierarchy.children[0].node, field: 'Width', value: '188' }));
    const updated = await read();
    assert.equal(updated.model.columns[1].children[0].fields.find(field => field.name === 'Width')?.value, '188');
    assert.equal(document.getText(), original.replace('from_pixels(160.0)', 'from_pixels(188.0)'));
    assert.notEqual(await image(), before, 'The nested width changes the rendered native header');
    assert.ok((await edit(updated, { action: 'remove', column: rowHeader.node, expected_model: initial.model.model })).isError);
    await undo();

    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    const active = await read();
    await tool('designer_select', { session, expected_revision: active.revision, node: active.node.id });
    await designer.revealDesign(panel, 'columns');
    const view = designer.columns;
    await until(async () => (await view.getChildren()).some(row => row.column.label === 'Account 🧭'
        && row.editor.context.token === panel.currentToken));
    const rows = await view.getChildren();
    const hierarchyRow = rows.find(row => row.column.kind === 'expander')!;
    const cellRow = rows.find(row => row.column.kind === 'template')!;
    assert.ok((await view.getChildren(hierarchyRow)).some(row => row.column.kind === 'text' && !row.field));
    const editingField = (await view.getChildren(cellRow)).find(row => row.field?.name === 'CellEditingTemplate');
    assert.ok(editingField && view.getTreeItem(editingField).command);

    const cancel = new vscode.CancellationTokenSource();
    try {
        const transfer = new vscode.DataTransfer();
        view.handleDrag([hierarchyRow], transfer, cancel.token);
        await view.handleDrop(rows[0], transfer, cancel.token);
        const moved = await read();
        assert.equal(moved.model.columns[0].kind, 'expander');
        assert.ok(document.getText().indexOf('// Keep the account labels') < document.getText().indexOf('tree_data_grid_row_header_column'));
        await undo();
    } finally { cancel.dispose(); }

    const current = await read();
    const column = current.model.columns.find(column => column.kind === 'template')!;
    const model = await query<{ model: string; slots: { rustBody: Range | null; canCreate: boolean }[] }>(
        'designer_item_templates', { node: column.node });
    assert.ok(model.slots[0].rustBody && model.slots[1].canCreate);
    assert.equal(column.label, status.label);
    await tool('designer_studio_edit', { session, expected_revision: current.revision, tool: 'designer_item_template',
        arguments: { node: column.node, property: 'CellEditingTemplate', expected_model: model.model,
            action: 'create_inline', rust_type: 'Account' } });
    assert.match(document.getText(), /\.cell_editing_template\(::ferroui_view::template::<Account, _>/);
    const withEditor = await read();
    const editor = withEditor.model.columns.find(column => column.kind === 'template')!;
    const editorPlan = await query<{ model: string }>('designer_item_templates', { node: editor.node });
    const scope = await tool<{ rust_template: string; revision: number }>('designer_studio_present', {
        session, expected_revision: withEditor.revision, tool: 'designer_item_template',
        arguments: { node: editor.node, property: 'CellEditingTemplate', expected_model: editorPlan.model, action: 'preview' },
    });
    assert.equal(scope.rust_template, 'CellEditingTemplate');
    assert.ok((await outline()).nodes.some(node => node.label === 'TextBox' && node.bounds));
    await tool('designer_studio_present', { session, expected_revision: scope.revision,
        tool: 'designer_definition', arguments: { action: 'exit' } });
    await undo();

    const empty = await read('Queue');
    content(await edit(empty, { action: 'insert', index: 0, kind: 'template', header: 'Review' }));
    assert.equal((await read('Queue')).model.columns[0].label, 'Review');
    await undo();
    panel.panel.dispose();
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path));
    console.log('RSX TreeDataGrid rendered native headers, edited nested columns, and preserved original Rust across dock drop and Undo');
}
