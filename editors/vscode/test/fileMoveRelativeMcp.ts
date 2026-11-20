import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { ProjectMetadata } from '../src/reloadProcess';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRelativeCompiledMove({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'Relative compiled move');
    const uri = (file: string) => vscode.Uri.joinPath(root, file);
    const write = (file: string, value: string) => vscode.workspace.fs.writeFile(uri(file), Buffer.from(value));
    const relative = (file: string) => vscode.workspace.asRelativePath(uri(file), false).replaceAll('\\', '/');
    const manifest = uri('Cargo.toml');
    const directory = uri('target/schema');
    const view = 'Views/Customer card.xaml';
    const from = 'Palettes/Actual.xaml';
    const to = 'Palettes/Archive/New colors.xaml';
    const color = '#176D8C';
    const group = 'views';
    const members = [{ key: 'Published/Card.xaml', path: view }, { key: 'Published/Colors.xaml', path: from }];
    const script = [
        'use ferroui_build::{Build, XamlGroup, xaml_document_file};', 'fn main() {',
        ...members.map(({ key, path }, index) => `    let input${index} = xaml_document_file!("${key}", "${path}");`),
        `    Build::from_env().compile_group(XamlGroup::new("${group}").document_files(&[input0, input1])).run();`,
        '}', '',
    ];
    const ns = 'xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"';
    const markup = `<Border ${ns} Background="{StaticResource Accent}"><Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="Colors.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources><TextBlock Text="Customer card" Foreground="White"/></Border>`;
    await Promise.all(['Views', 'Palettes/Archive', 'target/schema'].map(folder => vscode.workspace.fs.createDirectory(uri(folder))));
    await Promise.all([
        write('Cargo.toml', '[package]\nname="relative-compiled-move"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n'),
        write('build.rs', script.join('\n')), write('lib.rs', ''), write(view, markup),
        write(from, `<ResourceDictionary ${ns}><SolidColorBrush x:Key="Accent" Color="${color}"/></ResourceDictionary>`),
        write('target/schema/relative_compiled_move.xamlmeta', JSON.stringify({
            name: 'Relative.Compiled', crate_name: 'relative_compiled_move', documents: [], dependencies: [],
        })),
        write('target/schema/xaml-symbols.json', JSON.stringify({
            version: 1, manifest: manifest.fsPath, assembly: 'Relative.Compiled',
            groups: [{ module: group, kind: 'documents', discovery_root: null, file_backed: true,
                members: members.map(({ key, path }) => ({ key, source_path: path, symbol: null, public: false })) }],
            file_inputs: members.map(({ path }, index) => ({
                rust_file: 'build.rs', path, line: 3 + index, column: script[2 + index].indexOf('xaml_document_file!') + 1,
            })),
        })),
    ]);
    const extension = vscode.extensions.getExtension<{ publishProjectMetadata(metadata: ProjectMetadata): void }>('FerroUI.ferroui')!;
    (await extension.activate()).publishProjectMetadata({ manifest: manifest.fsPath, directories: [directory.fsPath], package: 'relative-compiled-move' });
    const doc = await vscode.workspace.openTextDocument(uri(view));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: relative(view) }));
    const image = async () => {
        const reply = await invoke('designer_preview_image', { session });
        return reply.content.find(item => item.type === 'image')?.data;
    };
    const before = await image();
    assert.ok(before, 'The relative include resolves through the compiled key from a different physical folder');
    const args = { path: relative(from), destination: relative(to) };
    const plan = content<FileMovePlan>(await invoke('ferroui_workspace_plan_xaml_move', args));
    assert.equal(plan.documents.length, 2);
    content(await invoke('ferroui_workspace_move_xaml', { ...args, review: plan.review }));
    const expectSource = async (file: string) => {
        const at = doc.positionAt(doc.getText().indexOf('Accent'));
        await until(async () => (await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeDefinitionProvider', doc.uri, at,
        ))?.some(location => location.uri.fsPath === uri(file).fsPath));
        assert.equal(await image(), before);
        assert.equal(doc.getText(), markup);
    };
    await expectSource(to);
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri('build.rs')));
    await vscode.commands.executeCommand('undo');
    await expectSource(from);
    await vscode.commands.executeCommand('redo');
    await expectSource(to);

    const palette = await vscode.workspace.openTextDocument(uri(to));
    const start = palette.positionAt(palette.getText().indexOf(color));
    const edit = new vscode.WorkspaceEdit();
    edit.replace(palette.uri, new vscode.Range(start, start.translate(0, color.length)), '#99463A');
    assert.ok(await vscode.workspace.applyEdit(edit));
    await until(async () => { const frame = await image(); return !!frame && frame !== before; });
    console.log('Compiled relative includes followed Apply, Undo, Redo, and an unsaved edit at the new path');
}
