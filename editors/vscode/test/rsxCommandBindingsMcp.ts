import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { Inspection, OutlineNode, Range } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

interface Field {
    authored: string | null;
    range: Range;
    typeName: string;
    editable: boolean;
}

interface Report {
    model: string;
    slots: { property: string; binding: Field; parameter: Field }[];
}

export async function verifyRsxCommandBindings({ invoke, content }: McpFixture): Promise<void> {
    const filename = 'ApprovalCommands.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, filename);
    const original = `fn before() -> Element { rsx! { TextBlock { text: "Account 🧭" } } }
fn approvals() -> Element {
    rsx! { StackPanel {
        Button { name: "Approve", command: /* keep 🪴 */ previous(), command_parameter(current()), content: "Approve" }
        Button { name: "Computed", with(install_runtime_bindings) }
        {runtime_help()}
    } }
}
`.replaceAll('\n', '\r\n');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(original));
    const document = await vscode.workspace.openTextDocument(uri);
    const cursor = document.positionAt(original.indexOf('rsx! { StackPanel'));
    const { session } = content<{ session: number }>(await invoke('designer_open', {
        path: filename, line: cursor.line, character: cursor.character,
    }));
    const read = async (name: string) => {
        content(await invoke('designer_wait', { session }));
        const frame = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = frame.nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        const report = content<Report>(await invoke('designer_studio_query', {
            session, tool: 'designer_rsx_commands', arguments: { node },
        }));
        return { node, report, revision: frame.revision };
    };
    const apply = (target: Awaited<ReturnType<typeof read>>, change: object, model = target.report.model) =>
        invoke('designer_studio_edit', {
            session, expected_revision: target.revision, tool: 'designer_rsx_command_edit',
            arguments: { node: target.node, expected_model: model, change },
        });
    const undo = async () => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
    };
    const initial = await read('Approve');
    const slot = initial.report.slots[0];
    assert.equal(slot.property, 'Command');
    assert.ok(slot.binding.typeName.includes('ICommand'));
    assert.equal(document.getText(asRange(slot.binding.range)), 'previous()');
    assert.equal(document.getText(asRange(slot.parameter.range)), 'current()');
    const inspector = content<Inspection>(await invoke('designer_inspect', { session, node: initial.node }));
    assert.equal(inspector.rustCommandBinding, true);
    const update = { property: 'Command', command: { kind: 'set', expression: 'approve.clone()' },
        parameter: { kind: 'set', expression: 'account.clone()' } };
    const invalid = { ...update, parameter: { kind: 'set', expression: 'account), invalid: true' } };
    assert.equal((await apply(initial, invalid)).isError, true);
    assert.equal(document.getText(), original);
    content(await apply(await read('Approve'), update));
    assert.equal(document.getText(), original.replace('previous()', 'approve.clone()').replace('current()', 'account.clone()'));
    await undo();
    const afterUndo = await read('Approve');
    assert.equal((await apply(afterUndo, update, initial.report.model)).isError, true);
    const clear = { property: 'Command', command: { kind: 'clear' }, parameter: { kind: 'clear' } };
    content(await apply(await read('Approve'), clear));
    assert.ok(!document.getText().includes('previous()'));
    assert.ok(!document.getText().includes('current()'));
    assert.ok(document.getText().includes('/* keep 🪴 */'));
    await undo();
    const computed = await read('Computed');
    assert.equal(computed.report.slots[0].binding.editable, false);
    assert.equal((await apply(computed, update)).isError, true);
    assert.equal(document.getText(), original);
    console.log('RSX command bindings applied and cleared mapped expressions in one Undo step and rejected stale or indirect assignments');
}
