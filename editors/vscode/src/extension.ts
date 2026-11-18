import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { DesignerWorkbench } from './designerWorkbench';
import { HotReload } from './hotReload';
import { FerroMcpProvider } from './mcpProvider';
import { xamlLanguage, XamlServer } from './xamlServer';

export function activate(context: vscode.ExtensionContext) {
    const languageOutput = vscode.window.createOutputChannel('FerroUI XAML');
    const previewOutput = vscode.window.createOutputChannel('FerroUI Designer');
    const server = new XamlServer(context, languageOutput);
    const designer = new DesignerWorkbench(context, previewOutput);
    const reload = new HotReload();
    const mcp = new FerroMcpProvider(context, new AgentTools(designer, reload), previewOutput);
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        server, designer, reload, mcp, languageOutput, previewOutput,
        vscode.workspace.onDidOpenTextDocument(startForXaml),
        vscode.workspace.onDidChangeConfiguration(event => {
            if ((event.affectsConfiguration('ferroui.languageServer.path') || event.affectsConfiguration('ferroui.types'))
                && vscode.workspace.textDocuments.some(document => document.languageId === xamlLanguage)) void server.start(true);
        }),
        vscode.commands.registerCommand('ferroui.restartLanguageServer', () => server.start(true)),
    );
    vscode.workspace.textDocuments.forEach(startForXaml);
    if (context.extensionMode === vscode.ExtensionMode.Test) {
        context.subscriptions.push(vscode.commands.registerCommand('ferroui.test.mcpDefinitions', () => mcp.provideMcpServerDefinitions()));
        return { designer };
    }
}
