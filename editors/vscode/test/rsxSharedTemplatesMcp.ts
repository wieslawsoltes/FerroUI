import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TableColumns } from '../src/columnEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { OutlineNode, Range, RustTemplateNavigation } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

interface Slot {
    property: string;
    range: Range;
    sourceUri: string | null;
    rustBody: Range | null;
    rustBodyUri: string | null;
    rustFactory: string | null;
    canCreate: boolean;
    canReset: boolean;
}

const consumerSource = `use ferroui_view::template as item;
fn shared_card() -> Template {
    item::<String, _>(|data| rsx! { StackPanel {
        TextBlock { name: "LocalTitle", text: "Recent accounts" }
        TextBlock { text: data }
    } })
}
fn overview() -> Element {
    rsx! { StackPanel {
        ListBox { name: "Directory", item_template: shared_card() }
        TableView { name: "Accounts", height: 180.0, columns: crate::account_columns::shared() }
    } }
}
`;
const columnsSource = `use ferroui_view::{table_columns as list, table_column};
use crate::account_templates::account_cell as cell;
pub fn shared() -> Columns {
    list([table_column().header("Account").cell_template(/* caller 🧭 */ cell())])
}
`;
const bodySource = `use ferroui_view::template as typed;
fn overview() -> Element { rsx! { TextBlock { name: "Other", text: "Other view" } } }
pub fn account_cell() -> Template {
    typed::<String, _>(|row| rsx! { StackPanel {
        TextBlock { name: "SharedTitle", text: "Customer profile 🧭" }
        TextBlock { text: row }
    } })
}
`.replaceAll('\n', '\r\n');

