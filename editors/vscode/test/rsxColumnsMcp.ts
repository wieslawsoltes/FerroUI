import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxColumns({ invoke, content }: McpFixture): Promise<void> {
    const path = 'column_workbench.rs';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const tool = async <T>(name: string, arguments_: object) => content<T>(await invoke(name, arguments_));
    const { session } = await tool<{ session: number }>('designer_open', { path });
    const outline = async () => {
        const ready = await tool<{ renderError: string | null }>('designer_wait', { session });
        assert.ok(!ready.renderError, ready.renderError ?? undefined);
        return tool<{ revision: number; nodes: { id: number; name?: string }[] }>('designer_outline', { session });
    };
    const read = async (name = 'Accounts') => {
        const frame = await outline();
        const node = frame.nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        const model = await tool<TableColumns>('designer_studio_query', { session, tool: 'designer_columns', arguments: { node } });
        assert.equal(model.table, node);
        return { revision: frame.revision, model };
    };
    const edit = (target: Awaited<ReturnType<typeof read>>, change: object, model = target.model.model) => invoke('designer_studio_edit', {
        session, expected_revision: target.revision, tool: 'designer_columns_edit',
        arguments: { table: target.model.table, expected_model: model, ...change },
    });
    const undo = async () => {
        await tool('designer_history', { session, expected_version: document.version, action: 'undo' });
        await until(() => document.getText() === original);
    };
    const image = async () => {
        await outline();
        const response = await invoke('designer_preview_image', { session });
        assert.equal(response.isError, false);
        const bytes = response.content.find(item => item.type === 'image')?.data;
        assert.ok(bytes);
        return bytes;
    };

    const computed = await read('Computed');
    assert.match(computed.model.reason ?? '', /defined by Rust/);
    assert.equal(document.getText(asRange(computed.model.sourceRange!)), 'computed_columns()');
    assert.ok((await edit(computed, { action: 'insert', index: 0, kind: 'table' })).isError);
    const initial = await read();
    assert.equal(initial.model.reason, null);
    assert.deepEqual(initial.model.columns.map(row => row.label), ['Account', 'Region', 'Status']);
    const template = initial.model.columns[2].fields.find(field => field.name === 'CellTemplate');
    assert.ok(template?.range && !template.editable);
    assert.equal(document.getText(asRange(template.range)), 'status_cell()');
    assert.ok((await edit(initial, { action: 'duplicate', column: initial.model.columns[2].node })).isError);
    const before = await image();
    const reviewed = await read();
    content(await edit(reviewed, { action: 'set', column: reviewed.model.columns[0].node, field: 'Header', value: 'Customer' }));
    const updated = await read();
    assert.equal(updated.model.columns[0].label, 'Customer');
    assert.equal(document.getText(), original.replace('header("Account")', 'header("Customer")'));
    assert.notEqual(await image(), before, 'Editing the Rust header updates the native table');
    assert.ok((await edit(updated, { action: 'move', column: updated.model.columns[1].node, index: 0 }, reviewed.model.model)).isError);
    await undo();

    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    const current = await read();
    await tool('designer_select', { session, expected_revision: current.revision, node: current.model.table });
    await designer.revealDesign(panel, 'columns');
    const view = designer.columns;
    await until(async () => {
        const rows = await view.getChildren();
        return rows.length === 3 && rows[0].column.label === 'Account' && rows[0].editor.context.token === panel.currentToken;
    });
    const [account, region] = await view.getChildren();
    const cancellation = new vscode.CancellationTokenSource();
    try {
        const transfer = new vscode.DataTransfer();
        view.handleDrag([region], transfer, cancellation.token);
        assert.equal([...transfer].length, 1);
        await view.handleDrop(account, transfer, cancellation.token);
        const moved = await read();
        assert.deepEqual(moved.model.columns.map(row => row.label), ['Region', 'Account', 'Status']);
        assert.ok(document.getText().indexOf('header("Region")') < document.getText().indexOf('// Main account name 🧭'));
        await undo();
    } finally { cancellation.dispose(); }
    const originalOrder = await read();
    content(await edit(originalOrder, { action: 'insert', index: 1, kind: 'table', header: 'Owner' }));
    const inserted = await read();
    assert.deepEqual(inserted.model.columns.map(row => row.label), ['Account', 'Owner', 'Region', 'Status']);
    await undo();
    panel.panel.dispose();
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path);
    await vscode.window.tabGroups.close(tabs);
    console.log('RSX Columns changed a native header, reordered from the dock, preserved computed Rust and undid each edit');
}
