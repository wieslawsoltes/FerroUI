import * as path from 'node:path';
import * as vscode from 'vscode';
import type { BuiltProjects } from './builtProjects';
import { ASSETS_VIEW, AssetsView } from './assetsView';
import { BEHAVIORS_VIEW, BehaviorsView } from './behaviorsView';
import { COLUMNS_VIEW, ColumnsView } from './columnsView';
import type { CommandBindings, CommandSite } from './commandBindings';
import { DATA_VIEW, DesignDataView } from './designDataView';
import { DetailsViews, EVENTS_VIEW, RESOURCES_VIEW } from './detailsViews';
import { DesignerPanel, type DesignerCanvasState } from './designerPanel';
import { DesignerDrag } from './designerDrag';
import type { DictionaryExtraction, DictionarySite } from './dictionaryExtraction';
import type { ResourceRename } from './resourceRename';
import type { ResourceMove } from './resourceMove';
import { DESIGNER_VIEW, restoredDesigner } from './designerViewState';
import { DockView } from './dockView';
import { sourceColumn } from './editorHistory';
import type { EventHandlers, HandlerEditSite } from './eventHandlers';
import { includedSourceUri } from './includedSource';
import { LIBRARY_VIEW, LibraryView, type DefinitionDestination } from './libraryView';
import { OutlineView } from './outlineView';
import { choosePlacement } from './placementPicker';
import { SelectionInspection } from './selectionInspection';
import { PROPERTIES_VIEW, PropertiesView } from './propertiesView';
import { showPropertyTrace, type DesignTraceSource, type PropertyTrace } from './propertyTrace';
import type { ProjectAssets } from './projectAssets';
import type { ProjectDefinition, ProjectLibrary } from './projectLibrary';
import { promptRsxControlName } from './rsxRename';
import { editBinding } from './studioBinding';
import { StudioContext } from './studioContext';
import type { StateTransitionControl } from './stateTransitionProtocol';
import { chooseLayout } from './studioLayout';
import { chooseWrapper } from './studioWrap';
import { promoteResource } from './studioResourcePromotion';
import { editBrush } from './studioBrush';
import { editItemTemplates } from './studioItemTemplates';
import type { ComponentLibrary } from './studioLibrary';
import { StudioView } from './studioView';
import type { LocatedStyleSource } from './stylesProtocol';
import { STYLES_VIEW, StylesView } from './stylesView';
import type { StylesPage } from './stylesProtocol';
import type { TimelineControl, TimelineMarkerAction } from './timelineProtocol';
import { TIMELINE_VIEW, TimelineView } from './timelineView';
import { errorMessage, type CanvasPresentation, type DesignerView, type DetailsView, type DragOrigin, type FrameTarget, type FromAssets, type FromDetails, type FromOutline, type FromToolbox, type Language, type NodeOperation, type NodeTarget, type OutlineMode, type ToolboxState, type ToToolbox } from './protocol';
import { asPosition, asRange, languageOf } from './textDocument';
import { toolboxHtml, webviewOptions } from './view';

const OUTLINE_VIEW = 'ferroui.outline';
const TOOLBOX_VIEW = 'ferroui.toolbox';
const docks = { outline: OUTLINE_VIEW, properties: PROPERTIES_VIEW, toolbox: TOOLBOX_VIEW,
    assets: ASSETS_VIEW, events: EVENTS_VIEW, resources: RESOURCES_VIEW, studio: 'ferroui.studio', timeline: TIMELINE_VIEW,
    data: DATA_VIEW, library: LIBRARY_VIEW, columns: COLUMNS_VIEW, styles: STYLES_VIEW, behaviors: BEHAVIORS_VIEW } as const;

interface DesignerColumns { source?: vscode.ViewColumn; design?: vscode.ViewColumn }