export async function verifySharedRsxTemplates({ invoke, content }: McpFixture): Promise<void> {
    const folder = 'RustTemplateModules';
    const viewFile = 'account_review.rs';
    const columnsFile = 'account_columns.rs';
    const bodyFile = 'account_templates.rs';
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, folder);
    await vscode.workspace.fs.createDirectory(root);
    const write = (file: string, text: string) =>
        vscode.workspace.fs.writeFile(vscode.Uri.joinPath(root, file), Buffer.from(text));
    await Promise.all([
        write('Cargo.toml', '[package]\nname = "template-modules"\nversion = "0.0.0"\nedition = "2021"\n[lib]\npath = "entry.rs"\n'),
        write('entry.rs', 'mod account_review; mod account_columns; mod account_templates;\n'),
        write(viewFile, consumerSource), write(columnsFile, columnsSource), write(bodyFile, bodySource),
    ]);
    const open = (file: string) => vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, file));
    const [view, columns, body] = await Promise.all([open(viewFile), open(columnsFile), open(bodyFile)]);
    const tool = <T>(name: string, args: object) => invoke(name, args).then(content<T>);
    const start = view.positionAt(consumerSource.indexOf('rsx! { StackPanel', consumerSource.indexOf('fn overview')));
    const { session } = await tool<{ session: number }>('designer_open', {
        path: `${folder}/${viewFile}`, line: start.line, character: start.character,
    });
    const { session: sharedSession } = await tool<{ session: number }>('designer_open', { path: `${folder}/${bodyFile}` });
    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    const ownerPanel = designer.designs().find(panel => panel.session === sharedSession)!;
    const outline = async (session: number) => {
        const ready = await tool<{ status: string; renderError?: string }>('designer_wait', { session });
        assert.equal(ready.status, 'Ready', ready.renderError);
        return tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
    };
    const query = <T>(name: string, node: number) => tool<T>('designer_studio_query', { session, tool: name, arguments: { node } });
    const read = async (name: string, isColumn = false) => {
        const frame = await outline(session);
        const control = frame.nodes.find(node => node.name === name);
        assert.ok(control);
        const node = isColumn ? (await query<TableColumns>('designer_columns', control.id)).columns[0].node : control.id;
        const { model, slots } = await query<{ model: string; slots: Slot[] }>('designer_item_templates', node);
        assert.ok(slots.length);
        return { frame, node, model, slot: slots[0] };
    };
    const run = (target: Awaited<ReturnType<typeof read>>, action: 'preview' | 'reset' | 'create_inline') =>
        invoke(action === 'preview' ? 'designer_studio_present' : 'designer_studio_edit', {
            session, expected_revision: target.frame.revision, tool: 'designer_item_template',
            arguments: { node: target.node, property: target.slot.property, expected_model: target.model,
                action, ...(action === 'create_inline' && { rust_type: 'String' }) },
        });
    const back = async (target: number) => tool<{ returned_session?: number }>('designer_studio_present', {
        session: target, expected_revision: (await outline(target)).revision, tool: 'designer_definition', arguments: { action: 'exit' },
    });

    try {
        assert.ok((await outline(sharedSession)).nodes.some(node => node.name === 'Other'));
        const local = await read('Directory');
        assert.equal(local.slot.rustFactory, 'shared_card');
        assert.equal(local.slot.rustBodyUri, null);
        const first = content<{ rust_template: string }>(await run(local, 'preview'));
        assert.equal(first.rust_template, 'ItemTemplate');
        assert.ok((await outline(session)).nodes.some(node => node.name === 'LocalTitle' && node.bounds));
        await back(session);

        const cell = await read('Accounts', true);
        const { slot } = cell;
        assert.equal(slot.rustFactory, 'account_cell');
        assert.equal(vscode.Uri.parse(slot.sourceUri!, true).fsPath, columns.uri.fsPath);
        assert.equal(vscode.Uri.parse(slot.rustBodyUri!, true).fsPath, body.uri.fsPath);
        assert.equal(columns.getText(asRange(slot.range)), 'cell()');
        assert.ok(body.getText(asRange(slot.rustBody!)).includes('Customer profile 🧭'));
        assert.ok(slot.canReset);
        const opened = content<{ opened_session: number; opened_template: RustTemplateNavigation }>(await run(cell, 'preview'));
        assert.equal(opened.opened_session, sharedSession);
        assert.deepEqual(opened.opened_template.range, slot.rustBody);
        const frame = await outline(sharedSession);
        const title = frame.nodes.find(node => node.name === 'SharedTitle');
        assert.ok(title?.bounds);
        await tool('designer_edit_property', { session: sharedSession, expected_revision: frame.revision, node: title.id,
            property: 'Text', value: 'Account profile 🧭', action: 'set' });
        assert.equal(body.getText(), bodySource.replace('Customer profile', 'Account profile'));
        assert.equal(view.getText(), consumerSource);
        assert.equal(columns.getText(), columnsSource);
        await tool('designer_history', { session: sharedSession, expected_version: body.version, action: 'undo' });
        assert.equal(body.getText(), bodySource);
        assert.equal((await back(sharedSession)).returned_session, session);
        assert.ok((await outline(sharedSession)).nodes.some(node => node.name === 'Other'));
        assert.ok((await invoke('designer_studio_present', { session, expected_revision: cell.frame.revision,
            tool: 'designer_item_template', arguments: { node: cell.node, property: slot.property,
                action: 'preview', expected_model: cell.model } })).isError);

        const path = `${folder}/${columnsFile}`;
        const undoAssignment = async () => {
            const { revision } = await tool<{ revision: string }>('ferroui_workspace_read', { path });
            await tool('ferroui_workspace_file', { path, expected_revision: revision, action: 'undo' });
        };
        const reset = content<{ edited_source: { uri: string } }>(await run(await read('Accounts', true), 'reset'));
        assert.equal(reset.edited_source.uri, slot.sourceUri);
        assert.ok(!columns.getText().includes('.cell_template('));
        assert.ok(columns.getText().includes('/* caller 🧭 */'));
        const empty = await read('Accounts', true);
        assert.ok(empty.slot.canCreate);
        content(await run(empty, 'create_inline'));
        assert.ok(columns.getText().includes('::ferroui_view::template::<String, _>'));
        assert.equal((await read('Accounts', true)).slot.rustBodyUri, slot.sourceUri);
        await undoAssignment();
        await undoAssignment();
        assert.equal(columns.getText(), columnsSource);
        assert.equal(body.getText(), bodySource);
        assert.equal(view.getText(), consumerSource);
    } finally {
        panel.panel.dispose();
        ownerPanel.panel.dispose();
    }
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => [viewFile, columnsFile, bodyFile].includes(tab.label));
    await vscode.window.tabGroups.close(tabs);
    console.log('Shared Rust templates opened in the original scope, returned to the consumer, and kept body and assignment Undo in their own files');
}
