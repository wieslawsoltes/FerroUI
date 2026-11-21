import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

const xaml = '<!-- Repeat 🧭 -->\r\n<StackPanel xmlns="https://github.com/ferroui">'
    + '<Button Content="Review"/><Button Content="Review"/></StackPanel>';
const rsx = [
    'fn review() { rsx! { StackPanel { children([',
    '    rsx! { Button { content("Review 🧭") } },',
    '    rsx! { Button { content("Review 🧭") } },',
    ']), } }; }',
    '',
].join('\r\n');

export async function verifyLayerIdentity(fixture: McpFixture): Promise<void> {
    await verifyReorder(fixture, 'LayerIdentity.xaml', xaml);
    await verifyReorder(fixture, 'layer_identity.rs', rsx);
}

async function verifyReorder({ invoke, content }: McpFixture, path: string, source: string): Promise<void> {
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    try {
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const outline = async () => {
            const frame = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
            return { revision: frame.revision, buttons: frame.nodes.filter(node => node.label === 'Button') };
        };
        const layers = async () => content<{
            layer_epoch: number; scope_epoch: number; isolation: number[];
            layers: { node: number; hidden: boolean; locked: boolean }[];
        }>(await invoke('designer_studio_query', { session, tool: 'designer_layers', arguments: {} }));
        const select = async (node: number, revision: number) =>
            content(await invoke('designer_select', { session, node, expected_revision: revision }));
        const present = async (args: object) => {
            const { revision } = await outline();
            const { layer_epoch, scope_epoch } = await layers();
            content(await invoke('designer_studio_present', { session, expected_revision: revision,
                tool: 'designer_layer', arguments: { ...args, expected_layer_epoch: layer_epoch, expected_scope_epoch: scope_epoch } }));
        };
        const check = async (hidden: boolean[], locked: boolean[]) => {
            const { buttons } = await outline();
            const model = new Map((await layers()).layers.map(layer => [layer.node, layer]));
            assert.deepEqual(buttons.map(node => model.get(node.id)?.hidden), hidden);
            assert.deepEqual(buttons.map(node => model.get(node.id)?.locked), locked);
            return buttons;
        };
        const history = async (action: 'undo' | 'redo', expected: string) => {
            const version = document.version;
            content(await invoke('designer_history', { session, expected_version: version, action }));
            assert.ok(document.version > version, `The reorder is one editor ${action} step`);
            assert.equal(document.getText(), expected);
        };
        const original = await outline();
        assert.equal(original.buttons.length, 2);
        await present({ action: 'set_hidden', nodes: [original.buttons[1].id], hidden: true });
        await present({ action: 'isolate', nodes: [original.buttons[0].id] });
        const { revision, buttons } = await outline();
        await select(buttons[0].id, revision);
        const move = { session, expected_revision: revision, node: buttons[0].id, anchor: buttons[1].id, position: 'after' };
        const version = document.version;
        content(await invoke('designer_place', move));
        assert.ok(document.version > version);
        const edited = document.getText();
        if (path.endsWith('.xaml')) assert.equal(edited, source);
        const moved = await check([true, false], [false, false]);
        assert.deepEqual((await layers()).isolation, [moved[1].id]);
        const selected = content<{ selection: number[] }>(await invoke('designer_wait', { session }));
        assert.deepEqual(selected.selection, [moved[1].id]);
        assert.equal((await invoke('designer_place', move)).isError, true);
        await present({ action: 'set_locked', nodes: [moved[1].id], locked: true });
        await history('undo', source);
        const restored = await check([false, true], [true, false]);
        assert.deepEqual((await layers()).isolation, [restored[0].id]);
        await history('redo', edited);
        await check([true, false], [false, true]);

        await present({ action: 'exit_isolation' });
        const deletion = await outline();
        content(await invoke('designer_remove', { session, expected_revision: deletion.revision, node: deletion.buttons[0].id }));
        const removed = document.getText();
        await check([false], [true]);
        await history('undo', edited);
        await check([true, false], [false, true]);
        await history('redo', removed);
        if (path.endsWith('.rs')) {
            const prefix = new vscode.WorkspaceEdit();
            prefix.insert(uri, new vscode.Position(0, 0), '// Move the view down 🧭\r\n');
            assert.equal(await vscode.workspace.applyEdit(prefix), true);
            await check([false], [true]);
        }
        await present({ action: 'reset' });
        if (path.endsWith('.rs')) await history('undo', removed);
        await history('undo', edited);
        await check([false, false], [false, false]);
        await history('undo', source);
        await check([false, false], [false, false]);
        assert.equal(document.eol, vscode.EndOfLine.CRLF);
    } finally {
        if (document.isDirty) await document.save();
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.label === `Design · ${path}` || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
        await vscode.window.tabGroups.close(tabs);
        await vscode.workspace.fs.delete(uri, { useTrash: false });
    }
    console.log(`Kept identical ${path} controls selected, hidden, and locked across checked moves, deletion, and editor Undo and Redo`);
}
