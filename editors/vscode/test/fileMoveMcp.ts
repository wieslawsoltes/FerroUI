import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyFileMoveMcp({ invoke, content }: McpFixture): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const directory = vscode.Uri.joinPath(workspace, 'Reviewed file move');
    const uri = (file: string) => vscode.Uri.joinPath(directory, file);
    const source = 'Palette.xaml';
    const destination = 'Archive/Palettes & Cards.xaml';
    const from = uri(source);
    const to = uri(destination);
    const view = uri('Customers.xaml');
    const additional = uri('Other.xaml');
    const ns = 'xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"';
    const root = `<ResourceDictionary ${ns}>`;
    const merged = (value: string) => `<ResourceDictionary.MergedDictionaries><ResourceInclude Source="${value}" /></ResourceDictionary.MergedDictionaries>`;
    const palette = [root, `  <!-- 🎨 --> ${merged('Base.xaml')}`,
        '  <SolidColorBrush x:Key="Accent" Color="{StaticResource Ink}" />', '</ResourceDictionary>', ''].join('\r\n');
    const referenced = [`<Border ${ns} Background="{StaticResource Accent}">`,
        `  <Border.Resources>${root}${merged(source)}</ResourceDictionary></Border.Resources>`,
        '  <TextBlock Text="Customer accounts" Foreground="White" />', '</Border>', ''].join('\r\n');
    const write = (file: vscode.Uri, value: string) => vscode.workspace.fs.writeFile(file, Buffer.from(value));
    const disk = async (file: vscode.Uri) => Buffer.from(await vscode.workspace.fs.readFile(file)).toString();
    const missing = (file: vscode.Uri) => vscode.workspace.fs.stat(file).then(() => false, error => error.code === 'FileNotFound');
    await vscode.workspace.fs.createDirectory(uri('Archive'));
    await Promise.all([write(from, palette), write(view, referenced),
        write(uri('Base.xaml'), `${root}<Color x:Key="Ink">#143965</Color></ResourceDictionary>`)]);
    const doc = await vscode.workspace.openTextDocument(view);
    const relative = (file: vscode.Uri) => vscode.workspace.asRelativePath(file, false).replaceAll('\\', '/');
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: relative(view) }));
    const snapshot = async () => {
        const result = await invoke('designer_preview_image', { session });
        const image = result.content.find(item => item.type === 'image')?.data;
        assert.ok(image);
        return image;
    };
    const before = await snapshot();
    const request = { path: relative(from), destination: relative(to) };
    const plan = async () => content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', request));
    const apply = (review: string) => invoke('ferroui_workspace_move_xaml', { ...request, review });
    const first = await plan();
    assert.equal(first.documents.length, 2);
    assert.equal(await disk(from), palette);
    assert.ok(await missing(to));
    await write(additional, `${root}${merged(source)}</ResourceDictionary>`);
    assert.ok((await apply(first.review)).isError, 'New unopened consumers make a review stale');
    const ready = await plan();
    assert.equal(ready.documents.length, 3);
    assert.ok(ready.documents.find(item => item.edit.uri.endsWith('Palette.xaml'))?.after.includes('Source="../Base.xaml"'));
    assert.ok((await apply('stale review')).isError);
    assert.ok(await missing(to));
    assert.equal(doc.getText(), referenced);
    await write(to, 'Occupied during review');
    assert.ok((await apply(ready.review)).isError);
    assert.equal(await disk(to), 'Occupied during review');
    assert.equal(await disk(from), palette);
    await vscode.workspace.fs.delete(to, { useTrash: false });
    content<FileMovePlan>(await apply((await plan()).review));
    assert.ok(await missing(from));
    const relocated = await vscode.workspace.openTextDocument(to);
    const other = await vscode.workspace.openTextDocument(additional);
    assert.ok(relocated.getText().includes('Source="../Base.xaml"'));
    const changedLink = 'Source="Archive/Palettes%20%26%20Cards.xaml"';
    assert.ok(doc.getText().includes(changedLink));
    assert.ok(other.getText().includes(changedLink));
    assert.equal(await snapshot(), before);
    const at = doc.positionAt(doc.getText().indexOf('Accent'));
    await until(async () => {
        const definitions = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', view, at);
        return definitions?.some(location => location.uri.fsPath === to.fsPath);
    });
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('undo');
    await until(async () => doc.getText() === referenced && await missing(to));
    assert.equal(await disk(from), palette);
    assert.match(other.getText(), /Source="Palette.xaml"/);
    await vscode.commands.executeCommand('redo');
    await until(async () => doc.getText().includes(changedLink) && await missing(from));
    await vscode.commands.executeCommand('undo');
    await until(async () => doc.getText() === referenced && await missing(to));
    const renamedView = uri('Archive/Customers (Moved).xaml');
    const moveView = { path: relative(view), destination: relative(renamedView) };
    const planView = async () => content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', moveView));
    const moveReviewedView = (review: string) => invoke('ferroui_workspace_move_xaml', { ...moveView, review });
    const occupied = vscode.workspace.onWillRenameFiles(event => {
        if (event.files.some(file => file.newUri.toString() === renamedView.toString())) {
            event.waitUntil(write(renamedView, 'Created while the move was starting'));
        }
    });
    try {
        assert.ok((await moveReviewedView((await planView()).review)).isError);
        assert.equal(doc.getText(), referenced, 'A collision restores the rebased draft at the original path');
        assert.equal(await disk(renamedView), 'Created while the move was starting');
        assert.equal(await snapshot(), before);
    } finally { occupied.dispose(); }
    await vscode.workspace.fs.delete(renamedView, { useTrash: false });
    content<FileMovePlan>(await moveReviewedView((await planView()).review));
    const titled = (file: string) => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === `Design · ${file}`));
    await until(() => titled('Customers (Moved).xaml'));
    assert.equal(await snapshot(), before);
    const movedViewer = await vscode.workspace.openTextDocument(renamedView);
    assert.match(movedViewer.getText(), /ResourceInclude Source="\.\.\/Palette\.xaml"/);
    await vscode.window.showTextDocument(movedViewer);
    await vscode.commands.executeCommand('undo');
    await until(async () => await missing(renamedView) && titled('Customers.xaml'));
    assert.equal(await snapshot(), before);
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(view));
    await vscode.commands.executeCommand('redo');
    await until(async () => await missing(view) && titled('Customers (Moved).xaml'));
    assert.equal(await snapshot(), before);
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(renamedView));
    await vscode.commands.executeCommand('undo');
    await until(async () => await missing(renamedView) && titled('Customers.xaml'));
    console.log('Reviewed XAML file move updated closed and open consumers, retained the native preview, and undid the move with its patches');
}
