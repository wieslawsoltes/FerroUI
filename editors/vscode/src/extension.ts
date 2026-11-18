import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { DesignerWorkbench } from './designerWorkbench';
import { HotReload } from './hotReload';
import { HostedDesign } from './hostedDesign';
import { FerroMcpProvider } from './mcpProvider';
import { ProjectSchemaBuild } from './projectSchemaBuild';
import { RuntimeConnections } from './runtimeConnections';
import { RuntimePanel } from './runtimePanel';
import { RsxLanguage } from './rsxLanguage';
import { StagesPanel } from './stagesPanel';
import { xamlLanguage, XamlServer } from './xamlServer';

let stopProcesses: (() => Promise<void>) | undefined;

export function activate(context: vscode.ExtensionContext) {
    const languageOutput = vscode.window.createOutputChannel('FerroUI XAML');
    const previewOutput = vscode.window.createOutputChannel('FerroUI Designer');
    const server = new XamlServer(context, languageOutput);
    const schemaBuild = new ProjectSchemaBuild(languageOutput, metadata => server.updateProject(metadata));
    const rsx = new RsxLanguage(context, languageOutput);
    const designer = new DesignerWorkbench(context, previewOutput);
    const hosted = new HostedDesign(context, previewOutput, metadata => server.updateProject(metadata));
    const stages = new StagesPanel(context, previewOutput);
    const reload = new HotReload();
    stopProcesses = async () => { await Promise.all([reload.stop(), hosted.dispose(), schemaBuild.dispose()]); };
    const runtimes = new RuntimeConnections(reload);
    const liveApp = new RuntimePanel(context, runtimes, reload, designer);
    const mcp = new FerroMcpProvider(context, new AgentTools(designer, reload, stages), previewOutput, runtimes);
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        schemaBuild, server, rsx, designer, hosted, stages, liveApp, reload, runtimes, mcp, languageOutput, previewOutput,
        reload.onDidBuild(metadata => server.updateProject(metadata)),
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

export async function deactivate(): Promise<void> {
    await stopProcesses?.();
    stopProcesses = undefined;
}
