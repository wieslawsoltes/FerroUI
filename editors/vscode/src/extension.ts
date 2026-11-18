import * as path from 'node:path';
import * as vscode from 'vscode';
import { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { languageOf } from './textDocument';
import { xamlLanguage, XamlServer } from './xamlServer';

export function activate(context: vscode.ExtensionContext): void {
    const languageOutput = vscode.window.createOutputChannel('FerroUI XAML');
    const previewOutput = vscode.window.createOutputChannel('FerroUI Designer');
    const server = new XamlServer(context, languageOutput);
    const panels = new Map<string, DesignerPanel>();
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        server, languageOutput, previewOutput,
        vscode.workspace.onDidOpenTextDocument(startForXaml),
        vscode.workspace.onDidChangeConfiguration(event => {
            if ((event.affectsConfiguration('ferroui.languageServer.path') || event.affectsConfiguration('ferroui.types'))
                && vscode.workspace.textDocuments.some(document => document.languageId === xamlLanguage)) void server.start(true);
        }),
        vscode.commands.registerCommand('ferroui.restartLanguageServer', () => server.start(true)),
        vscode.commands.registerCommand('ferroui.openDesigner', async (uri?: vscode.Uri) => {
            try {
                const active = vscode.window.activeTextEditor;
                const document = uri ? await vscode.workspace.openTextDocument(uri) : active?.document;
                const language = document && languageOf(document);
                if (!document || !language) throw new Error('Open a local FerroUI XAML or Rust file.');
                const sourceEditor = active?.document === document ? active : await vscode.window.showTextDocument(document);
                const key = document.uri.toString();
                const previous = panels.get(key);
                if (previous) {
                    previous.show(sourceEditor.selection.active);
                    return;
                }
                const panel = vscode.window.createWebviewPanel('ferroui.designer', `Design · ${path.basename(document.fileName)}`, {
                    viewColumn: vscode.ViewColumn.Beside, preserveFocus: true,
                }, {
                    enableScripts: true,
                    retainContextWhenHidden: true,
                    localResourceRoots: ['dist', 'media'].map(folder => vscode.Uri.joinPath(context.extensionUri, folder)),
                });
                const designer = new DesignerPanel(context, previewOutput, panel, document, language, sourceEditor.selection.active, () => panels.delete(key));
                panels.set(key, designer);
            } catch (error) {
                void vscode.window.showErrorMessage(errorMessage(error));
            }
        }),
        { dispose: () => { for (const panel of panels.values()) panel.dispose(); } },
    );
    vscode.workspace.textDocuments.forEach(startForXaml);
}
