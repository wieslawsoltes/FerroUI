import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { DictionaryPlan, DictionaryTarget } from '../src/dictionaryProtocol';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

const source = [
    '<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">',
    '  <Grid.Resources>',
    '    <!-- 🎨 Customer palette -->',
    '    <Color x:Key="Ink">#245783</Color>',
    '    <SolidColorBrush x:Key="Card" Color="{StaticResource Ink}" />',
    '  </Grid.Resources>',
    '  <Border Background="{StaticResource Card}" Padding="24"><TextBlock Text="Customer account" Foreground="White" /></Border>',
    '</Grid>',
    '',
].join('\r\n');

export async function verifyDictionaryExtraction({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const uri = vscode.Uri.joinPath(root, 'Resource extraction.xaml');
    const fileName = 'Customer colors & cards.xaml';
    const destination = vscode.Uri.joinPath(root, fileName);
    const write = (uri: vscode.Uri, text: string) => vscode.workspace.fs.writeFile(uri, Buffer.from(text));
    const file = async (uri: vscode.Uri) => Buffer.from(await vscode.workspace.fs.readFile(uri)).toString();
    const absent = async (uri: vscode.Uri) => vscode.workspace.fs.stat(uri).then(() => false, error => error.code === 'FileNotFound');
    await write(uri, source);
    const doc = await vscode.workspace.openTextDocument(uri);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: 'Resource extraction.xaml' }));
    const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    const entry = nodes.find(node => node.label === 'SolidColorBrush');
    assert.ok(entry);
    const args = { session, node: entry.id, expected_revision: revision };
    const image = async () => {
        const reply = await invoke('designer_preview_image', { session });
        const frame = content<{ revision: number }>(reply);
        const data = reply.content.find(item => item.type === 'image')?.data;
        assert.ok(data);
        return { ...frame, data };
    };
    const before = await image();
    const position = doc.positionAt(doc.getText().indexOf('SolidColorBrush'));
    await until(async () => {
        const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>('vscode.executeCodeActionProvider', uri,
            new vscode.Range(position, position), 'refactor.extract');
        return !!actions?.some(action => action.command?.command === 'ferroui.extractResourceDictionary');
    });
    const target = content<DictionaryTarget>(await invoke('designer_dictionary_extraction', args));
    assert.equal(target.entries, 2);
    assert.equal(target.suggestedFile, 'Resource extraction.Resources.xaml');
    const preview = async (name: string) => content<DictionaryPlan>(await invoke('designer_dictionary_extraction', { ...args, file_name: name }));
    const apply = (plan: DictionaryPlan, overrides = {}) => invoke('designer_extract_dictionary', {
        ...args, file_name: plan.file.fileName, review: plan.review, ...overrides,
    });
    const plan = await preview(fileName);
    assert.ok(plan.file.source.includes('<!-- 🎨 Customer palette -->'));
    assert.ok(await absent(destination));
    assert.equal(doc.getText(), source);
    assert.ok((await apply(plan, { review: '0000000000000000' })).isError);
    assert.ok((await apply(plan, { expected_revision: revision - 1 })).isError);
    const occupied = await preview('Occupied.xaml');
    const occupiedUri = vscode.Uri.joinPath(root, occupied.file.fileName);
    await write(occupiedUri, 'Created while the review was open');
    assert.ok((await apply(occupied)).isError);
    assert.equal(await file(occupiedUri), 'Created while the review was open');
    assert.equal(doc.getText(), source);

    const applied = content<DictionaryPlan>(await apply(plan));
    assert.ok(doc.isDirty);
    assert.equal(await file(uri), source);
    assert.equal(await file(destination), plan.file.source);
    assert.equal(applied.file.uri, destination.toString());
    assert.ok(doc.getText().includes('ResourceInclude Source="Customer%20colors%20%26%20cards.xaml"'));
    const after = await image();
    assert.notEqual(after.revision, before.revision);
    assert.equal(after.data, before.data);
    const at = doc.positionAt(doc.getText().lastIndexOf('Card'));
    await until(async () => {
        const definitions = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', uri, at);
        return definitions?.some(location => location.uri.fsPath.endsWith(`/${fileName}`) || location.uri.fsPath.endsWith(`\\${fileName}`));
    });
    const history = async (action: 'undo' | 'redo') => content(await invoke('designer_history', { session, expected_version: doc.version, action }));
    const changed = doc.getText();
    await history('undo');
    await until(async () => doc.getText() === source && await absent(destination));
    await history('redo');
    await until(async () => doc.getText() === changed && !await absent(destination));
    assert.equal(await file(destination), plan.file.source);
    await history('undo');
    await until(async () => doc.getText() === source && await absent(destination));
    console.log('VS Code reviewed and atomically extracted a resource dictionary, preserved the rendered image and supported one Undo and Redo');
}
