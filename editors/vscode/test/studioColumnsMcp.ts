import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyColumnsMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ColumnsWorkbench.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const read = async (name: string) => {
        const { renderError } = content<{ renderError: string | null }>(await invoke('designer_wait', { session }));
        assert.ok(!renderError, renderError ?? '');
        const { revision, nodes } = content<{ revision: number; nodes: { id: number; name: string | null }[] }>(
            await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === name);
        assert.ok(node);
        const model = content<TableColumns>(await invoke('designer_studio_query', {
            session, tool: 'designer_columns', arguments: { node: node.id },
        }));
        assert.equal(model.table, node.id);
        assert.equal(model.reason, null);
        return { revision, model };
    };
    const edit = (target: Awaited<ReturnType<typeof read>>, change: object, model = target.model.model) =>
        invoke('designer_studio_edit', {
            session, expected_revision: target.revision, tool: 'designer_columns_edit',
            arguments: { table: target.model.table, expected_model: model, ...change },
        });
    const undo = async () => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
    };

    const portfolio = await read('Portfolio');
    assert.deepEqual(portfolio.model.columns.map(column => column.label), ['Account', 'Owner', 'Region']);
    const owner = portfolio.model.columns[1];
    assert.equal(document.getText(asRange(owner.range)), 'TableViewColumn');
    content(await edit(portfolio, { action: 'move', column: owner.node, index: 0 }));
    const reordered = await read('Portfolio');
    assert.deepEqual(reordered.model.columns.map(column => column.label), ['Owner', 'Account', 'Region']);
    assert.match(document.getText(), /<!-- Contacts are shared by the customer and account views\. -->/);
    const reject = { action: 'set', column: reordered.model.columns[0].node, field: 'Width', value: '2*' };
    const before = document.version;
    assert.ok((await edit(reordered, reject, portfolio.model.model)).isError, 'A stale column model is rejected on a current frame');
    assert.equal(document.version, before);
    await undo();

    const grid = await read('Status');
    assert.deepEqual(grid.model.columns.map(column => column.kind), ['text', 'template', 'checkBox']);
    const template = grid.model.columns[1].fields.find(field => field.name === 'CellTemplate');
    assert.equal(template?.editable, false);
    assert.ok(template?.range);
    assert.equal(document.getText(asRange(template.range)), 'TreeDataGridTemplateColumn.CellTemplate');
    const active = grid.model.columns[2];
    content(await edit(grid, { action: 'set', column: active.node, field: 'Width', value: '124' }));
    assert.equal(document.getText(), original.replace('Header="Active" Width="108"', 'Header="Active" Width="124"'));
    await undo();
    const restored = await read('Status');
    content(await edit(restored, { action: 'insert', index: 1, kind: 'template' }));
    assert.equal((await read('Status')).model.columns.length, 4);
    assert.ok(document.getText().includes('<TextBlock Text="{ReflectionBinding}"'));
    await undo();
    const flat = await read('Status');
    content(await edit(flat, { action: 'insert', index: 3, kind: 'expander' }));
    const hierarchy = await read('Status');
    const expander = hierarchy.model.columns[3];
    assert.equal(expander.fields.find(field => field.name === 'ChildrenBinding')?.value, '{ReflectionBinding Children}');
    assert.equal(expander.children[0].kind, 'text');
    assert.ok((await edit(hierarchy, { action: 'clear', column: expander.node, field: 'ChildrenBinding' })).isError);
    assert.ok((await edit(hierarchy, { action: 'clear', column: expander.children[0].node, field: 'Binding' })).isError);
    await undo();

    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code columns shared native authoring plans, rejected stale models, preserved XAML templates and used one undo per edit');
}
