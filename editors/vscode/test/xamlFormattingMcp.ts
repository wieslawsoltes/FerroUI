import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';
import { until } from './wait';

const path = 'FormattingReview.xaml';
const source = [
    '<Grid>',
    "<Button Content='Review 🧭' Width = '80'/>",
    '<TextBlock><Run Text="A"/><Run Text="B"/></TextBlock>',
    '<!-- @formatter:off --><Border  Padding="4"><Button/></Border><!-- @formatter:on -->',
    '',
    '</Grid>',
    '',
].join('\r\n');

export async function verifyXamlFormatting({ invoke, content }: McpFixture): Promise<void> {
    const [primary, secondary] = vscode.workspace.workspaceFolders!;
    const target = vscode.workspace.workspaceFile ? vscode.ConfigurationTarget.WorkspaceFolder : vscode.ConfigurationTarget.Workspace;
    const documents: vscode.TextDocument[] = [];
    const restore: (() => Thenable<void>)[] = [];
    try {
        for (const [folder, layout] of [[primary, 'onePerLine'], [secondary, 'inline']] as const) {
            if (!folder) continue;
            const config = vscode.workspace.getConfiguration('ferroui.xaml.formatting', folder.uri);
            const before = config.inspect<string>('attributeLayout');
            const previous = target === vscode.ConfigurationTarget.WorkspaceFolder
                ? before?.workspaceFolderValue : before?.workspaceValue;
            restore.push(() => config.update('attributeLayout', previous, target));
            await config.update('attributeLayout', layout, target);
            const uri = vscode.Uri.joinPath(folder.uri, path);
            await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
            const document = await vscode.workspace.openTextDocument(uri);
            documents.push(document);
            let edits: vscode.TextEdit[] | undefined;
            await until(async () => {
                edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
                    'vscode.executeFormatDocumentProvider', uri, { tabSize: 2, insertSpaces: true });
                return Boolean(edits?.length);
            });
            const formatted = applyToOriginal(document, edits!);
            const button = layout === 'onePerLine'
                ? "  <Button\r\n    Content='Review 🧭'\r\n    Width = '80'/>"
                : "  <Button Content='Review 🧭' Width = '80'/>";
            assert.ok(formatted.includes(button));
            assert.ok(formatted.includes('<Run Text="A"/><Run Text="B"/>'));
            assert.ok(formatted.includes('<!-- @formatter:off --><Border  Padding="4"><Button/></Border><!-- @formatter:on -->'));
            assert.equal(formatted.replaceAll('\r\n', '').includes('\n'), false);
        }
        const document = documents[0];
        const location = { path, folder: primary.uri.toString() };
        const uri = document.uri;
        const start = source.indexOf('<Button');
        const end = source.indexOf('/>') + 2;
        const selection = new vscode.Range(document.positionAt(start), document.positionAt(end));
        const range = { start: { line: selection.start.line, character: selection.start.character },
            end: { line: selection.end.line, character: selection.end.character } };
        const standard = await vscode.commands.executeCommand<vscode.TextEdit[]>(
            'vscode.executeFormatRangeProvider', uri, selection, { tabSize: 2, insertSpaces: true });
        assert.equal(applyToOriginal(document, standard),
            source.replace(" Content='Review 🧭' Width = '80'", "\r\n    Content='Review 🧭'\r\n    Width = '80'"));
        const status = content<{ revision: string }>(await invoke('ferroui_workspace_read', location));
        const request = { ...location, action: 'format', expected_revision: status.revision, range,
            options: { attributeLayout: 'onePerLine', tabSize: 2, insertSpaces: true, closingBracketOnNewLine: true } };
        const edited = content<{ revision: string }>(await invoke('ferroui_workspace_file', request));
        assert.equal(document.getText(), source.replace(" Content='Review 🧭' Width = '80'", "\r\n    Content='Review 🧭'\r\n    Width = '80'\r\n  "));
        assert.equal((await invoke('ferroui_workspace_file', request)).isError, true);
        content(await invoke('ferroui_workspace_file', { ...location, action: 'undo', expected_revision: edited.revision }));
        assert.equal(document.getText(), source);
        console.log(`FerroUI XAML document and selection formatting passed with ${documents.length} workspace folder setting(s), protected content and one MCP Undo`);
    } finally {
        for (const reset of restore.reverse()) await reset();
        const uris = new Set(documents.map(document => document.uri.toString()));
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.input instanceof vscode.TabInputText && uris.has(tab.input.uri.toString()));
        await vscode.window.tabGroups.close(tabs);
        for (const document of documents) await vscode.workspace.fs.delete(document.uri, { useTrash: false });
    }
}

function applyToOriginal(document: vscode.TextDocument, edits: readonly vscode.TextEdit[]): string {
    let output = document.getText();
    for (const edit of [...edits].sort((a, b) => b.range.start.compareTo(a.range.start))) {
        output = output.slice(0, document.offsetAt(edit.range.start)) + edit.newText + output.slice(document.offsetAt(edit.range.end));
    }
    return output;
}
