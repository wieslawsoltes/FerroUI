import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { ProjectDefinition } from '../src/projectLibrary';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyProjectLibrary({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const path = 'Review Library.xaml';
    const uri = vscode.Uri.joinPath(root, path);
    const source = [
        '<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">',
        '  <!-- 🌱 --><DataTemplate x:Key="LibraryProjectCard"><TextBlock Text="Card"/></DataTemplate>',
        '</ResourceDictionary>',
    ].join('\r\n');
    const list = async (query: string) => content<{ entries: ProjectDefinition[]; total: number }>(
        await invoke('ferroui_project_library', { folder: root.toString(), query, limit: 1 }));
    const edit = async (document: vscode.TextDocument, replacement: string) => {
        const change = new vscode.WorkspaceEdit();
        change.replace(uri, new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length)), replacement);
        assert.ok(await vscode.workspace.applyEdit(change));
    };
    const close = async () => {
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.label === `Design · ${path}` || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
        if (tabs.length) assert.ok(await vscode.window.tabGroups.close(tabs));
    };
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    try {
        await until(async () => (await list('libraryprojectcard review')).total === 1);
        assert.ok(!vscode.workspace.textDocuments.some(document => document.uri.toString() === uri.toString()));
        const [entry] = (await list('libraryprojectcard review')).entries;
        assert.deepEqual(entry.definition.range.start, { line: 1, character: 14 });
        assert.equal(entry.path, path);
        const opened = content<{ session: number; revision: number }>(await invoke('designer_open_project_definition', { entry }));
        assert.ok(Number.isSafeInteger(opened.revision));
        const { scope } = content<{ scope: { definition: { title: string } } }>(await invoke('designer_studio_query', {
            session: opened.session, tool: 'designer_definition_scope', arguments: {},
        }));
        assert.match(scope.definition.title, /LibraryProjectCard/);
        const document = await vscode.workspace.openTextDocument(uri);
        assert.equal(document.getText(), source);
        await edit(document, '<incomplete');
        await until(async () => (await list('libraryprojectcard')).total === 0);
        assert.ok((await invoke('designer_open_project_definition', { entry })).isError);
        await edit(document, source.replace('LibraryProjectCard', 'LibraryDraftCard'));
        await until(async () => (await list('librarydraftcard')).total === 1);
        const [draft] = (await list('librarydraftcard')).entries;
        content(await invoke('designer_open_project_definition', { entry: draft, destination: 'source' }));
        const editor = vscode.window.activeTextEditor!;
        assert.equal(editor.document, document);
        assert.equal(document.getText(editor.selection), 'DataTemplate');
        assert.ok(document.isDirty);
        await vscode.commands.executeCommand('workbench.action.files.revert');
        await until(() => !document.isDirty);
        await close();
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source.replace('LibraryProjectCard', 'LibraryDiskCard')));
        await until(async () => (await list('librarydiskcard')).total === 1);
    } finally {
        const editor = vscode.window.activeTextEditor;
        if (editor?.document.uri.toString() === uri.toString() && editor.document.isDirty) {
            await vscode.commands.executeCommand('workbench.action.files.revert');
        }
        await close();
        await vscode.workspace.fs.delete(uri, { useTrash: false });
    }
    console.log('Project Library discovered an unopened file, opened its native scope, rejected an invalid draft, and refreshed the disk index');
}
