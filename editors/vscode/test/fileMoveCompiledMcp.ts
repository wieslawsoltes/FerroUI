import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyCompiledFileMove({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Published XAML move');
    const uri = (path: string) => vscode.Uri.joinPath(root, path);
    const write = (path: string, source: string) => vscode.workspace.fs.writeFile(uri(path), Buffer.from(source));
    const relative = (path: string) => vscode.workspace.asRelativePath(uri(path), false).replaceAll('\\', '/');
    const exists = (path: string) => vscode.workspace.fs.stat(uri(path)).then(() => true, () => false);
    const original = 'Views/Palette source.xaml';
    const relocated = 'Views/Archive/Customer palette.xaml';
    const alias = 'Palette.xaml';
    const resource = `ferres://published_xaml_move/${alias}`;
    const rust = `pub const PALETTE: &str = "${resource}";\r\n`;
    const ns = 'xmlns="https://github.com/ferroui"';
    const consumer = `<ResourceDictionary ${ns}><ResourceDictionary.MergedDictionaries><ResourceInclude Source="${resource}" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>`;
    const script = [
        'use ferroui_build::{Build, XamlGroup};',
        'fn main() {',
        `    Build::from_env().compile_group(XamlGroup::new("published").documents(&[("${alias}", include_str!("${original}"))])).run();`,
        '}', '',
    ].join('\r\n');
    await vscode.workspace.fs.createDirectory(uri('Views/Archive'));
    await Promise.all([
        write('Cargo.toml', '[package]\nname="published-xaml-move"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n'),
        write('build.rs', script), write('lib.rs', rust), write('Consumer.xaml', consumer),
        write(original, `<ResourceDictionary ${ns} />`), write(alias, `<ResourceDictionary ${ns} />`),
    ]);
    const request = (from: string, to: string) => ({ path: relative(from), destination: relative(to) });
    const plan = async (args: object) => content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', args));
    const unrelated = await plan(request(alias, 'Views/Archive/Unrelated.xaml'));
    assert.equal(unrelated.documents.length, 1, 'A physical filename can coincide with someone else’s published alias');
    const args = request(original, relocated);
    const review = await plan(args);
    assert.equal(review.documents.length, 2);
    const build = review.documents.find(document => document.edit.uri.endsWith('/build.rs'));
    assert.equal(build?.after, script.replace(original, relocated));
    content(await invoke('ferroui_workspace_move_xaml', { ...args, review: review.review }));
    const buffer = await vscode.workspace.openTextDocument(uri('build.rs'));
    assert.equal(buffer.getText(), script.replace(original, relocated));
    assert.ok(await exists(relocated));
    assert.equal((await vscode.workspace.openTextDocument(uri('Consumer.xaml'))).getText(), consumer);
    assert.equal((await vscode.workspace.openTextDocument(uri('lib.rs'))).getText(), rust);
    await vscode.window.showTextDocument(buffer);
    await vscode.commands.executeCommand('undo');
    await until(async () => buffer.getText() === script && await exists(original) && !await exists(relocated));
    await vscode.commands.executeCommand('redo');
    await until(async () => buffer.getText() === script.replace(original, relocated) && await exists(relocated));
    console.log('Reviewed compiled XAML move preserved its published alias and undid and redid the literal include');
}
