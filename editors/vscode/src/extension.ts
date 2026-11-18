import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { BuiltProjects } from './builtProjects';
import { DesignerWorkbench } from './designerWorkbench';
import { DiagnosticsView } from './diagnosticsView';
import { HotReload } from './hotReload';
import { HostedDesign } from './hostedDesign';
import { FerroMcpProvider } from './mcpProvider';
import { ProjectAssets } from './projectAssets';
import { ProjectInventory } from './projectInventory';
import { ProjectScaffold } from './projectScaffold';
import { ProjectSchemaBuild } from './projectSchemaBuild';
import { ProjectView } from './projectView';
import type { ProjectMetadata } from './reloadProcess';
import { RuntimeConnections } from './runtimeConnections';
import { RuntimePanel } from './runtimePanel';
import { RuntimeSourceEditor } from './runtimeSource';
import { RuntimeSourceTools } from './runtimeSourceTools';
import { RsxLanguage } from './rsxLanguage';
import { ScenarioRunner } from './scenarioRunner';
import { ScenarioTesting } from './scenarioTesting';
import { StagesPanel } from './stagesPanel';
import { xamlLanguage, XamlServer } from './xamlServer';

let stopProcesses: (() => Promise<void>) | undefined;

export function activate(context: vscode.ExtensionContext) {
    const languageOutput = vscode.window.createOutputChannel('FerroUI XAML');
    const previewOutput = vscode.window.createOutputChannel('FerroUI Designer');
    const server = new XamlServer(context, languageOutput);
    const projects = new BuiltProjects(context.workspaceState, languageOutput);
    const publish = (metadata: ProjectMetadata) => { projects.update(metadata); server.updateProject(metadata); };
    projects.metadata.forEach(metadata => server.updateProject(metadata));
    const schemaBuild = new ProjectSchemaBuild(languageOutput, publish);
    const rsx = new RsxLanguage(context, languageOutput, projects);
    const scenarios = new ScenarioRunner(context);
    const scenarioTesting = new ScenarioTesting(scenarios);
    const assets = new ProjectAssets();
    const inventory = new ProjectInventory(context, previewOutput, assets);
    const scaffolds = new ProjectScaffold();
    const designer = new DesignerWorkbench(context, previewOutput, projects, assets);
    const hosted = new HostedDesign(context, previewOutput, publish);
    const stages = new StagesPanel(context, previewOutput, projects);
    const reload = new HotReload();
    const projectView = new ProjectView(inventory, projects, reload);
    stopProcesses = async () => { await Promise.all([reload.stop(), hosted.dispose(), schemaBuild.dispose()]); };
    const runtimes = new RuntimeConnections(reload);
    const runtimeSource = new RuntimeSourceEditor(designer, reload);
    const liveApp = new RuntimePanel(context, runtimes, reload, runtimeSource);
    const diagnostics = new DiagnosticsView(reload.history, runtimes, runtimeSource);
    const runtimeTools = new RuntimeSourceTools(runtimes, runtimeSource);
    const mcp = new FerroMcpProvider(context, new AgentTools(designer, reload, stages, runtimeTools, inventory, scaffolds, scenarios), previewOutput, runtimes);
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        projectView, diagnostics, projects, schemaBuild, inventory, scaffolds, assets, server, rsx, scenarioTesting, designer, hosted, stages, liveApp, reload, runtimes, mcp, languageOutput, previewOutput,
        reload.onDidBuild(publish),
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
