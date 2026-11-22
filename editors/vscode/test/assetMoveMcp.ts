import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { McpFixture } from './mcp';
import { verifyPublishedAssetMove } from './publishedAssetMoveMcp';
import { until } from './wait';

export async function verifyAssetMove({ invoke, content }: McpFixture): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const folder = vscode.Uri.joinPath(workspace, 'AssetMoveFixture');
    const from = vscode.Uri.joinPath(folder, 'Logo Ł.png');
    const to = vscode.Uri.joinPath(folder, 'Assets', 'Moved #Logo.png');
    const xamlUri = vscode.Uri.joinPath(folder, 'Consumer.xaml');
    const rustUri = vscode.Uri.joinPath(folder, 'consumer.rs');
    const source = '<StackPanel xmlns="https://github.com/ferroui"><Image Source="./Logo%20%C5%81.png"/><TextBlock Text="Keep Logo Ł.png"/></StackPanel>';
    const rustSource = 'fn view() { rsx! { Image { source: ::ferroui_view::embedded_image(::core::include_bytes!(concat!("./", /* original */ r#"Logo Ł"#, ".png"))) } }; }\r\n';
    const bytes = Buffer.from([0, 18, 29, 255]);
    const request = { path: 'AssetMoveFixture/Logo Ł.png', destination: 'AssetMoveFixture/Assets/Moved #Logo.png', folder: workspace.toString() };
    const plan = async () => content<FileMovePlan>(await invoke('ferroui_project_plan_asset_move', request));
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(folder, 'Assets'));
    await vscode.workspace.fs.writeFile(from, bytes);
    await vscode.workspace.fs.writeFile(xamlUri, Buffer.from(source));
    await vscode.workspace.fs.writeFile(rustUri, Buffer.from(rustSource));
    const rust = await vscode.workspace.openTextDocument(rustUri);
    const xaml = await vscode.workspace.openTextDocument(xamlUri);
    try {
        await until(async () => (await plan()).documents.length === 2);
        const stale = await plan();
        const version = rust.version;
        const draft = rustSource.replace('/* original */', '/* draft 🌱 */');
        const change = new vscode.WorkspaceEdit();
        change.replace(rustUri, new vscode.Range(rust.positionAt(0), rust.positionAt(rust.getText().length)), draft);
        assert.ok(await vscode.workspace.applyEdit(change));
        assert.notEqual(rust.version, version);
        assert.match((await invoke('ferroui_project_move_asset', { ...request, review: stale.review })).content[0]?.text ?? '', /move changed/);
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(from)), bytes);

        const current = await plan();
        assert.equal(current.documents.find(document => document.edit.uri.endsWith('/consumer.rs'))?.edit.version, rust.version);
        content(await invoke('ferroui_project_move_asset', { ...request, review: current.review }));
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(to)), bytes);
        await assert.rejects(async () => vscode.workspace.fs.stat(from));
        assert.equal(xaml.getText(), source.replace('./Logo%20%C5%81.png', './Assets/Moved%20%23Logo.png'));
        assert.equal(rust.getText(), draft.replace('r#"Logo Ł"#', 'r#"Assets/Moved #Logo"#'));

        await vscode.window.showTextDocument(xaml, { preview: false });
        await vscode.commands.executeCommand('undo');
        await until(async () => (await vscode.workspace.fs.stat(from).then(() => true, () => false)) && xaml.getText() === source && rust.getText() === draft);
        await assert.rejects(async () => vscode.workspace.fs.stat(to));
    } finally {
        for (const document of [rust, xaml]) {
            if (document.isDirty) {
                await vscode.window.showTextDocument(document, { preview: false });
                await vscode.commands.executeCommand('workbench.action.files.revert');
                await until(() => !document.isDirty);
            }
        }
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.input instanceof vscode.TabInputText
            && [rustUri.toString(), xamlUri.toString()].includes(tab.input.uri.toString()));
        if (tabs.length) await vscode.window.tabGroups.close(tabs);
        await vscode.workspace.fs.delete(folder, { recursive: true, useTrash: false });
    }
    console.log('Reviewed asset move updated XAML and Rust drafts and restored the binary and both buffers with one Undo');
    await verifyPublishedAssetMove({ invoke, content });
}
