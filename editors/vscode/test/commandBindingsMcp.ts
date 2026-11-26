import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

interface CommandReport {
    uri: string;
    version: number;
    slots: { property: string; binding: { editable: boolean; authored: string | null } }[];
    members: { path: string; command: boolean }[];
}

export async function verifyCommandBindings({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const path = 'CommandBindingsWorkbench.xaml';
    const source = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, path));
    const original = source.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const read = async (name: string) => {
        content(await invoke('designer_wait', { session }));
        const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        const target = { session, expected_revision: revision, node };
        const report = content<CommandReport>(await invoke('designer_command_bindings', target));
        assert.equal(report.slots[0]?.property, 'Command');
        return { target, report };
    };
    const ready = await read('SaveButton');
    assert.ok(ready.report.members.some(member => member.path === 'Command' && member.command));
    assert.ok(ready.report.members.some(member => member.path === 'Content' && !member.command));
    const inputs = { ...ready.target, expected_uri: ready.report.uri, expected_version: ready.report.version };
    const binding = { property: 'Command', command: { kind: 'bind', path: 'Command' }, parameter: { kind: 'bind', path: 'Content' } };
    assert.equal((await invoke('designer_command_binding', { ...inputs,
        change: { ...binding, command: { kind: 'bind', path: 'Content' } } })).isError, true);
    assert.equal(source.getText(), original);
    content(await invoke('designer_command_binding', { ...inputs, change: binding }));
    assert.match(source.getText(), /Command="\{Binding Command\}"/);
    assert.match(source.getText(), /CommandParameter="\{Binding Content\}"/);
    assert.equal((await invoke('designer_command_binding', { ...inputs, change: binding })).isError, true);
    content(await invoke('designer_history', { session, expected_version: source.version, action: 'undo' }));
    assert.equal(source.getText(), original);

    const unsupported = await read('NamedCommandButton');
    assert.equal(unsupported.report.slots[0].binding.editable, false);
    assert.equal(unsupported.report.slots[0].binding.authored, '{Binding #SaveButton.Command}');
    assert.equal((await invoke('designer_command_binding', { ...unsupported.target,
        expected_uri: unsupported.report.uri, expected_version: unsupported.report.version,
        change: { ...binding, command: { kind: 'clear' } } })).isError, true);
    assert.equal(source.getText(), original);
    console.log('VS Code command bindings selected a typed command and parameter, rejected stale edits and preserved named sources');
}