export class DesignerWorkbench implements vscode.Disposable {
    private readonly panels = new Map<string, DesignerPanel>();
    private readonly moves = new Map<DesignerPanel, ReturnType<typeof setTimeout>>();
    private readonly previewChanged = new vscode.EventEmitter<void>();
    readonly onDidChangePreview = this.previewChanged.event;
    private readonly outline: OutlineView;
    private readonly toolbox: DockView<ToolboxState, FromToolbox, ToToolbox>;
    private readonly properties: PropertiesView;
    private readonly inspection: SelectionInspection;
    private readonly details: DetailsViews;
    private readonly assets: AssetsView;
    private readonly studio: StudioView;
    private readonly timeline: TimelineView;
    private readonly data: DesignDataView;
    private readonly library: LibraryView;
    private readonly styles: StylesView;
    readonly columns: ColumnsView;
    readonly behaviors: BehaviorsView;
    private readonly drag = new DesignerDrag(update => {
        this.outline.post(update);
        this.toolbox.post(update);
        this.assets.view.post(update);
        for (const panel of this.panels.values()) panel.post(update);
    }, origin => this.assets.ownsImage(origin), async (panel, origin) => {
        const source = await this.assets.library.imageSource(panel.document.uri, origin.asset ?? '');
        if (source !== origin.source) throw new Error('The image changed during the drag. Start again from Assets.');
    });
    private readonly subscriptions: vscode.Disposable[];
    private nextSession = 0;
    private disposed = false;
    private active?: DesignerPanel;
    private previewKey?: string;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly projects: BuiltProjects, assets: ProjectAssets, readonly handlers: EventHandlers,
        readonly dictionaryExtraction: DictionaryExtraction, private readonly resourceRename: ResourceRename,
        private readonly resourceMove: ResourceMove,
        readonly projectLibrary: ProjectLibrary, readonly commandBindings: CommandBindings) {
        const changed = () => this.updateDocks();
        this.inspection = new SelectionInspection(changed);
        this.outline = new OutlineView(context.extensionUri, changed, message => { void this.outlineMessage(message); });
        this.toolbox = new DockView(context.extensionUri, toolboxHtml, changed, message => { void this.toolboxMessage(message); });
        this.details = new DetailsViews(context.extensionUri, changed, (view, message) => { void this.detailsMessage(view, message); });
        this.assets = new AssetsView(context.extensionUri, assets, changed, message => { void this.assetsMessage(message); });
        this.properties = new PropertiesView(context.extensionUri, changed, (target, operation) => this.edit(target, operation),
            () => { void this.showDock(TOOLBOX_VIEW); },
            async (frame, range) => { await this.active?.revealSource(frame, range); },
            async (target, authoring) => {
                const panel = this.active;
                if (!panel?.canUse(target) || panel.selectedIds.length !== 1 || panel.selectedIds[0] !== target.node) {
                    throw new Error('Select one control in the current preview to edit an authored property.');
                }
                const context = new StudioContext(panel, target.token, () => this.active === panel);
                if (authoring.action === 'itemTemplate' || authoring.action === 'controlTemplate') {
                    await editItemTemplates(context, target.node, authoring.action === 'controlTemplate' ? 'Template' : undefined);
                }
                else if (authoring.action === 'binding') await editBinding(context, target.node, authoring.property);
                else if (authoring.action === 'promoteResource') await promoteResource(context, target.node, authoring.property);
                else await editBrush(context, target.node, authoring.property);
            }, async (frame, target, property) => {
                const panel = this.active;
                if (!panel?.supportsPropertyTrace || !panel.canUse(frame)) throw new Error('Wait for the current native preview.');
                const current = () => this.active === panel && (target.type === 'runtime'
                    ? panel.runtimeSelection?.inspector_id === target.id
                    : panel.selectedIds.length === 1 && panel.selectedIds[0] === target.node);
                const context = new StudioContext(panel, frame.token, current);
                const args = target.type === 'runtime' ? { runtime_node: target.id, property } : { node: target.node, property };
                const trace = await context.read<PropertyTrace<DesignTraceSource>>('designer_property_trace', args);
                await showPropertyTrace(trace, source => context.revealRange(source.range), source => source.text);
            });
        this.timeline = new TimelineView(context.extensionUri, changed);
        this.studio = new StudioView(changed, (context, route) => this.timeline.openTransition(context, route));
        this.data = new DesignDataView(context.extensionUri, changed);
        this.styles = new StylesView(context.extensionUri, changed, async panel => {
            await vscode.window.showTextDocument(panel.document, { viewColumn: this.definitionColumns(panel).source, preserveFocus: true });
            await this.revealDesign(panel, 'canvas');
            await this.revealDesign(panel, 'styles');
        });
        this.library = new LibraryView(changed, async (panel, node) => {
            this.styles.openDefinition(panel, node);
            await this.showDock(STYLES_VIEW, panel.session);
        }, projectLibrary, async (entry, destination) => {
            await this.openProjectDefinition(entry, destination, AbortSignal.timeout(15_000));
        });
        this.columns = new ColumnsView(changed);
        this.behaviors = new BehaviorsView(changed);
        const snippets = vscode.workspace.createFileSystemWatcher('**/.ferroui/toolbox/*.xaml');
        const xaml = vscode.workspace.createFileSystemWatcher('**/*.[xX][aA][mM][lL]');
        const rust = vscode.workspace.createFileSystemWatcher('**/{Cargo.toml,*.[rR][sS]}');
        const refreshResources = (uri: vscode.Uri) => {
            for (const panel of this.panels.values()) panel.dependencyChanged(uri);
        };
        const refreshToolbox = (uri: vscode.Uri) => {
            const folder = vscode.workspace.getWorkspaceFolder(uri);
            if (!folder) return;
            for (const panel of this.panels.values()) {
                if (vscode.workspace.getWorkspaceFolder(panel.document.uri)?.uri.toString() === folder.uri.toString()) panel.configure();
            }
        };
        const editTemplates = async (member?: 'Template') => {
            try {
                const panel = this.active;
                const token = panel?.currentToken;
                if (!panel || token === undefined || panel.status || panel.selectedIds.length !== 1) {
                    throw new Error('Select one control or column in the current FerroUI designer.');
                }
                await editItemTemplates(new StudioContext(panel, token, () => this.active === panel), panel.selectedIds[0], member);
            } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        };
        this.subscriptions = [
            snippets, snippets.onDidCreate(refreshToolbox), snippets.onDidChange(refreshToolbox), snippets.onDidDelete(refreshToolbox),
            xaml, xaml.onDidCreate(refreshResources), xaml.onDidChange(refreshResources), xaml.onDidDelete(refreshResources),
            rust, rust.onDidCreate(refreshResources), rust.onDidChange(refreshResources), rust.onDidDelete(refreshResources),
            vscode.window.registerWebviewPanelSerializer(DESIGNER_VIEW, {
                deserializeWebviewPanel: (panel, state) => this.restore(panel, state),
            }),
            vscode.window.registerWebviewViewProvider(OUTLINE_VIEW, this.outline),
            vscode.window.registerWebviewViewProvider(TOOLBOX_VIEW, this.toolbox),
            vscode.window.registerWebviewViewProvider(PROPERTIES_VIEW, this.properties),
            vscode.window.registerWebviewViewProvider(EVENTS_VIEW, this.details.events),
            vscode.window.registerWebviewViewProvider(RESOURCES_VIEW, this.details.resources),
            vscode.window.registerWebviewViewProvider(ASSETS_VIEW, this.assets.view),
            vscode.window.registerWebviewViewProvider(TIMELINE_VIEW, this.timeline.view),
            vscode.window.registerWebviewViewProvider(DATA_VIEW, this.data.view),
            vscode.window.registerWebviewViewProvider(STYLES_VIEW, this.styles.view),
            vscode.window.onDidChangeActiveTextEditor(() => this.followFocus()),
            vscode.workspace.onWillRenameFiles(event => {
                for (const { oldUri, newUri } of event.files) this.prepareMove(oldUri, newUri);
            }),
            vscode.workspace.onDidRenameFiles(event => {
                for (const { oldUri, newUri } of event.files) void this.finishMove(oldUri, newUri);
            }),
            vscode.window.tabGroups.onDidChangeTabGroups(() => this.followFocus()),
            vscode.window.tabGroups.onDidChangeTabs(() => this.followFocus()),
            vscode.commands.registerCommand('ferroui.openDesigner', (uri?: vscode.Uri) => this.open(uri)),
            vscode.commands.registerCommand('ferroui.showOutline', (session?: number) => this.showDock(OUTLINE_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showProperties', (session?: number) => this.showDock(PROPERTIES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showToolbox', (session?: number) => this.showDock(TOOLBOX_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showEvents', (session?: number) => this.showDock(EVENTS_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showResources', (session?: number) => this.showDock(RESOURCES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showAssets', (session?: number) => this.showDock(ASSETS_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showTimeline', (session?: number) => this.showDock(TIMELINE_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showData', (session?: number) => this.showDock(DATA_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showLibrary', (session?: number) => this.showDock(LIBRARY_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showStyles', (session?: number) => this.showDock(STYLES_VIEW, session)),
            vscode.commands.registerCommand('ferroui.showColumns', () => {
                if (this.columns.available) return this.showDock(COLUMNS_VIEW);
            }),
            vscode.commands.registerCommand('ferroui.showBehaviors', async (session?: number) => {
                if (this.behaviors.available) await this.showDock(BEHAVIORS_VIEW, session);
                else void vscode.window.showInformationMessage('Select one control in an XAML design to edit its behaviors.');
            }),
            vscode.commands.registerCommand('ferroui.editItemTemplates', () => editTemplates()),
            vscode.commands.registerCommand('ferroui.editControlTemplate', () => editTemplates('Template')),
            vscode.commands.registerCommand('ferroui.editCommandBinding', async () => {
                try {
                    const panel = this.active;
                    const token = panel?.currentToken;
                    if (!panel || token === undefined || panel.selectedIds.length !== 1) {
                        throw new Error('Select one authored control in the current XAML design.');
                    }
                    await this.openCommandBinding({ session: panel.session, token, node: panel.selectedIds[0] });
                } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
            vscode.commands.registerCommand('ferroui.rsx.renameControl', async () => {
                try {
                    const editor = vscode.window.activeTextEditor;
                    const panel = editor?.document.languageId === 'rust' ? await this.openDocument(editor.document.uri, editor.selection.active) : this.active;
                    if (panel?.language !== 'rust') throw new Error('Select a named RSX control in a Rust designer.');
                    await promptRsxControlName(panel);
                } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
            vscode.commands.registerCommand('ferroui.studio.layout', async () => {
                try {
                    if (!this.active) throw new Error('Open a FerroUI designer to arrange controls.');
                    await this.layout(this.active);
                } catch (error) {
                    void vscode.window.showErrorMessage(errorMessage(error));
                }
            }),
            vscode.commands.registerCommand('ferroui.studio.wrapSelection', async (session?: number, token?: number) => {
                try {
                    const panel = session !== undefined && token !== undefined ? this.panelFor({ session, token }) : this.active;
                    if (!panel) throw new Error('Open a FerroUI designer to wrap controls.');
                    await chooseWrapper(panel, () => this.active === panel);
                } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
            vscode.commands.registerCommand('ferroui.revealDesignerSource', () => {
                const panel = this.active;
                const node = panel?.selectedNode;
                const token = panel?.currentToken;
                if (panel && node && token !== undefined) return this.select({ session: panel.session, token, node: node.id }, true);
            }),
        ];
        this.updateDocks();
    }

    private async open(uri?: vscode.Uri): Promise<void> {
        try {
            await this.openDocument(uri);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    async openDocument(uri?: vscode.Uri, caret?: vscode.Position, columns?: DesignerColumns): Promise<DesignerPanel> {
        const active = vscode.window.activeTextEditor;
        const document = uri ? await vscode.workspace.openTextDocument(uri) : active?.document;
        const language = document && languageOf(document);
        if (!document || !language) throw new Error('Open a local FerroUI XAML or Rust file.');
        const source = active?.document === document ? active : await vscode.window.showTextDocument(document, { viewColumn: columns?.source });
        const position = caret ? document.validatePosition(caret) : source.selection.active;
        const key = document.uri.toString();
        const previous = this.panels.get(key);
        if (previous) {
            previous.show(position);
            this.setActive(previous);
            return previous;
        }
        const panel = vscode.window.createWebviewPanel(DESIGNER_VIEW, `Design · ${path.basename(document.fileName)}`, {
            viewColumn: columns?.design ?? vscode.ViewColumn.Beside, preserveFocus: true,
        }, { ...webviewOptions(this.context.extensionUri), retainContextWhenHidden: true });
        const designer = this.attach(panel, document, language, position);
        this.setActive(designer);
        return designer;
    }

    async openProjectDefinition(entry: ProjectDefinition, destination: DefinitionDestination, signal: AbortSignal) {
        const { document, range, ensure } = await this.projectLibrary.resolve(entry, signal);
        const columns = this.active && this.definitionColumns(this.active);
        const address = { uri: document.uri.toString(), range: entry.definition.range };
        if (destination === 'source') {
            await vscode.window.showTextDocument(document, { viewColumn: columns?.source, selection: range, preview: false });
            ensure();
            return address;
        }
        const panel = await this.openDocument(document.uri, range.start, columns);
        const snapshot = await panel.waitForSnapshot(signal);
        ensure();
        const token = panel.currentToken;
        if (token === undefined) throw new Error('Wait for the current definition preview.');
        const { components } = await panel.studioQuery('designer_components', {}, undefined, signal) as ComponentLibrary;
        ensure();
        const target = snapshot.nodes.find(node => asRange(node.nameRange).isEqual(range));
        const definition = entry.definition;
        if (panel.currentToken !== token || !target || !components.some(component => component.node === target.id
            && component.kind === definition.kind && component.display_name === definition.name
            && (component.key ?? undefined) === definition.key && (component.target_type ?? undefined) === definition.targetType)) {
            throw new Error('The accepted preview no longer contains this Library definition. Search again.');
        }
        if (snapshot.definitionScope !== target.id) {
            const scope = await panel.studioPresent(token, 'designer_definition', { action: 'enter', node: target.id }, undefined, signal);
            ensure();
            if (scope?.definitionScope !== target.id || panel.snapshot?.definitionScope !== target.id) {
                throw new Error('The design changed while opening the definition.');
            }
        }
        panel.panel.reveal(undefined, true);
        return { ...address, session: panel.session, revision: panel.currentToken };
    }

    private async restore(panel: vscode.WebviewPanel, state: unknown): Promise<void> {
        try {
            const saved = restoredDesigner(state);
            if (saved) {
                const document = await vscode.workspace.openTextDocument(saved.uri);
                const language = languageOf(document);
                if (language && !this.disposed && !this.panels.has(document.uri.toString())) {
                    this.attach(panel, document, language, document.validatePosition(saved.caret), saved);
                    this.followFocus();
                    return;
                }
            }
        } catch (error) {
            this.output.appendLine(`Could not restore the designer. ${errorMessage(error)}`);
        }
        panel.dispose();
    }

    private attach(panel: vscode.WebviewPanel, document: vscode.TextDocument, language: Language,
        caret: vscode.Position, restored?: DesignerCanvasState): DesignerPanel {
        const key = document.uri.toString();
        const designer: DesignerPanel = new DesignerPanel(this.context, this.output, this.projects, panel, document, ++this.nextSession, language,
            caret, () => {
                this.panels.delete(designer.document.uri.toString());
                clearTimeout(this.moves.get(designer));
                this.moves.delete(designer);
                this.followFocus();
            }, message => {
                void this.drag.receive(designer, message).catch(error => designer.post({ type: 'edit', error: errorMessage(error) }));
            }, (token, target, signal) => this.navigateDefinition(designer, token, target, signal), restored);
        designer.onDidChange(() => this.followFocus());
        this.panels.set(key, designer);
        return designer;
    }

    private async navigateDefinition(consumer: DesignerPanel, token: number,
        target: NonNullable<CanvasPresentation['openDefinition']> | string, signal: AbortSignal): Promise<number | undefined> {
        const ensureConsumer = () => {
            signal.throwIfAborted();
            if (consumer.currentToken !== token) throw new Error('The consuming preview changed. Reopen the included definition.');
        };
        if (typeof target === 'string') {
            ensureConsumer();
            const previous = this.panels.get(target);
            if (!previous || previous.isDisposed) return;
            await this.revealDesign(previous, 'canvas');
            return previous.session;
        }
        const uri = await includedSourceUri(consumer.document.uri, target.source);
        ensureConsumer();
        const designer = await this.openDocument(uri, asPosition(target.source.range.start), this.definitionColumns(consumer));
        try {
            await designer.waitForSnapshot(signal);
            ensureConsumer();
            const current = designer.currentToken;
            if (current === undefined) throw new Error('Wait for the included XAML preview.');
            if ('sample' in target) {
                const opened = await designer.studioPresent(current, 'designer_definition', { action: 'enter_included_template', target }, undefined, signal);
                if (!opened || designer.currentToken === undefined) throw new Error('The dictionary changed while opening the template.');
                await designer.revealSource({ session: designer.session, token: designer.currentToken }, target.source.range);
            } else {
                const { editor, range } = await designer.studioQuery('designer_style_source', { source: target.source }, undefined, signal) as LocatedStyleSource;
                const ensureOwner = () => {
                    ensureConsumer();
                    if (current !== designer.currentToken) throw new Error('The included source changed. Reopen it from the consumer.');
                };
                ensureOwner();
                await designer.revealSource({ session: designer.session, token: current }, range);
                ensureOwner();
                this.styles.openSource(designer, editor, consumer);
                await this.revealDesign(designer, 'styles');
            }
            designer.panel.reveal(undefined, true);
            return designer.session;
        } catch (error) {
            if (this.active === designer && !consumer.isDisposed && !signal.aborted) await this.revealDesign(consumer, 'canvas');
            throw error;
        }
    }

    private definitionColumns(panel: DesignerPanel): DesignerColumns {
        const design = panel.panel.viewColumn;
        const source = sourceColumn(panel.document);
        return { design, source: source !== undefined && source !== design ? source
            : design === vscode.ViewColumn.One ? vscode.ViewColumn.Beside : vscode.ViewColumn.One };
    }

    private prepareMove(from: vscode.Uri, to: vscode.Uri): void {
        const panel = this.panels.get(from.toString());
        if (!panel || panel.language !== 'xaml' || to.scheme !== 'file' || !to.path.toLowerCase().endsWith('.xaml')) return;
        clearTimeout(this.moves.get(panel));
        panel.startFileMove();
        this.moves.set(panel, setTimeout(() => {
            this.moves.delete(panel);
            panel.finishFileMove();
        }, 15000));
    }

    cancelFileMove(from: vscode.Uri): void {
        const panel = this.panels.get(from.toString());
        if (!panel || !this.moves.has(panel)) return;
        clearTimeout(this.moves.get(panel));
        this.moves.delete(panel);
        panel.finishFileMove();
    }

    private async finishMove(from: vscode.Uri, to: vscode.Uri): Promise<void> {
        const key = from.toString();
        const panel = this.panels.get(key);
        if (panel && !this.moves.has(panel)) this.prepareMove(from, to);
        if (!panel || !this.moves.has(panel)) return;
        clearTimeout(this.moves.get(panel));
        this.moves.delete(panel);
        try {
            const document = await vscode.workspace.openTextDocument(to);
            if (this.panels.get(key) !== panel) return;
            this.panels.delete(key);
            this.panels.set(document.uri.toString(), panel);
            panel.finishFileMove(document);
        } catch (error) {
            this.output.appendLine(`Could not follow the moved XAML file. ${errorMessage(error)}`);
            panel.finishFileMove();
        }
    }

    designs(): readonly DesignerPanel[] { return [...this.panels.values()]; }
    get activeDesign(): DesignerPanel | undefined { return this.active; }

    async revealDesign(panel: DesignerPanel, view: DesignerView, stylesPage?: StylesPage): Promise<void> {
        if (view === 'canvas') {
            panel.panel.reveal(undefined, false);
            this.setActive(panel);
            return;
        }
        if (view === 'styles' && stylesPage) this.styles.showPage(panel, stylesPage);
        await this.showDock(docks[view], panel.session);
    }

    controlTimeline(panel: DesignerPanel, frame: FrameTarget, control: TimelineControl, signal: AbortSignal) {
        return this.timeline.control(panel, frame, control, signal);
    }

    timelineMarkers(panel: DesignerPanel, frame: FrameTarget, program: number | undefined, signal: AbortSignal, edit?: TimelineMarkerAction) {
        return this.timeline.previewMarkers(panel, frame, program, signal, edit);
    }

    controlStateTransition(panel: DesignerPanel, frame: FrameTarget, control: StateTransitionControl, signal: AbortSignal) {
        return this.timeline.controlTransition(panel, frame, control, signal);
    }

    setOutlineMode(panel: DesignerPanel, mode: OutlineMode): void {
        this.setActive(panel);
        this.outline.switch(mode, panel);
    }

    async projectAssets(panel: DesignerPanel) {
        const folder = vscode.workspace.getWorkspaceFolder(panel.document.uri);
        return folder ? this.assets.library.list(folder, panel.document.uri, true) : [];
    }

    async projectImage(panel: DesignerPanel, id: string): Promise<Extract<DragOrigin, { type: 'image' }>> {
        return { type: 'image', source: await this.assets.library.imageSource(panel.document.uri, id) };
    }

    async projectImageSource(panel: DesignerPanel, target: NodeTarget, id: string): Promise<Extract<NodeOperation, { type: 'setProperty' }>> {
        if (!panel.canUse(target) || !panel.nodes.some(node => node.id === target.node && node.canSetImageSource)) {
            throw new Error('Choose an editable image source in the current design.');
        }
        const image = await this.projectImage(panel, id);
        return { type: 'setProperty', node: target.node, name: 'Source', value: image.source };
    }

    private followFocus(): void {
        if (this.disposed) return;
        const editor = vscode.window.activeTextEditor;
        const candidate = editor && this.panels.get(editor.document.uri.toString());
        const source = candidate?.isDisposed ? undefined : candidate;
        const panels = [...this.panels.values()].filter(designer => !designer.isDisposed);
        const canvas = panels.find(designer => designer.panel.active);
        const fallback = this.active && panels.includes(this.active) && this.active.panel.visible ? this.active : undefined;
        this.setActive(canvas ?? source ?? fallback ?? panels.reverse().find(designer => designer.panel.visible));
    }

    private setActive(panel: DesignerPanel | undefined): void {
        if (this.active !== panel) {
            this.active?.setInspected(false);
            this.active = panel;
        }
        this.updateDocks();
    }

    private updateDocks(): void {
        const panel = this.active;
        panel?.setInspected(this.outline.visible || this.properties.visible || this.toolbox.visible
            || this.details.events.visible || this.details.resources.visible || this.assets.view.visible || this.studio.visible
            || this.timeline.view.visible || this.data.view.visible || this.library.visible || this.columns.visible
            || this.styles.view.visible || this.behaviors.visible);
        const file = panel ? vscode.workspace.asRelativePath(panel.document.uri, false) : '';
        this.studio.update(panel, file);
        this.timeline.update(panel, file);
        this.data.update(panel, file);
        this.library.update(panel, file);
        this.columns.update(panel);
        this.behaviors.update(panel);
        this.styles.update(panel, file);
        const token = panel?.currentToken;
        const node = panel?.selectedNode;
        const frame = panel && token !== undefined && !panel.status ? { session: panel.session, token } : undefined;
        const previewKey = panel ? `${panel.session}:${frame?.token ?? panel.status}` : undefined;
        if (this.previewKey !== previewKey) {
            this.previewKey = previewKey;
            this.previewChanged.fire();
        }
        const inspection = this.properties.visible || this.details.events.visible ? this.inspection.read(panel, frame) : {};
        this.outline.update(panel, frame, file);
        this.toolbox.show({ frame, assets: panel ? [...panel.assets] : [], canInsert: Boolean(panel && this.insertionParent(panel)),
            message: panel?.status }, file);
        this.properties.show({
            session: panel?.session,
            node,
            runtime: panel?.runtimeSelection,
            frame,
            fields: inspection.data?.properties,
            itemTemplates: inspection.data?.itemTemplates,
            controlTemplate: inspection.data?.controlTemplate,
            propertyTrace: panel?.supportsPropertyTrace,
            target: panel && node && token !== undefined && !panel.status
                ? { session: panel.session, token, node: node.id, selection: [...panel.selectedIds] } : undefined,
            selectionCount: panel?.selectedIds.length,
            recordingTime: this.timeline.recordingTime(panel),
            message: panel?.status ?? inspection.message,
        }, file);
        const codeBehind = Boolean(panel?.language === 'xaml' && this.projects.forDocument(panel.document.uri)
            && panel.nodes.find(node => node.parent === null)?.properties.some(attribute => attribute.name.endsWith(':Class')));
        this.details.update(panel, frame, file, inspection, codeBehind);
        this.assets.update(panel, frame, Boolean(panel && this.insertionParent(panel)));
    }

    private insertionParent(panel: DesignerPanel): number | undefined {
        let node = panel.selectedNode ?? panel.nodes.at(0);
        while (node) {
            if (node.canInsert) return node.id;
            node = panel.nodes.find(candidate => candidate.id === node?.parent);
        }
        return undefined;
    }

    private panelFor(frame: FrameTarget): DesignerPanel {
        if (this.active?.canUse(frame)) return this.active;
        throw new Error('The active design changed. Wait for the current preview and try again.');
    }

    private async outlineMessage(message: FromOutline): Promise<void> {
        try {
            switch (message.type) {
                case 'mode': return this.outline.switch(message.mode, this.active);
                case 'layout': return await this.layout(this.panelFor(message.frame));
                case 'select': return await this.outline.select(this.panelFor(message.frame), message.frame, message.id, message.openSource, message.extend);
                case 'reorder': return await this.panelFor(message.frame).place(message.frame, {
                    origin: message.origin,
                    target: { type: 'outline', node: message.anchor, position: message.position },
                });
                case 'beginDrag': case 'endDrag': case 'drop':
                    if (this.active) await this.drag.receive(this.active, message);
                    break;
            }
        } catch (error) {
            this.outline.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private layout(panel: DesignerPanel): Promise<void> {
        return chooseLayout(panel, () => this.active === panel);
    }

    private async toolboxMessage(message: FromToolbox): Promise<void> {
        try {
            if (message.type === 'insert') {
                const panel = this.panelFor(message.frame);
                const asset = panel.assets.find(asset => asset.id === message.control);
                if (!asset) throw new Error('This control is unavailable in the current design.');
                const origin = { type: 'asset' as const, control: asset.id };
                if (message.choosePlacement) {
                    const placement = await choosePlacement(panel, message.frame, origin, asset.title);
                    if (placement) await this.panelFor(message.frame).place(message.frame, placement);
                    return;
                }
                const node = this.insertionParent(panel);
                if (node === undefined) throw new Error('Select a panel that accepts children or drag the control to a drop target.');
                await panel.place(message.frame, { origin, target: { type: 'outline', node, position: 'inside' } });
            } else if (message.type !== 'ready' && this.active) {
                await this.drag.receive(this.active, message);
            }
        } catch (error) {
            this.toolbox.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private async detailsMessage(view: DetailsView, message: FromDetails): Promise<void> {
        if (message.type === 'reveal') {
            await this.active?.revealSource(message.frame, message.range);
            return;
        }
        if (message.type === 'ready') return;
        try {
            if (message.type === 'commandBinding') {
                if (view !== 'events') throw new Error('Use the Events view to edit the command binding.');
                await this.openCommandBinding(message.target);
                this.details.events.post({ type: 'edit', request: message.request });
                return;
            }
            if (message.type === 'extractDictionary' || message.type === 'renameResource' || message.type === 'moveResource') {
                if (view !== 'resources') throw new Error('Use the Resources view to review project resources.');
                const panel = this.panelFor(message.frame);
                if (message.type === 'extractDictionary') {
                    await this.dictionaryExtraction.openReview(await this.dictionarySite(panel, message.frame, message.resource));
                } else if (message.type === 'renameResource') {
                    const document = panel.document;
                    await this.resourceRename.open({ document, request: { version: document.version, position: message.range.start } });
                } else {
                    await this.resourceMove.open(panel.document, message.range.start);
                }
                this.details.resources.post({ type: 'edit', request: message.request });
                return;
            }
            if (message.type === 'handler') {
                if (view !== 'events') throw new Error('Use the Events view for code-behind.');
                const panel = this.panelFor(message.target);
                const site = this.eventHandlerSite(panel, message.target, message.target.node, message.event);
                const validate = site.validate;
                site.validate = () => {
                    this.panelFor(message.target);
                    if (panel.selectedNode?.id !== message.target.node) throw new Error('The event selection changed. Choose the handler again.');
                    validate?.();
                };
                if (message.action === 'create') await this.handlers.create(site);
                else {
                    const revealed = await this.handlers.revealExisting(site).catch(() => {
                        site.validate?.();
                        return false;
                    });
                    if (!revealed && message.range) await panel.revealSource(message.target, message.range);
                }
                this.details.events.post({ type: 'edit', request: message.request });
                return;
            }
            const correctOperation = view === 'events' ? message.operation.type === 'setEvent' : message.operation.type === 'applyResource';
            if (!correctOperation) throw new Error('Use the matching designer view for this edit.');
            await this.edit(message.target, message.operation);
            this.details[view].post({ type: 'edit', request: message.request });
        } catch (error) {
            this.details[view].post({ type: 'edit', request: message.request, error: errorMessage(error) });
        }
    }

    private async openCommandBinding(target: NodeTarget): Promise<void> {
        const panel = this.panelFor(target);
        await this.commandBindings.open(this.commandBindingSite(panel, target, true));
    }

    commandBindingSite(panel: DesignerPanel, target: NodeTarget, requireSelection = false): CommandSite {
        const control = panel.nodes.find(control => control.id === target.node && control.selectable !== false);
        if (!control) throw new Error('Select an authored control in the current XAML design.');
        const validate = () => {
            if (panel.language !== 'xaml' || !panel.canUse(target)
                || requireSelection && (this.active !== panel || panel.selectedIds.length !== 1 || panel.selectedNode?.id !== target.node)) {
                throw new Error('The source or selection changed. Reopen the command binding picker.');
            }
        };
        validate();
        return { document: panel.document, version: panel.document.version, position: control.nameRange.start, validate,
            reveal: range => panel.revealSource(target, range) };
    }

    eventHandlerSite(panel: DesignerPanel, frame: FrameTarget, node: number, event: string): HandlerEditSite {
        const control = panel.nodes.find(control => control.id === node);
        if (!control) throw new Error('Select an authored event target.');
        const validate = () => {
            if (panel.language !== 'xaml' || !panel.canUse(frame)) {
                throw new Error('Use the current XAML design and an authored event target.');
            }
        };
        validate();
        return { document: panel.document, request: { version: panel.document.version, position: control.nameRange.start, event }, validate };
    }

    async dictionarySite(panel: DesignerPanel, frame: FrameTarget, node: number): Promise<DictionarySite> {
        const validate = () => {
            if (panel.language !== 'xaml' || !panel.canUse(frame)) throw new Error('Read the current XAML design before extracting its resources.');
        };
        validate();
        const position = panel.nodes.find(control => control.id === node)?.nameRange.start
            ?? (await panel.resources(frame)).find(resource => resource.id === node)?.range.start;
        validate();
        if (!position) throw new Error('Choose an authored resource or its owner.');
        return { document: panel.document, request: { version: panel.document.version, position }, validate };
    }

    private async assetsMessage(message: FromAssets): Promise<void> {
        try {
            if (message.type === 'preview' || message.type === 'open' || message.type === 'copy' || message.type === 'move') {
                await this.assets.browse(message.type, message.id);
            } else if (message.type === 'setImageSource') {
                const panel = this.panelFor(message.target);
                const operation = await this.projectImageSource(panel, message.target, message.id);
                await this.edit(message.target, operation);
            } else if (message.type === 'insert') {
                const panel = this.panelFor(message.frame);
                const origin = await this.projectImage(panel, message.id);
                if (message.choosePlacement) {
                    const placement = await choosePlacement(panel, message.frame, origin, path.basename(message.id));
                    if (!placement) return;
                    const current = await this.projectImage(panel, message.id);
                    if (current.source !== origin.source) throw new Error('The image moved. Choose its placement again.');
                    await this.panelFor(message.frame).place(message.frame, placement);
                    return;
                }
                const node = this.insertionParent(panel);
                if (node === undefined) throw new Error('Select a panel that accepts children or drag the image onto the canvas.');
                await panel.place(message.frame, { origin, target: { type: 'outline', node, position: 'inside' } });
            } else if ((message.type === 'beginDrag' || message.type === 'endDrag' || message.type === 'drop') && this.active) {
                await this.drag.receive(this.active, message);
            }
        } catch (error) {
            this.assets.view.post({ type: 'assetStatus', message: errorMessage(error), error: true });
        }
    }

    private showDock(id: string, session?: number): Thenable<void> {
        if (session !== undefined) {
            const panel = [...this.panels.values()].find(designer => designer.session === session);
            if (panel) this.setActive(panel);
        }
        return vscode.commands.executeCommand(`${id}.focus`);
    }

    private async select(target: NodeTarget, openSource = false): Promise<void> {
        try {
            await this.active?.select(target, openSource);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    private async edit(target: NodeTarget, operation: NodeOperation): Promise<void> {
        if (this.active?.session !== target?.session || !this.active) {
            throw new Error('The active design changed. Select a control in the current design.');
        }
        await this.active.editNode(target, operation, this.timeline.recordingTime(this.active) !== undefined);
    }

    editPropertyForAgent(panel: DesignerPanel, token: number, operation: Extract<NodeOperation, { type: 'setProperty' | 'resetProperty' }>,
        signal: AbortSignal): Promise<void> {
        return panel.editPropertyForAgent({ session: panel.session, token }, operation,
            this.timeline.recordingTime(panel) !== undefined, signal);
    }

    dispose(): void {
        this.disposed = true;
        this.drag.clear();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.previewChanged.dispose();
        this.outline.dispose();
        this.toolbox.dispose();
        this.properties.dispose();
        this.details.dispose();
        this.assets.dispose();
        this.studio.dispose();
        this.timeline.dispose();
        this.data.dispose();
        this.library.dispose();
        this.styles.view.dispose();
        this.columns.dispose();
        this.behaviors.dispose();
        for (const timer of this.moves.values()) clearTimeout(timer);
        this.moves.clear();
        const panels = [...this.panels.values()];
        this.panels.clear();
        this.active = undefined;
        for (const panel of panels) panel.dispose();
    }
}
