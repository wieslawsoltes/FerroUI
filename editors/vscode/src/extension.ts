import * as vscode from 'vscode';
import { AndroidSessions } from './androidSessions';
import { AppSessions } from './appSessions';
import { AssetMoves } from './assetMoves';
import { BrowserSessions } from './browserSessions';
import { CommandBindings } from './commandBindings';
import { AgentTools } from './agentTools';
import { FileMoves } from './fileMoves';
import { BuiltProjects } from './builtProjects';
import { Collaboration } from './collaboration';
import { DesignHandoff } from './designHandoff';
import { DesignTokenCommands } from './designTokenCommands';
import { DesignerWorkbench } from './designerWorkbench';
import { DictionaryExtraction } from './dictionaryExtraction';
import type { DictionaryRequest } from './dictionaryProtocol';
import { DiagnosticsView } from './diagnosticsView';
import { EventHandlers } from './eventHandlers';
import type { HandlerRequest } from './eventHandlerProtocol';
import { HotReload } from './hotReload';
import { HostedDesign } from './hostedDesign';
import { IosSimulatorSessions } from './iosSimulatorSessions';
import { FerroMcpProvider } from './mcpProvider';
import { ProjectAssets } from './projectAssets';
import { ProjectInventory } from './projectInventory';
import { ProjectLibrary } from './projectLibrary';
import { ProjectScaffold } from './projectScaffold';
import { ProjectSchemaBuild } from './projectSchemaBuild';
import { ProjectSetup } from './projectSetup';
import { ProjectView } from './projectView';
import { errorMessage } from './protocol';
import { RegisteredRename } from './registeredRename';
import type { ProjectMetadata } from './reloadProcess';
import { ResourceRename, type ResourceRenameRequest } from './resourceRename';
import { ResourceMove } from './resourceMove';
import { RuntimeConnections } from './runtimeConnections';
import { RuntimePanels } from './runtimePanels';
import { RuntimeSourceEditor } from './runtimeSource';
import { RuntimeSourceTools } from './runtimeSourceTools';
import { RuntimeProfiles } from './runtimeProfiles';
import { RsxLanguage } from './rsxLanguage';
import { ScenarioRunner } from './scenarioRunner';
import { ScenarioReview } from './scenarioReview';
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
    const scenarioReviews = new ScenarioReview(context, scenarios);
    const scenarioTesting = new ScenarioTesting(scenarios, scenarioReviews);
    const assets = new ProjectAssets();
    const assetMoves = new AssetMoves(server);
    const inventory = new ProjectInventory(context, previewOutput, assets, assetMoves);
    const designTokens = new DesignTokenCommands(context, inventory);
    const scaffolds = new ProjectScaffold();
    const projectSetup = new ProjectSetup(inventory);
    const handlers = new EventHandlers(server);
    const extraction = new DictionaryExtraction(server);
    const resourceRename = new ResourceRename(server);
    const resourceMove = new ResourceMove(server);
    server.eventDefinition = (document, position, token) => projects.forDocument(document.uri)
        ? handlers.definitionAt(document, position, token) : Promise.resolve(undefined);
    const designer = new DesignerWorkbench(context, previewOutput, projects, assets, handlers, extraction, resourceRename, resourceMove, new ProjectLibrary(server), new CommandBindings(server));
    const designHandoff = new DesignHandoff(context, designer);
    const collaboration = new Collaboration(context, previewOutput,
        () => vscode.window.activeTextEditor?.document ?? designer.activeDesign?.document);
    const fileMoves = new FileMoves(server, from => designer.cancelFileMove(from));
    const propertyRename = new RegisteredRename(server, 'property');
    const typeRename = new RegisteredRename(server, 'type');
    const hosted = new HostedDesign(context, previewOutput, publish);
    const stages = new StagesPanel(context, previewOutput, projects);
    const reload = new HotReload();
    const projectView = new ProjectView(inventory, projects, reload, projectSetup);
    const runtimes = new RuntimeConnections(reload);
    const simulatorSessions = new IosSimulatorSessions(runtimes);
    const androidSessions = new AndroidSessions(runtimes);
    const browserSessions = new BrowserSessions(runtimes);
    stopProcesses = async () => {
        await Promise.all([reload.stop(), simulatorSessions.shutdown(), androidSessions.shutdown(), browserSessions.shutdown(),
            hosted.dispose(), schemaBuild.dispose(), scenarios.shutdown(), collaboration.shutdown()]);
    };
    const runtimeSource = new RuntimeSourceEditor(designer, reload);
    const liveApp = new RuntimePanels(context, runtimes, reload, runtimeSource);
    const appSessions = new AppSessions(reload, runtimes, liveApp, simulatorSessions, androidSessions, browserSessions);
    const runtimeProfiles = new RuntimeProfiles(runtimes, runtimeSource);
    const diagnostics = new DiagnosticsView(reload.history, runtimes, runtimeSource, designer, runtimeProfiles);
    const runtimeTools = new RuntimeSourceTools(runtimes, runtimeSource);
    const mcp = new FerroMcpProvider(context, new AgentTools(designer, reload, stages, runtimeTools, runtimeProfiles, appSessions, inventory, scaffolds, projectSetup, scenarios, scenarioReviews, hosted, fileMoves, collaboration, {
        property: propertyRename, type: typeRename, resource: resourceRename, resourceMove,
    }, designHandoff.actions), previewOutput, runtimes);
    const startForXaml = (document: vscode.TextDocument) => {
        if (document.languageId === xamlLanguage && document.uri.scheme === 'file') void server.start();
    };
    context.subscriptions.push(
        designHandoff,
        projectView, diagnostics, runtimeProfiles, projects, schemaBuild, inventory, designTokens, scaffolds, projectSetup, assets, assetMoves, server, rsx, scenarioTesting, scenarioReviews, scenarios, designer, collaboration, extraction, fileMoves, propertyRename, typeRename, resourceRename, resourceMove, hosted, stages, liveApp, appSessions, reload, runtimes, mcp, languageOutput, previewOutput,
        simulatorSessions, androidSessions, browserSessions, reload.onDidBuild(publish),
        vscode.workspace.onDidOpenTextDocument(startForXaml),
        vscode.workspace.onDidChangeConfiguration(event => {
            if ((event.affectsConfiguration('ferroui.languageServer.path') || event.affectsConfiguration('ferroui.types'))
                && (server.running || vscode.workspace.textDocuments.some(document => document.languageId === xamlLanguage))) void server.start(true);
        }),
        vscode.commands.registerCommand('ferroui.restartLanguageServer', () => server.start(true)),
        vscode.commands.registerCommand('ferroui.importAsset', async () => {
            try {
                const uri = designer.activeDesign?.document.uri ?? vscode.window.activeTextEditor?.document.uri;
                await inventory.assetImport.choose(uri && vscode.workspace.getWorkspaceFolder(uri));
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
        vscode.commands.registerCommand('ferroui.moveAsset', async (uri?: vscode.Uri) => {
            try { await assetMoves.open(uri); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        }),
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
        vscode.commands.registerCommand('ferroui.moveResource', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) return;
            try { await resourceMove.open(editor.document, editor.selection.active); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
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
        return { designer, runtimeSource, fileMoves, propertyRename, typeRename, resourceRename, resourceMove, collaboration, appSessions, publishProjectMetadata: publish };
    }
}

export async function deactivate(): Promise<void> {
    await stopProcesses?.();
    stopProcesses = undefined;
}
