import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { FileMoves } from './fileMoves';
import { BuiltProjects } from './builtProjects';
import { DesignTokenCommands } from './designTokenCommands';
import { DesignerWorkbench } from './designerWorkbench';
import { DictionaryExtraction } from './dictionaryExtraction';
import type { DictionaryRequest } from './dictionaryProtocol';
import { DiagnosticsView } from './diagnosticsView';
import { EventHandlers } from './eventHandlers';
import type { HandlerRequest } from './eventHandlerProtocol';
import { HotReload } from './hotReload';
import { HostedDesign } from './hostedDesign';
import { FerroMcpProvider } from './mcpProvider';
import { ProjectAssets } from './projectAssets';
import { ProjectInventory } from './projectInventory';
import { ProjectScaffold } from './projectScaffold';
import { ProjectSchemaBuild } from './projectSchemaBuild';
import { ProjectView } from './projectView';
import { errorMessage } from './protocol';
import { RegisteredRename } from './registeredRename';
import type { ProjectMetadata } from './reloadProcess';
import { ResourceRename, type ResourceRenameRequest } from './resourceRename';
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
    const designTokens = new DesignTokenCommands(inventory);
    const scaffolds = new ProjectScaffold();
    const handlers = new EventHandlers(server);
    const extraction = new DictionaryExtraction(server);
    const resourceRename = new ResourceRename(server);
    server.eventDefinition = (document, position, token) => projects.forDocument(document.uri)
        ? handlers.definitionAt(document, position, token) : Promise.resolve(undefined);
    const designer = new DesignerWorkbench(context, previewOutput, projects, assets, handlers, extraction, resourceRename);
    const fileMoves = new FileMoves(server, from => designer.cancelFileMove(from));
    const propertyRename = new RegisteredRename(server, 'property');
    const typeRename = new RegisteredRename(server, 'type');
    const hosted = new HostedDesign(context, previewOutput, publish);
    const stages = new StagesPanel(context, previewOutput, projects);
    const reload = new HotReload();
    const projectView = new ProjectView(inventory, projects, reload);
    stopProcesses = async () => { await Promise.all([reload.stop(), hosted.dispose(), schemaBuild.dispose()]); };
    const runtimes = new RuntimeConnections(reload);
    const runtimeSource = new RuntimeSourceEditor(designer, reload);
    const liveApp = new RuntimePanel(context, runtimes, reload, runtimeSource);
    const diagnostics = new DiagnosticsView(reload.history, runtimes, runtimeSource, designer);
    const runtimeTools = new RuntimeSourceTools(runtimes, runtimeSource);
    const mcp = new FerroMcpProvider(context, new AgentTools(designer, reload, stages, runtimeTools, inventory, scaffolds, scenarios, hosted, fileMoves, {
        property: propertyRename, type: typeRename, resource: resourceRename,
    }), previewOutput, runtimes);
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        projectView, diagnostics, projects, schemaBuild, inventory, designTokens, scaffolds, assets, server, rsx, scenarioTesting, designer, extraction, fileMoves, propertyRename, typeRename, resourceRename, hosted, stages, liveApp, reload, runtimes, mcp, languageOutput, previewOutput,
        reload.onDidBuild(publish),
        vscode.workspace.onDidOpenTextDocument(startForXaml),
        vscode.workspace.onDidChangeConfiguration(event => {
            if ((event.affectsConfiguration('ferroui.languageServer.path') || event.affectsConfiguration('ferroui.types'))
                && vscode.workspace.textDocuments.some(document => document.languageId === xamlLanguage)) void server.start(true);
        }),
        vscode.commands.registerCommand('ferroui.restartLanguageServer', () => server.start(true)),
        vscode.commands.registerCommand('ferroui.moveXamlFile', async (uri?: vscode.Uri) => {
            try { await fileMoves.open(uri ?? vscode.window.activeTextEditor?.document.uri); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.commands.registerCommand('ferroui.renameRegisteredProperty', async () => {
            try { await propertyRename.open(vscode.window.activeTextEditor); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.commands.registerCommand('ferroui.renameRegisteredType', async () => {
            try { await typeRename.open(vscode.window.activeTextEditor); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.commands.registerCommand('ferroui.renameResource', async (action?: { uri: string; request: ResourceRenameRequest }) => {
            try {
                if (action) {
                    await resourceRename.open({ document: await vscode.workspace.openTextDocument(vscode.Uri.parse(action.uri)), request: action.request });
                } else if (vscode.window.activeTextEditor) {
                    const { document, selection } = vscode.window.activeTextEditor;
                    await resourceRename.open({ document, request: { version: document.version, position: selection.active } });
                }
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.workspace.onDidGrantWorkspaceTrust(() => { if (vscode.workspace.textDocuments.some(document => document.languageId === xamlLanguage)) void server.start(true); }),
        vscode.commands.registerCommand('ferroui.extractResourceDictionary', async (action?: { uri: string; request: DictionaryRequest }) => {
            try {
                if (action) {
                    await extraction.openReview({ document: await vscode.workspace.openTextDocument(vscode.Uri.parse(action.uri)), request: action.request });
                } else if (vscode.window.activeTextEditor) {
                    const { document, selection } = vscode.window.activeTextEditor;
                    await extraction.openReview({ document, request: { version: document.version, position: selection.active } });
                }
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.commands.registerCommand('ferroui.createEventHandler', async (action?: { uri: string; request: HandlerRequest }) => {
            try {
                if (action) {
                    await handlers.create({ document: await vscode.workspace.openTextDocument(vscode.Uri.parse(action.uri)), request: action.request });
                } else if (vscode.window.activeTextEditor) {
                    const { document, selection } = vscode.window.activeTextEditor;
                    await handlers.create({ document, request: { version: document.version, position: selection.active } });
                }
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
    );
    vscode.workspace.textDocuments.forEach(startForXaml);
    if (context.extensionMode === vscode.ExtensionMode.Test) {
        context.subscriptions.push(vscode.commands.registerCommand('ferroui.test.mcpDefinitions', () => mcp.provideMcpServerDefinitions()));
        return { designer, runtimeSource, fileMoves, propertyRename, typeRename, resourceRename, publishProjectMetadata: publish };
    }
}

export async function deactivate(): Promise<void> {
    await stopProcesses?.();
    stopProcesses = undefined;
}
