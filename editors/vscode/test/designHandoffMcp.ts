import { strict as assert } from 'node:assert';
import { symlink, unlink } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { HandoffPlan } from '../src/designHandoffProtocol';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==', 'base64');
const xaml = '<StackPanel xmlns="https://github.com/ferroui" Name="Layout"><TextBlock Text="Checkout"/></StackPanel>';
const rust = [
    'fn title() { rsx! { TextBlock { text: "Primary 🛍" } }; }',
    'fn checkout() { rsx! { StackPanel { name: "Checkout",',
    '    TextBlock { text: headline() /* Keep the expression 🧭 */ }',
    '} }; }',
    '',
].join('\r\n');
const definitions = [
    '<StackPanel xmlns="https://github.com/ferroui" xmlns:id="http://schemas.microsoft.com/winfx/2006/xaml">',
    ...['Compact', 'Expanded'].map(scope => `  <StackPanel id:Name="${scope}"><StackPanel.Resources><!-- ○ 🧪 --><DataTemplate id:Key="Tile"><TextBlock Text="${scope}"/></DataTemplate></StackPanel.Resources></StackPanel>`),
    '</StackPanel>',
].join('\r\n');

export async function verifyDesignHandoff({ invoke, content }: McpFixture): Promise<void> {
    const base = 'Design Handoff';
    const linked = 'Linked Handoff';
    const root = vscode.workspace.workspaceFolders![0].uri;
    const alias = vscode.Uri.joinPath(root, linked);
    let aliasCreated = false;
    const file = (name: string) => vscode.Uri.joinPath(root, base, name);
    const path = (name: string) => `${base}/${name}`;
    const manifest = path('Checkout.ferroui-handoff.json');
    const send = async <T>(tool: string, args: object) => content<T>(await invoke(tool, args));
    const plan = (session: number) => send<HandoffPlan>('designer_design_handoff', { session, manifest });
    const apply = async (session: number, id: string, review: string | null, action: string, options = {}) =>
        invoke('designer_apply_handoff', { session, manifest, action, id, review, ...options });
    const outline = (session: number) => send<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
    const mappings = [
        { id: 'checkout-action', name: 'Checkout action', target: { kind: 'control', toolbox: 'Button' } },
        { id: 'lookalike', target: { kind: 'control', toolbox: 'MissingButton' } },
        { id: 'template', target: { kind: 'definition', path: path('Definitions.xaml'), key: 'Tile', scope: 'Expanded' } },
        { id: 'ambiguous', target: { kind: 'definition', path: path('Definitions.xaml'), key: 'Tile' } },
        { id: 'mark', target: { kind: 'asset', path: path('Assets/Brand Icon.png'), transfer: { direction: 'import', file: 'Mark.png' } } },
        { id: 'export', target: { kind: 'asset', path: path('Assets/Brand Icon.png'), transfer: { direction: 'export', file: 'Delivery/Mark.png' } } },
    ];
    await vscode.workspace.fs.createDirectory(file(''));
    await Promise.all([
        vscode.workspace.fs.writeFile(file('Checkout.xaml'), Buffer.from(xaml)),
        vscode.workspace.fs.writeFile(file('checkout.rs'), Buffer.from(rust)),
        vscode.workspace.fs.writeFile(file('Definitions.xaml'), Buffer.from(definitions)),
        vscode.workspace.fs.writeFile(file('Mark.png'), image),
        vscode.workspace.fs.writeFile(file('Checkout.ferroui-handoff.json'), Buffer.from(JSON.stringify({ version: 1, mappings }))),
    ]);
    const knownTabs = new Set(['Design · Checkout.xaml', 'Design · checkout.rs', 'Handoff · Checkout.xaml', 'Handoff · checkout.rs']);
    const close = async () => {
        const folders = [file('').path, alias.path];
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(({ label, input }) => knownTabs.has(label)
            || (input instanceof vscode.TabInputText && folders.some(folder => input.uri.path.startsWith(folder + '/'))));
        if (tabs.length) assert.ok(await vscode.window.tabGroups.close(tabs));
    };
    try {
        const { session } = await send<{ session: number }>('designer_open', { path: path('Checkout.xaml') });
        const reviewed = (await plan(session)).rows;
        const get = (id: string) => reviewed.find(row => row.id === id)!;
        assert.match(get('lookalike').issue!, /exact Toolbox ID/);
        assert.match(get('ambiguous').issue!, /More than one definition/);
        assert.deepEqual(get('template').definition?.definition.range.start, { line: 2, character: definitions.split('\r\n')[2].indexOf('<DataTemplate') + 1 });
        assert.equal(get('mark').reference, 'Assets/Brand%20Icon.png');
        assert.equal(get('mark').copy?.direction, 'import');
        assert.equal(get('mark').insertion, null);

        const changed = Buffer.concat([image, Buffer.from([5])]);
        await vscode.workspace.fs.writeFile(file('Mark.png'), changed);
        assert.ok((await apply(session, 'mark', get('mark').review, 'copy')).isError);
        const fresh = (await plan(session)).rows.find(row => row.id === 'mark')!;
        await vscode.workspace.fs.createDirectory(file('Assets'));
        await vscode.workspace.fs.writeFile(file('Assets/Brand Icon.png'), Buffer.from('occupied'));
        assert.ok((await apply(session, 'mark', fresh.review, 'copy')).isError);
        assert.equal(Buffer.from(await vscode.workspace.fs.readFile(file('Assets/Brand Icon.png'))).toString(), 'occupied');
        await vscode.workspace.fs.delete(file('Assets/Brand Icon.png'));
        content(await apply(session, 'mark', fresh.review, 'copy'));
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(file('Assets/Brand Icon.png'))), changed);

        const rows = (await plan(session)).rows;
        const exported = rows.find(row => row.id === 'export')!;
        content(await apply(session, 'export', exported.review, 'copy'));
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(file('Delivery/Mark.png'))), changed);
        const mark = (await plan(session)).rows.find(row => row.id === 'mark')!;
        let frame = await outline(session);
        const placement = { expected_revision: frame.revision, anchor: frame.nodes.find(node => node.name === 'Layout')!.id, position: 'inside' };
        content(await apply(session, 'mark', mark.review, 'place', placement));
        const document = await vscode.workspace.openTextDocument(file('Checkout.xaml'));
        assert.match(document.getText(), /Source="Assets\/Brand%20Icon.png"/);
        assert.ok((await apply(session, 'mark', mark.review, 'place', placement)).isError);
        await send('designer_history', { session, expected_version: document.version, action: 'undo' });
        assert.equal(document.getText(), xaml);

        const template = (await plan(session)).rows.find(row => row.id === 'template')!;
        content(await apply(session, 'template', template.review, 'source'));
        assert.equal(vscode.window.activeTextEditor?.document.getText(vscode.window.activeTextEditor.selection), 'DataTemplate');
        const draft = vscode.window.activeTextEditor!.document;
        const prepare = new vscode.WorkspaceEdit();
        prepare.insert(draft.uri, new vscode.Position(0, 0), '<!-- 🧭 reviewed draft -->\r\n');
        assert.ok(await vscode.workspace.applyEdit(prepare));
        assert.ok((await apply(session, 'template', template.review, 'source')).isError);
        const updated = (await plan(session)).rows.find(row => row.id === 'template')!;
        assert.equal(updated.definition?.definition.range.start.line, 3);
        await vscode.commands.executeCommand('workbench.action.files.revert');
        await until(() => !draft.isDirty);
        const open = await send<{ opened: boolean }>('designer_apply_handoff', { session, manifest, action: 'openReview' });
        assert.equal(open.opened, true);
        await until(() => vscode.window.tabGroups.all.flatMap(group => group.tabs).some(tab => tab.label === 'Handoff · Checkout.xaml'));

        const rsx = (await send<{ session: number }>('designer_open', { path: path('checkout.rs'), line: 1, character: 20 })).session;
        frame = await outline(rsx);
        const control = (await plan(rsx)).rows.find(row => row.id === 'checkout-action')!;
        assert.match(control.reference!, /rsx!/);
        content(await apply(rsx, 'checkout-action', control.review, 'place', {
            expected_revision: frame.revision, anchor: frame.nodes.find(node => node.name === 'Checkout')!.id, position: 'inside',
        }));
        const rustDoc = await vscode.workspace.openTextDocument(file('checkout.rs'));
        assert.match(rustDoc.getText(), /Button \{/);
        assert.match(rustDoc.getText(), /text: headline\(\) \/\* Keep the expression 🧭 \*\//);
        assert.equal(rustDoc.getText().split('\r\n')[0], rust.split('\r\n')[0]);
        await send('designer_history', { session: rsx, expected_version: rustDoc.version, action: 'undo' });
        assert.equal(rustDoc.getText(), rust);

        await symlink(file('').fsPath, alias.fsPath, process.platform === 'win32' ? 'junction' : 'dir');
        aliasCreated = true;
        const opened = await send<{ session: number }>('designer_open', { path: `${linked}/Checkout.xaml` });
        const asset = (await plan(opened.session)).rows.find(row => row.id === 'mark');
        assert.equal(asset?.reference, 'Assets/Brand%20Icon.png');
    } finally {
        for (const name of ['Checkout.xaml', 'checkout.rs', 'Definitions.xaml']) {
            const draft = vscode.workspace.textDocuments.find(document => document.uri.toString() === file(name).toString() && document.isDirty);
            if (draft) {
                await vscode.window.showTextDocument(draft);
                await vscode.commands.executeCommand('workbench.action.files.revert');
            }
        }
        await close();
        if (aliasCreated) await unlink(alias.fsPath);
        await vscode.workspace.fs.delete(file(''), { recursive: true, useTrash: false });
    }
    console.log('Design handoff reviewed scoped definitions, copied exact assets in both directions and placed checked XAML and RSX source with one Undo');
}
