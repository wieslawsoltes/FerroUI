import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { revision } from '../src/reloadProcess';
import type { RuntimeNode, RuntimeProperty, RuntimeSource } from '../src/runtimeProtocol';
import type { RuntimeSourceEditor } from '../src/runtimeSource';
import { withNativeApp } from './nativeAppFixture';
import { until } from './wait';

export async function verifyLiveXaml(root: vscode.Uri, editor: RuntimeSourceEditor): Promise<void> {
    if (!process.env.FERROUI_TEST_HOSTED_APP) return;
    const file = vscode.Uri.joinPath(root, 'LiveCustomerWindow.xaml');
    const originalBytes = await vscode.workspace.fs.readFile(file);
    const original = new TextDecoder().decode(originalBytes).replace(/\r?\n/g, '\r\n');
    const withBom = Buffer.from('\uFEFF' + original);
    await vscode.workspace.fs.writeFile(file, withBom);
    try {
        await withNativeApp(['--xaml-live', file.fsPath], async client => {
            assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'Heading' })).matched);
            const [heading] = await client.call<RuntimeNode[]>('find_elements', { name: 'Heading' });
            const source = await client.call<RuntimeSource>('get_source', { node_id: heading.node_id });
            assert.equal(source.revision, revision(Buffer.from(original)));
            const selected = await editor.read(client, heading.node_id);
            assert.ok(selected);
            const { document, position } = selected;
            assert.equal(document.uri.toString(), file.toString());
            assert.equal(document.lineAt(position).text[position.character], 'T');
            assert.ok(editor.canApply(selected, 'attached'));

            const [search] = await client.call<RuntimeNode[]>('find_elements', { name: 'Search' });
            await client.call('type_text', { node_id: search.node_id, text: 'Łódź' });
            assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'Result', text: 'Łódź' })).matched);

            const signal = new AbortController().signal;
            const initialVersion = document.version;
            const replacement = 'Customers in Łódź';
            await assert.rejects(
                editor.apply(client, heading, 'Text', replacement, signal, 'attached', () => false, initialVersion),
                /runtime selection or its source revision changed/i,
            );
            await editor.apply(client, heading, 'Text', replacement, signal, 'attached', () => true, initialVersion);
            assert.equal(document.getText(), original.replace('Open customer records', replacement));
            assert.deepEqual(await vscode.workspace.fs.readFile(file), withBom);
            const [property] = await client.call<RuntimeProperty[]>('get_properties', { node_id: heading.node_id, properties: ['Text'] });
            assert.equal(property.value, 'Open customer records');
            await assert.rejects(
                editor.apply(client, heading, 'Text', 'Stale draft', signal, 'attached', () => true, initialVersion),
                /wait for the running app to accept that revision/i,
            );
            await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.One, preserveFocus: false });
            await until(() => vscode.window.activeTextEditor?.document === document);
            await vscode.commands.executeCommand('undo');
            assert.equal(document.getText(), original);
            assert.ok(editor.canApply(selected, 'attached'));
        });
    } finally {
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
            .filter(tab => tab.label === 'LiveCustomerWindow.xaml' || tab.label === 'Design · LiveCustomerWindow.xaml');
        await vscode.window.tabGroups.close(tabs);
        await vscode.workspace.fs.writeFile(file, originalBytes);
    }
    console.log('Live App verified a runtime-loaded XAML source, drove a bound input, applied a guarded Unicode edit and undid it');
}
