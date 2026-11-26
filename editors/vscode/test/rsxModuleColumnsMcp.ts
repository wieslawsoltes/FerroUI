import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { applyTextEdits, asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxModuleColumns({ invoke, content }: McpFixture): Promise<void> {
    const directory = 'RustColumnsModule';
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, directory);
    const moduleFile = 'account_columns.rs';
    const viewFile = 'account_view.rs';
    const viewSource = `use crate::account_columns::shared as accounts;
fn view() { rsx! { StackPanel {
    TableView { name: "Accounts", height: 160.0, columns: accounts() }
    TableView { name: "Recent", height: 120.0, columns: accounts() }
} }; }
`;
    const columnSource = [
        'use ferroui_view::{table_columns as list, table_column};',
        'pub(crate) fn shared() -> Columns {',
        '    list([',
        '        table_column().header(/* 🧭 */ "Account").binding_path("Customer.Name"),',
        '        table_column().header("Region").binding_path("Region"),',
        '    ])',
        '}',
        '',
    ].join('\r\n');
    await vscode.workspace.fs.createDirectory(root);
    const write = (file: string, source: string) =>
        vscode.workspace.fs.writeFile(vscode.Uri.joinPath(root, file), Buffer.from(source));
    await Promise.all([
        write('Cargo.toml', '[package]\nname = "columns-module-review"\nversion = "0.0.0"\nedition = "2021"\n[lib]\npath = "entry.rs"\n'),
        write('entry.rs', 'mod account_columns; mod account_view;\n'),
        write(moduleFile, columnSource),
        write(viewFile, viewSource),
    ]);
    const owner = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, moduleFile));
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, viewFile));
    const draft = columnSource.replace('"Account"', '"Draft account"');
    await applyTextEdits(owner, owner.version,
        [{ range: { start: { line: 0, character: 0 }, end: owner.positionAt(columnSource.length) }, newText: draft }]);
    const tool = <T>(name: string, args: object) => invoke(name, args).then(content<T>);
    const { session } = await tool<{ session: number }>('designer_open', { path: `${directory}/${viewFile}` });
    const read = async (name: string) => {
        const frame = await tool<{ status: string }>('designer_wait', { session });
        assert.equal(frame.status, 'Ready');
        const { nodes, revision } = await tool<{ revision: number; nodes: { id: number; name?: string }[] }>('designer_outline', { session });
        const node = nodes.find(node => node.name === name);
        assert.ok(node);
        const model = await tool<TableColumns>('designer_studio_query', { session, tool: 'designer_columns', arguments: { node: node.id } });
        assert.equal(model.reason, null);
        return { revision, model };
    };
    const { model, revision } = await read('Accounts');
    assert.equal(model.factory, 'shared');
    assert.ok(model.sourceUri);
    assert.equal(await realpath(vscode.Uri.parse(model.sourceUri, true).fsPath), await realpath(owner.uri.fsPath));
    assert.equal(model.columns[0].label, 'Draft account');
    assert.match(owner.getText(asRange(model.sourceRange!)), /^list\(\[/);
    const header = model.columns[0].fields.find(field => field.name === 'Header');
    assert.ok(header?.range);
    assert.equal(owner.getText(asRange(header.range)), '"Draft account"');
    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    try {
        await tool('designer_select', { session, expected_revision: revision, node: model.table });
        await designer.revealDesign(panel, 'columns');
        await until(async () => (await designer.columns.getChildren())[0]?.editor.context.token === panel.currentToken);
        const [row] = await designer.columns.getChildren();
        const field = (await designer.columns.getChildren(row)).find(row => row.field?.name === 'Header');
        assert.ok(field);
        await vscode.commands.executeCommand('ferroui.columns.reveal', field);
        const source = vscode.window.visibleTextEditors.find(editor => editor.document === owner);
        assert.equal(source && owner.getText(source.selection), '"Draft account"');
        const current = await read('Accounts');
        const edit = { session, expected_revision: current.revision, tool: 'designer_columns_edit', arguments: {
            table: current.model.table, expected_model: current.model.model, action: 'set', column: current.model.columns[0].node,
            field: 'Header', value: 'Customer account',
        } };
        const version = owner.version;
        const result = await tool<{ edited_source: { uri: string; version: number } }>('designer_studio_edit', edit);
        assert.deepEqual(result.edited_source, { uri: model.sourceUri, version: version + 1 });
        assert.equal(owner.getText(), draft.replace('"Draft account"', '"Customer account"'));
        assert.equal(document.getText(), viewSource);
        assert.equal((await read('Accounts')).model.columns[0].label, 'Customer account');
        assert.equal((await read('Recent')).model.columns[0].label, 'Customer account');
        assert.equal((await invoke('designer_studio_edit', edit)).isError, true);
        const path = `${directory}/${moduleFile}`;
        const undo = async () => {
            const { revision } = await tool<{ revision: string }>('ferroui_workspace_read', { path });
            await tool('ferroui_workspace_file', { path, expected_revision: revision, action: 'undo' });
        };
        await undo();
        assert.equal(owner.getText(), draft);
        await until(async () => (await read('Recent')).model.columns[0].label === 'Draft account');
        await undo();
        assert.equal(owner.getText(), columnSource);
        await until(async () => (await read('Accounts')).model.columns[0].label === 'Account');
    } finally {
        panel.panel.dispose();
    }
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === viewFile || tab.label === moduleFile);
    await vscode.window.tabGroups.close(tabs);
    console.log('RSX Columns revealed the companion module, edited its unsaved declaration and refreshed both tables through Undo');
}
