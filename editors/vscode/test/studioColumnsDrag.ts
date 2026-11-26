import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { runEditorHistory } from '../src/editorHistory';
import { until } from './wait';

export async function verifyColumnsDrag(designer: DesignerWorkbench, workspace: vscode.Uri): Promise<void> {
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, 'ColumnsWorkbench.xaml'));
    const original = document.getText();
    const table = original.indexOf('<TableView Name="Portfolio"');
    assert.notEqual(table, -1);
    const panel = await designer.openDocument(document.uri, document.positionAt(table + 1));
    const view = designer.columns;
    const cancellation = new vscode.CancellationTokenSource();
    const token = cancellation.token;
    const order = async (labels: string[]) => {
        await until(async () => {
            const rows = await view.getChildren();
            return rows.length === labels.length && rows.every((row, index) =>
                row.column.label === labels[index] && row.editor.context.token === panel.currentToken);
        });
        return view.getChildren();
    };
    const undo = async () => {
        const version = document.version;
        await runEditorHistory(document, 'undo', () => assert.equal(document.version, version), AbortSignal.timeout(5000));
        await until(() => document.version !== version);
        assert.equal(document.getText(), original);
    };
    try {
        const frame = await panel.waitForSnapshot(AbortSignal.timeout(15_000));
        assert.ok(frame.image, frame.renderError ?? undefined);
        await designer.revealDesign(panel, 'columns');
        const [account, , region] = await order(['Account', 'Owner', 'Region']);
        const width = (await view.getChildren(account)).find(row => row.field?.name === 'Width');
        assert.ok(width);

        const fieldDrag = new vscode.DataTransfer();
        view.handleDrag([width], fieldDrag, token);
        assert.equal([...fieldDrag].length, 0, 'Only top-level columns can be dragged');

        const transfer = new vscode.DataTransfer();
        view.handleDrag([region], transfer, token);
        const [entry] = [...transfer];
        assert.ok(entry, 'The controller supplies a column payload');
        const [mime, item] = entry;
        const serialized = new vscode.DataTransfer();
        serialized.set(mime, new vscode.DataTransferItem(await item.asString()));
        const cancelled = new vscode.CancellationTokenSource();
        try {
            cancelled.cancel();
            await view.handleDrop(width, serialized, cancelled.token);
            assert.equal(document.getText(), original);
        } finally { cancelled.dispose(); }

        const malformed = new vscode.DataTransfer();
        malformed.set(mime, new vscode.DataTransferItem('invalid column payload'));
        await view.handleDrop(width, malformed, token);
        assert.equal(document.getText(), original);

        await view.handleDrop(width, serialized, token);
        await order(['Region', 'Account', 'Owner']);
        assert.ok(document.getText().indexOf('Header="Region"') < document.getText().indexOf('Header="Account"'));
        assert.match(document.getText(), /<!-- Contacts are shared by the customer and account views\. -->/);
        const changed = document.getText();
        await view.handleDrop(undefined, serialized, token);
        assert.equal(document.getText(), changed, 'A transfer cannot move a column again after the model changes');
        await undo();

        const [first] = await order(['Account', 'Owner', 'Region']);
        const append = new vscode.DataTransfer();
        view.handleDrag([first], append, token);
        await view.handleDrop(undefined, append, token);
        await order(['Owner', 'Region', 'Account']);
        await undo();
    } finally {
        cancellation.dispose();
        panel.panel.dispose();
        await until(() => !designer.designs().includes(panel));
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === document.uri.toString());
        await vscode.window.tabGroups.close(tabs);
    }
    console.log('VS Code Columns reordered on a property and at the list end, fenced cancelled and stale drops, and undid each edit');
}
