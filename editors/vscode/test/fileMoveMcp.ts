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
    await verifyRustReferences({ invoke, content });
}

async function verifyRustReferences({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Rust XAML move');
    const uri = (file: string) => vscode.Uri.joinPath(root, file);
    const write = (file: string, source: string) => vscode.workspace.fs.writeFile(uri(file), Buffer.from(source));
    const originalPath = 'Views/Palette.xaml';
    const movedPath = 'Views/Archive/Customer palette.xaml';
    const build = `use ferroui_build::{Build, XamlGroup};\nfn main() {\n    println!("cargo:rerun-if-changed=${originalPath}");\n    Build::from_env().input("${originalPath}").compile_group(XamlGroup::new("compiled_xaml").documents(&[("${originalPath}", include_str!("${originalPath}"))])).run();\n}\n`;
    const disk = 'pub const TEXT: &str = "Views/Palette.xaml";\r\n';
    const draft = `${disk}pub const SOURCE: &str = { /* 🎨 */ include_str!(r#"${originalPath}"#) };\r\n`;
    await vscode.workspace.fs.createDirectory(uri('Views/Archive'));
    await Promise.all([
        write('Cargo.toml', '[package]\nname="rust-xaml-move"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n'),
        write('build.rs', build), write('lib.rs', disk), write(originalPath, '<ResourceDictionary xmlns="https://github.com/ferroui" />'),
    ]);
    const doc = await vscode.workspace.openTextDocument(uri('lib.rs'));
    const editor = await vscode.window.showTextDocument(doc);
    await editor.edit(edit => edit.insert(doc.positionAt(doc.getText().length), draft.slice(disk.length)));
    const path = vscode.workspace.asRelativePath(uri(originalPath), false).replaceAll('\\', '/');
    const destination = vscode.workspace.asRelativePath(uri(movedPath), false).replaceAll('\\', '/');
    const plan = () => invoke('ferroui_workspace_plan_xaml_move', { path, destination });
    const before = content<FileMovePlan>(await plan());
    assert.equal(before.documents.length, 3);
    const buildEdit = before.documents.find(entry => entry.edit.uri.endsWith('/build.rs'))!;
    assert.equal(buildEdit.after, build.replaceAll(originalPath, movedPath));
    const rustEdit = before.documents.find(entry => entry.edit.uri.endsWith('/lib.rs'))!;
    assert.equal(rustEdit.edit.version, doc.version);
    assert.equal(rustEdit.before, draft);
    assert.equal(rustEdit.after, draft.replace(`r#"${originalPath}"#`, `r#"${movedPath}"#`));
    await editor.edit(edit => edit.insert(doc.positionAt(doc.getText().length), '// Keep this unsaved line too.\r\n'));
    const apply = (review: string) => invoke('ferroui_workspace_move_xaml', { path, destination, review });
    assert.ok((await apply(before.review)).isError);
    const editedDraft = doc.getText();
    content<FileMovePlan>(await apply(content<FileMovePlan>(await plan()).review));
    const buildDoc = await vscode.workspace.openTextDocument(uri('build.rs'));
    assert.equal(buildDoc.getText(), buildEdit.after);
    assert.equal(doc.getText(), editedDraft.replace(`r#"${originalPath}"#`, `r#"${movedPath}"#`));
    const exists = (file: string) => vscode.workspace.fs.stat(uri(file)).then(() => true, () => false);
    assert.ok(await exists(movedPath));
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('undo');
    await until(async () => doc.getText() === editedDraft && buildDoc.getText() === build && await exists(originalPath));
    await vscode.commands.executeCommand('redo');
    await until(async () => doc.getText().includes(`r#"${movedPath}"#`) && buildDoc.getText() === buildEdit.after && await exists(movedPath));
    console.log('Reviewed XAML file move updated an unopened Cargo build and an unsaved Rust source, rejected stale drafts, and grouped source history');
}
