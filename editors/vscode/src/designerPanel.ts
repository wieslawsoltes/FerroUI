import * as path from 'node:path';
import * as vscode from 'vscode';
import { recordedGeometry, recordedMove } from './animationGeometry';
import type { BuiltProject, BuiltProjects } from './builtProjects';
import type { GuideCatalog, GuideEdit } from './canvasGuidesProtocol';
import { CanvasRequests } from './canvasRequests';
import { rustModuleProject, type RustModuleProject } from './cargoTarget';
import { DesignerDependencies, dirtyPreviewSources } from './designerDependencies';
import { DesignerSnapping, snappedPlacement, snappedResize } from './designerSnapping';
import { GuideViews, type GuideAnchor } from './guideViews';
import { previewSourceUri } from './includedSource';
import { LayerViews } from './layerViews';
import type { LayerTransfer } from './layerHistory';
import { NativePreview } from './nativePreview';
import { RustTemplateScopes } from './rustTemplateScopes';
import { DEFAULT_VIEWPORT, errorMessage, isViewport, withViewportTheme, type Asset, type CanvasSnapshot, type DragMessage, type DropHint, type FrameTarget, type FromDesigner, type Language, type NodeOperation, type NodeTarget, type Operation, type Placement, type Snapshot, type Source, type ToDesigner, type Viewport } from './protocol';
import { applyTextEdits, asPosition, asRange, exactRange, offsetAfterChanges, sourceOf } from './textDocument';
import type { TimelinePreviewPose, TimelineTargetOverrides } from './timelineProtocol';
import type { SpacingCatalog, SpacingDraft, SpacingRequest, SpacingResponse } from './spacingProtocol';
import { designerHtml, webviewOptions } from './view';
import type { EditResult, Inspection, InspectionTarget, PublishedAsset, PublishedAssetInput, Range, ResourceRow, RuntimeRow, RuntimeTreeMode } from './protocol';
import type { CanvasAction, CanvasFocusOrder, CanvasInteraction, CanvasPresentation, DefinitionNavigation, DesignDataPresentation, DesignerViewState, ResizeGesture, RustTemplateNavigation, StudioPresentation } from './protocol';
import { isInside } from './workspacePaths';

const mappedStudioEdits = new Set(['designer_selection_layout_edit', 'designer_property', 'designer_grid_track_edit',
    'designer_spacing_edit', 'designer_wrap_selection_edit']);
export type DesignerCanvasState = Pick<DesignerViewState, 'viewport' | 'designData' | 'layerViews' | 'guideViews'>;

interface Frame {
    token: number;
    version: number;
    viewport: Viewport;
    source: Source;
    snapshot: Snapshot;
    canvas: CanvasSnapshot;
    guideCatalog?: GuideCatalog;
    guideAnchor?: GuideAnchor;
    columnAuthoring: boolean;
    behaviorAuthoring: boolean;
    libraryScopes: boolean;
    itemTemplateAuthoring: boolean;
    controlTemplateAuthoring: boolean;
    propertyTrace: boolean;
    includedPropertyTrace: boolean;
    styleComposition: boolean;
    rustStyleComposition: boolean;
    timelineRecording: boolean;
    timelineGeometryRecording: boolean;
    timelineKeyframeDrafts: boolean;
    timelineKeyframeScale: boolean;
    timelineKeyframeTransfer: boolean;
    rsxKeyframeStructure: boolean;
    rsxAnimationPrograms: boolean;
    timelineCurves: boolean;
    timelineCurveVelocity: boolean;
    timelineCombinedPlayback: boolean;
    timelineClipPlacement: boolean;
    stateTransitionPreview: boolean;
    rsxStateAuthoring: boolean;
    rsxStateStructure: boolean;
    rsxStateTriggers: boolean;
}

type PropertyEdit = Extract<NodeOperation, { type: 'setProperty' | 'resetProperty' }>;

export class DesignerPanel implements vscode.Disposable {
    private readonly canvasRequests = new CanvasRequests();
    private readonly dependencies: DesignerDependencies;
    private readonly templateScopes = new RustTemplateScopes();
    private layers: LayerViews;
    private guides: GuideViews;
    private readonly snapping = new DesignerSnapping(() => this.canvasGuides, frame => this.canUse(frame), () => this.getClient());
    private readonly subscriptions: vscode.Disposable[] = [];
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private client?: Promise<NativePreview>;
    private frame?: Frame;
    private disposed = false;
    private ready = false;
    private timer?: ReturnType<typeof setTimeout>;
    private sourceCloseTimer?: ReturnType<typeof setTimeout>;
    private generation = 0;
    private pending = true;
    private relocating = false;
    private rendering = false;
    private editing = false;
    private viewport: Viewport;
    private designData?: DesignDataPresentation;
    private timelinePose?: TimelinePreviewPose;
    private timelineTargets?: TimelineTargetOverrides;
    private animationCatalogEpoch = 0;
    private animationRecording = false;
    private retainedSelection?: { version: number } & ({ ranges: Range[] } | { starts: number[] });
    // VS Code can report several queued source reveals after the designer selects a control.
    private readonly revealedSelections = new WeakMap<vscode.TextEditor, { version: number; selections: vscode.Selection[] }>();
    private timelineProgramSelection?: { version: number; key: string };
    private inspected = false;
    private readers = 0;
    private problem?: string;
    private caret: vscode.Position;
    private selected?: number;
    private selection: number[] = [];
    private runtime?: RuntimeRow;
    private blockOffset?: number;
    private project?: BuiltProject;
    private rustProject?: Promise<RustModuleProject | undefined>;
    assets: readonly Asset[] = [];

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly output: vscode.OutputChannel,
        private readonly projects: BuiltProjects,
        readonly panel: vscode.WebviewPanel,
        private sourceDocument: vscode.TextDocument,
        readonly session: number,
        readonly language: Language,
        caret: vscode.Position,
        onClose: () => void,
        private readonly onDrag: (message: DragMessage | Extract<FromDesigner, { type: 'previewDrop' }>) => void,
        private readonly onDefinitionNavigation: (token: number, target: DefinitionNavigation,
            signal: AbortSignal) => Promise<number | undefined>,
        restored?: DesignerCanvasState,
    ) {
        const document = this.document;
        this.dependencies = new DesignerDependencies(language === 'rust', () => this.schedule(0));
        this.caret = caret;
        this.viewport = restored?.viewport ?? DEFAULT_VIEWPORT;
        this.designData = restored?.designData;
        this.layers = new LayerViews(document, language, restored?.layerViews);
        this.guides = new GuideViews(document, restored?.guideViews);
        this.project = projects.forDocument(document.uri);
        let visible = panel.visible;
        this.subscriptions.push(
            panel.onDidDispose(() => { this.dispose(); onClose(); }),
            panel.onDidChangeViewState(() => {
                if (panel.visible && !visible) {
                    this.syncCanvas();
                    void this.render();
                }
                visible = panel.visible;
                this.changed.fire();
            }),
            panel.webview.onDidReceiveMessage((message: FromDesigner) => { void this.message(message); }),
            projects.onDidChange(() => {
                const project = projects.forDocument(this.document.uri);
                if (project === this.project) return;
                this.project = project;
                this.rustProject = undefined;
                this.schedule(0);
            }),
            vscode.workspace.onDidChangeTextDocument(event => {
                if (!event.contentChanges.length) return;
                if (event.document !== this.document) { this.dependencyChanged(event.document.uri); return; }
                if (this.blockOffset !== undefined) {
                    this.blockOffset = offsetAfterChanges(this.blockOffset, event.contentChanges);
                }
                this.templateScopes.sourceChanged(event.contentChanges);
                const transfer = this.layers.sourceChanged(event);
                if (transfer) this.retainLayerSelection(event.document.version, transfer);
                this.guides.sourceChanged(event.contentChanges);
                this.schedule();
            }),
            vscode.workspace.onDidCloseTextDocument(closed => {
                if (closed === this.document) void this.sourceClosed(closed);
                else if (closed.isDirty || this.language === 'rust') this.dependencyChanged(closed.uri);
            }),
            vscode.window.onDidChangeTextEditorSelection(event => this.editorSelectionChanged(event)),
            vscode.workspace.onDidChangeConfiguration(event => {
                if (event.affectsConfiguration('ferroui.preview.path')) { this.stopClient(); this.schedule(0); }
                if (this.language === 'rust' && event.affectsConfiguration('ferroui.hotReload.cargoPath', this.document.uri)) {
                    this.rustProject = undefined;
                    this.schedule(0);
                }
            }),
            vscode.workspace.onDidGrantWorkspaceTrust(() => {
                if (this.language === 'rust') { this.rustProject = undefined; this.schedule(0); }
            }),
        );
        panel.webview.options = webviewOptions(context.extensionUri);
        panel.webview.html = designerHtml(panel.webview, context.extensionUri, language);
    }

    get document(): vscode.TextDocument { return this.sourceDocument; }

    startFileMove(): void {
        clearTimeout(this.sourceCloseTimer);
        this.relocating = true;
        this.schedule(0);
    }

    finishFileMove(document?: vscode.TextDocument): void {
        if (this.disposed) return;
        this.relocating = false;
        if (document) {
            this.layers = new LayerViews(document, this.language, this.layers.saved);
            this.guides = new GuideViews(document, this.guides.saved);
            this.sourceDocument = document;
            this.rustProject = undefined;
            this.caret = document.validatePosition(this.caret);
            this.project = this.projects.forDocument(document.uri);
            this.panel.title = `Design · ${document.uri.path.split('/').at(-1)}`;
        } else if (this.document.isClosed) {
            this.panel.dispose();
            return;
        }
        this.schedule(0);
    }

    private async sourceClosed(closed: vscode.TextDocument): Promise<void> {
        if (this.relocating) return;
        if (this.language !== 'xaml' || closed.uri.scheme !== 'file') {
            this.panel.dispose();
            return;
        }
        this.schedule(0);
        const missing = await vscode.workspace.fs.stat(closed.uri).then(
            () => false, (error: vscode.FileSystemError) => error.code === 'FileNotFound');
        if (this.disposed || this.document !== closed || this.relocating) return;
        if (missing) {
            // Undo can close the source before reporting the inverse rename.
            this.sourceCloseTimer = setTimeout(() => this.panel.dispose(), 1500);
        } else this.panel.dispose();
    }

    show(caret: vscode.Position): void {
        this.updateCaret(caret);
        if (!this.frame || this.frame.version !== this.document.version) {
            this.blockOffset = this.document.offsetAt(caret);
            this.schedule(0);
        }
        this.panel.reveal(undefined, true);
    }

    get publishedAssetProject(): BuiltProject | undefined { return this.project; }

    async publishedAssets(): Promise<PublishedAsset[]> {
        try {
            return await this.queryPublishedAssets((client, source) => client.publishedAssets(source)) ?? [];
        } catch (error) {
            this.output.appendLine(`Published assets: ${errorMessage(error)}`);
            return [];
        }
    }

    async publishedAsset(address: string): Promise<PublishedAssetInput> {
        const result = await this.queryPublishedAssets((client, source) => client.publishedAsset(source, address));
        if (!result) throw new Error('Build the owning Cargo package to access its published assets.');
        return result;
    }

    private async queryPublishedAssets<T>(query: (client: NativePreview, source: Source) => Promise<T>): Promise<T | undefined> {
        const project = this.project;
        if (!project) return;
        const document = this.document;
        const client = await this.getClient();
        if (!client.capabilities.publishedAssets) return;
        const source = await client.inProject(sourceOf(document, this.language, this.caret), project);
        const result = await query(client, source);
        if (this.project !== project || this.document !== document || this.disposed) {
            throw new Error('The active Cargo design changed. Choose the asset again.');
        }
        return result;
    }

    get currentToken(): number | undefined {
        const frame = this.frame;
        if (frame && !this.disposed && !this.document.isClosed
            && frame.token === this.generation && frame.version === this.document.version) return frame.token;
        return undefined;
    }

    get selectedIds(): readonly number[] { return this.runtime ? [] : this.selection; }

    get nodes(): readonly Snapshot['nodes'][number][] { return this.frame?.snapshot.nodes ?? emptyNodes; }
    get selectedNode(): Snapshot['nodes'][number] | undefined { return this.runtime ? undefined : this.nodes.find(node => node.id === this.selected); }
    get runtimeSelection(): RuntimeRow | undefined { return this.runtime; }
    get dimensions(): Viewport { return this.viewport; }
    get supportsHighContrastTheme(): boolean { return Boolean(this.frame?.canvas.highContrastTheme); }
    get authoredTimelinePose(): TimelinePreviewPose | undefined { return this.timelinePose; }
    get authoredTimelineProgram(): string | undefined {
        if (this.timelineProgramSelection?.version === this.document.version) return this.timelineProgramSelection.key;
    }
    get timelineCatalogEpoch(): number { return this.animationCatalogEpoch; }
    get supportsTimelineRecording(): boolean { return Boolean(this.language === 'xaml' && this.frame?.timelineRecording); }
    get supportsSelectionWrapping(): boolean { return Boolean(this.frame?.canvas.wrapAvailable); }
    get supportsColumnAuthoring(): boolean { return Boolean(this.frame?.columnAuthoring); }
    get supportsItemTemplates(): boolean { return Boolean(this.frame?.itemTemplateAuthoring); }
    get supportsBehaviorAuthoring(): boolean { return this.language === 'xaml' && Boolean(this.frame?.behaviorAuthoring); }
    get supportsLibraryScopes(): boolean { return Boolean(this.frame?.libraryScopes); }
    get supportsControlTemplate(): boolean { return Boolean(this.frame?.controlTemplateAuthoring); }
    get supportsPropertyTrace(): boolean { return Boolean(this.frame?.propertyTrace); }
    get supportsIncludedPropertyTrace(): boolean { return Boolean(this.frame?.includedPropertyTrace); }
    get supportsStyleComposition(): boolean { return Boolean(this.frame?.styleComposition); }
    get supportsRustStyles(): boolean { return Boolean(this.frame?.rustStyleComposition); }
    get supportsTimelineGeometryRecording(): boolean { return Boolean(this.supportsTimelineRecording && this.frame?.timelineGeometryRecording); }
    get supportsTimelineKeyframeDrafts(): boolean {
        return Boolean(this.frame?.timelineKeyframeDrafts && (this.language === 'xaml' || this.frame.snapshot.mappedRustAnimations));
    }
    get supportsTimelineKeyframeScale(): boolean { return Boolean(this.frame?.timelineKeyframeScale); }
    get supportsTimelineKeyframeTransfer(): boolean { return Boolean(this.frame?.timelineKeyframeTransfer); }
    get supportsRustKeyframeStructure(): boolean {
        return Boolean(this.language === 'rust' && this.supportsTimelineKeyframeDrafts && this.frame?.rsxKeyframeStructure);
    }
    get supportsRustAnimationPrograms(): boolean {
        return Boolean(this.language === 'rust' && this.frame?.rsxAnimationPrograms && this.frame.snapshot.image);
    }
    get rustAnimationOwners(): readonly number[] {
        return this.supportsRustAnimationPrograms ? this.frame?.snapshot.rustAnimationOwners ?? [] : [];
    }
    get supportsTimelineCurves(): boolean { return Boolean(this.frame?.timelineCurves); }
    get supportsTimelineCurveVelocity(): boolean { return Boolean(this.frame?.timelineCurveVelocity); }
    get supportsTimelineCombinedPlayback(): boolean { return Boolean(this.frame?.timelineCombinedPlayback); }
    get supportsTimelineClipPlacement(): boolean { return Boolean(this.frame?.timelineClipPlacement); }
    get supportsStateTransitionPreview(): boolean { return Boolean(this.frame?.stateTransitionPreview); }
    get supportsRustStates(): boolean { return Boolean(this.language === 'rust' && this.frame?.rsxStateAuthoring); }
    get supportsRustStateStructure(): boolean { return this.supportsRustStates && Boolean(this.frame?.rsxStateStructure); }
    get supportsRustStateTriggers(): boolean { return this.supportsRustStates && Boolean(this.frame?.rsxStateTriggers); }
    get animationRecordingTime(): number | undefined { return this.animationRecording && !this.timelinePose?.playback ? this.timelinePose?.time : undefined; }
    get isDisposed(): boolean { return this.disposed; }
    get canvasReady(): boolean { return !this.disposed && this.ready && this.panel.visible; }
    get status(): string | undefined {
        if (this.editing) return 'Applying edit…';
        if (this.currentToken === undefined) return this.problem ?? 'Updating preview…';
        return undefined;
    }

    setInspected(inspected: boolean): void {
        if (inspected === this.inspected) return;
        this.inspected = inspected;
        if (inspected) void this.render();
    }

    setAnimationRecording(enabled: boolean): void {
        if (this.animationRecording === enabled) return;
        this.animationRecording = enabled;
        this.showRecording();
    }

    async select(target: NodeTarget, openSource = false, extend = false): Promise<void> {
        if (!this.canUse(target)) return;
        const node = this.nodes.find(node => node.id === target.node);
        if (!node) return;
        if (extend) {
            const existing = this.selectedIds.filter(id => id !== node.id);
            const selection = this.selectedIds.includes(node.id) ? existing : [...existing, node.id].slice(-128);
            this.setSelection(selection.at(-1), selection);
            return;
        }
        const range = asRange(node.nameRange);
        this.caret = range.start;
        this.setSelection(node.id);
        await this.reveal(range, openSource);
    }

    async selectBlock(target: FrameTarget, index: number, openSource = false): Promise<void> {
        if (!this.canUse(target)) throw new Error('Read the current RSX invocations before selecting one.');
        const block = this.frame?.snapshot.blocks[index];
        if (!block) throw new Error('Choose an RSX invocation from the current preview.');
        this.templateScopes.clear();
        this.caret = asPosition(block.start);
        this.blockOffset = this.document.offsetAt(this.caret);
        this.schedule(0);
        await this.reveal(new vscode.Range(this.caret, this.caret), openSource);
    }

    async controlCanvas(action: CanvasAction, signal: AbortSignal) {
        if (action.zoom && action.scale !== undefined) throw new Error('Choose a zoom preset or an explicit scale.');
        const geometry = action.zoom || action.scale !== undefined || action.pan_x !== undefined || action.pan_y !== undefined;
        if (geometry && !(await this.waitForSnapshot(signal)).image) throw new Error('Render an artboard before changing its zoom or pan.');
        if (action.focus_order === true) {
            await this.waitForSnapshot(signal);
            if (!this.frame?.canvas.focusOrderAvailable) throw new Error('Open a native preview with focus-order support.');
        }
        if (action.spacing_helpers === true) {
            await this.waitForSnapshot(signal);
            if (!this.frame?.canvas.spacingAvailable) throw new Error('Open a native preview with spacing helpers.');
        }
        if (action.mode === 'interact') {
            await this.waitForSnapshot(signal);
            if (!this.frame?.canvas.interactive) throw new Error('Open an interactive native preview first.');
        }
        if (!this.canvasReady) {
            const ready = AbortSignal.any([signal, AbortSignal.timeout(5000)]);
            await new Promise<void>((resolve, reject) => {
                const cancel = () => end(ready.reason);
                const end = (error?: unknown) => {
                    change.dispose();
                    ready.removeEventListener('abort', cancel);
                    if (error) reject(error);
                    else resolve();
                };
                const change = this.onDidChange(() => {
                    if (this.disposed) end(new Error('The designer was closed.'));
                    else if (this.canvasReady) end();
                });
                if (ready.aborted) cancel();
                else ready.addEventListener('abort', cancel, { once: true });
            });
        }
        return this.canvasRequests.execute(this.panel.webview, action, signal);
    }

    async runtimeTree(target: FrameTarget, tree: RuntimeTreeMode): Promise<RuntimeRow[]> {
        if (!this.canUse(target)) throw new Error('Wait for the current preview.');
        const answer = await (await this.getClient()).studioQuery(target.token, 'designer_tree', { tree });
        return (answer as { nodes: RuntimeRow[] }).nodes;
    }

    async inspect(frame: FrameTarget, target: InspectionTarget): Promise<Inspection> {
        if (!this.canUse(frame) || !this.frame) throw new Error('Wait for the current preview.');
        const source = this.frame.source;
        return (await this.getClient()).inspect(source, frame.token, target);
    }

    async resources(frame: FrameTarget, target?: number): Promise<ResourceRow[]> {
        if (!this.canUse(frame) || !this.frame) throw new Error('Wait for the current preview.');
        const source = this.frame.source;
        return (await this.getClient()).resources(source, frame.token, target);
    }

    async revealSource(frame: FrameTarget, range: Range, preserveView = false): Promise<void> {
        if (this.canUse(frame)) await this.reveal(this.document.validateRange(asRange(range)), true, preserveView);
    }

    async revealRustModule(frame: FrameTarget, uri: string, range: Range, signal?: AbortSignal, validate?: () => void): Promise<void> {
        if (!this.canUse(frame)) throw new Error('Read the current Rust preview before opening a module.');
        const { document } = await this.checkRustModule(frame.token, uri);
        signal?.throwIfAborted();
        validate?.();
        const source = vscode.window.visibleTextEditors.find(editor => editor.document === this.document);
        await vscode.window.showTextDocument(document, { selection: exactRange(document, range),
            viewColumn: source?.viewColumn ?? vscode.ViewColumn.One, preview: false, preserveFocus: true });
    }

    async checkRustModule(token: number, uri: string) {
        if (this.language !== 'rust') throw new Error('Open a mapped Rust preview to use a companion module.');
        if (this.currentToken !== token) throw new Error('Read the current source before opening a Rust module.');
        const client = await this.getClient();
        const document = await vscode.workspace.openTextDocument(await previewSourceUri(this.document.uri, uri, 'rust'));
        const version = document.version;
        await client.verifyRustModule(token, uri, document.getText());
        if (this.currentToken !== token || document.version !== version || document.isClosed) {
            throw new Error('The Rust source changed. Use the current preview to retry.');
        }
        return { document, version };
    }

    async selectRuntime(target: FrameTarget, runtime: RuntimeRow, openSource: boolean): Promise<void> {
        if (!this.canUse(target)) return;
        if (runtime.source_id !== null) {
            await this.select({ ...target, node: runtime.source_id }, openSource);
            return;
        }
        this.runtime = runtime;
        this.post({ type: 'selection', token: target.token, selected: undefined, runtime });
        this.changed.fire();
    }

    clearRuntimeSelection(): void {
        if (this.runtime) this.setSelection(this.selected);
    }

    async editNode(target: NodeTarget, operation: NodeOperation, recording = false): Promise<void> {
        const checkSelection = () => {
            if (this.currentToken !== target.token || target.session !== this.session
                || operation.node !== target.node || this.selectedNode?.id !== target.node) {
                throw new Error('The selection changed. Use the current Properties view to retry.');
            }
            if ('selection' in target && (!Array.isArray(target.selection) || target.selection.length !== this.selection.length
                || target.selection.some((id, index) => id !== this.selection[index]))) {
                throw new Error('The selection changed before this property edit.');
            }
        };
        checkSelection();
        if (recording && operation.type === 'setProperty' && await this.recordProperty(
            target, operation, target.selection ?? [target.node], AbortSignal.timeout(15_000), checkSelection)) return;
        checkSelection();
        if (target.selection && target.selection.length > 1 && (operation.type === 'setProperty' || operation.type === 'resetProperty')) {
            await this.studioEdit(target.token, 'designer_property', { action: operation.type === 'setProperty' ? 'set' : 'reset',
                property: operation.name, nodes: target.selection, ...(operation.type === 'setProperty' ? { value: operation.value } : {}) },
            target.selection, AbortSignal.timeout(15_000));
            return;
        }
        if (operation.type === 'applyResource' && operation.frame !== target.token) throw new Error('Read the current resource scope.');
        await this.edit(this.frame!, operation);
    }

    async editPropertyForAgent(frame: FrameTarget, operation: PropertyEdit, recording: boolean, signal: AbortSignal): Promise<void> {
        if (recording && operation.type === 'setProperty'
            && await this.recordProperty(frame, operation, [operation.node], signal)) return;
        await this.agentEdit(frame.token, operation, signal);
    }

    private async recordProperty(frame: FrameTarget, operation: Extract<PropertyEdit, { type: 'setProperty' }>,
        nodes: number[], signal: AbortSignal, validate?: () => void): Promise<boolean> {
        const pose = this.timelinePose;
        if (!pose || operation.value.startsWith('{')) return false;
        const target = nodes.length > 1 ? { type: 'selection' as const, nodes }
            : { type: 'source' as const, node: operation.node };
        const inspection = await this.inspect(frame, target);
        signal.throwIfAborted();
        if (!this.canUse(frame) || pose !== this.timelinePose) throw new Error('The recording frame changed. Use the current Properties view to retry.');
        validate?.();
        if (!inspection.properties.some(field => field.name === operation.name && field.canRecord)) return false;
        await this.studioEdit(frame.token, 'designer_record_property', { program: pose.program, time: pose.time,
            nodes, property: operation.name, value: operation.value }, undefined, signal, validate);
        return true;
    }

    canUse(target: FrameTarget): boolean {
        const token = this.currentToken;
        return !this.editing && token !== undefined && target?.session === this.session && target.token === token;
    }

    get canvasGuides() {
        const frame = this.frame;
        if (frame?.guideCatalog && frame.guideAnchor && this.currentToken !== undefined) {
            return this.guides.state({ session: this.session, token: frame.token }, frame.guideAnchor, frame.guideCatalog);
        }
    }

    async editCanvasGuides(target: FrameTarget, epoch: number, edit: GuideEdit, signal?: AbortSignal) {
        const checked = () => {
            const state = this.canvasGuides;
            if (!state || !this.canUse(target) || state.epoch !== epoch) throw new Error('Read the current canvas guides before editing.');
            return state;
        };
        const state = checked();
        if (edit.action === 'configure_snap' && !state.snapping) throw new Error('Update the FerroUI preview helper to configure snapping.');
        const anchor = this.frame!.guideAnchor!;
        const client = await this.getClient();
        signal?.throwIfAborted();
        checked();
        const update = await client.editCanvasGuides(state.settings, edit);
        signal?.throwIfAborted();
        checked();
        this.guides.remember(anchor, update.settings, state.snapping === true);
        const accepted = this.canvasGuides!;
        this.rememberView();
        this.post({ type: 'guides', state: accepted });
        return { ...accepted, selected: update.selected };
    }

    async place(target: FrameTarget, placement: Placement): Promise<void> {
        if (!this.canUse(target) || !this.frame?.snapshot.image) throw new Error('Wait for the current layout before placing controls.');
        await this.edit(this.frame, { type: 'place', frame: target.token, placement });
    }

    async previewPlacement(target: FrameTarget, placement: Placement): Promise<DropHint> {
        if (!this.canUse(target)) throw new Error('The design changed during the drag.');
        const recording = this.recordingGeometryPose() !== undefined && Boolean(recordedMove(placement));
        const snap = placement.target.type === 'canvas' ? placement.target.snap : undefined;
        const adjustment = snap ? await this.snapping.resolve(target, snap, { type: 'place', placement, recording }) : undefined;
        const client = await this.getClient();
        if (!this.canUse(target)) throw new Error('The design changed during the drag.');
        if (snap) this.snapping.validate(target, snap);
        const hint = await client.placement(target.token, adjustment ? snappedPlacement(placement, adjustment.delta) : placement, recording);
        if (snap) this.snapping.validate(target, snap);
        return { ...hint, bounds: adjustment?.bounds ?? hint.bounds, snapLines: adjustment?.guides };
    }

    async previewResize(target: FrameTarget, gesture: ResizeGesture) {
        if (!this.canUse(target)) throw new Error('The design changed during the resize.');
        const { node, handle, dx, dy, snap } = gesture;
        const adjustment = snap && await this.snapping.resolve(target, snap, { type: 'resize', node, handle, dx, dy });
        return { ...snappedResize(gesture, adjustment?.delta ?? { x: 0, y: 0 }), bounds: adjustment?.bounds, snapLines: adjustment?.guides ?? [] };
    }

    get snapshot(): Snapshot | undefined { return this.currentToken === undefined ? undefined : this.frame?.snapshot; }

    async waitForSnapshot(signal: AbortSignal): Promise<Snapshot> {
        signal.throwIfAborted();
        if (this.snapshot) return this.snapshot;
        if (this.problem || this.disposed) throw new Error(this.problem ?? 'The designer was closed.');
        this.readers++;
        try {
            return await new Promise((resolve, reject) => {
                const stop = () => finish(() => reject(signal.reason));
                const finish = (done: () => void) => {
                    changes.dispose();
                    signal.removeEventListener('abort', stop);
                    done();
                };
                const changes = this.onDidChange(() => {
                    const snapshot = this.snapshot;
                    if (snapshot) finish(() => resolve(snapshot));
                    else if (this.problem || this.disposed) finish(() => reject(new Error(this.problem ?? 'The designer was closed.')));
                });
                signal.addEventListener('abort', stop, { once: true });
                void this.render();
            });
        } finally { this.readers--; }
    }

    async agentEdit(token: number, operation: Operation, signal: AbortSignal, validate?: () => void): Promise<void> {
        if (this.currentToken !== token || !this.frame) throw new Error('Read the current designer revision before editing.');
        await this.edit(this.frame, operation, signal, validate);
    }

    async studioTools() {
        const client = await this.getClient();
        const { tools, xamlEdits, rsxEdits, presentations } = await client.studioTools();
        const edits = this.language === 'xaml' ? xamlEdits : [
            ...xamlEdits.filter(tool => this.mappedStudioEdit((tool as { name: string }).name)),
            ...(rsxEdits ?? []).filter(tool => {
                const { name } = tool as { name: string };
                if (name === 'designer_state') return this.supportsRustStates;
                if (name === 'designer_item_template') return this.supportsItemTemplates;
                if (name === 'designer_rsx_style_edit') return this.supportsRustStyles;
                if (name === 'designer_rsx_command_edit') return true;
                return this.supportsTimelineKeyframeDrafts || this.supportsRustAnimationPrograms || this.supportsTimelineClipPlacement;
            }),
        ];
        const availablePresentations = presentations?.filter(tool =>
            (tool as { name: string }).name !== 'designer_behavior' || (this.language === 'xaml' && client.capabilities.behaviorAuthoring));
        const queries = tools.filter(tool => {
            const { name } = tool as { name: string };
            if (name === 'designer_rsx_styles') return this.language === 'rust' && this.supportsRustStyles;
            if (name === 'designer_rsx_commands') return this.language === 'rust';
            return this.language === 'xaml' || name !== 'designer_behaviors';
        });
        return { tools: queries, presentations: availablePresentations ?? [], edits };
    }

    private mappedStudioEdit(name: string): boolean {
        return mappedStudioEdits.has(name) || (name === 'designer_columns_edit' && this.supportsColumnAuthoring)
            || (name === 'designer_rsx_style_edit' && this.supportsRustStyles)
            || name === 'designer_rsx_command_edit';
    }

    async studioQuery(name: string, args: object, selection: number[] | undefined, signal: AbortSignal): Promise<unknown> {
        await this.waitForSnapshot(signal);
        const token = this.currentToken!;
        const client = await this.getClient();
        signal.throwIfAborted();
        if (this.editing || this.currentToken !== token) throw new Error('Wait for the current preview.');
        return client.studioQuery(token, name, args, this.studioSelection(selection));
    }

    async studioEdit(token: number, name: string, args: object, selection: number[] | undefined, signal: AbortSignal,
        validate?: () => void): Promise<EditResult> {
        const { action, previous } = args as { action?: string; previous?: number | null };
        const rustKeyframe = name === 'designer_program' && this.supportsTimelineKeyframeDrafts
            && ((action === 'upsert_keyframe' && (previous != null || this.supportsRustKeyframeStructure))
                || action === 'move_keyframes' || (action === 'scale_keyframes' && this.supportsTimelineKeyframeScale)
                || (this.supportsTimelineKeyframeTransfer && (action === 'mirror_keyframes' || action === 'paste_keyframes'))
                || (this.supportsRustKeyframeStructure && action !== undefined &&
                    ['record_keyframe', 'remove_keyframe', 'duplicate_keyframes', 'remove_keyframes'].includes(action)));
        const rustState = name === 'designer_state' && this.supportsRustStates
            && (action === 'upsert' || (this.supportsRustStateStructure && action === 'remove')
                || (this.supportsRustStateTriggers && action === 'set_event_trigger'));
        const rustProgram = name === 'designer_program' && this.supportsRustAnimationPrograms
            && action !== undefined && ['create', 'rename', 'duplicate', 'remove'].includes(action);
        const rustPlacement = name === 'designer_program' && action === 'place_clips' && this.supportsTimelineClipPlacement;
        const rustTemplate = name === 'designer_item_template' && this.supportsItemTemplates
            && (action === 'create_inline' || action === 'reset');
        if (this.language === 'rust' && !rustKeyframe && !rustProgram && !rustState && !rustPlacement && !rustTemplate && !this.mappedStudioEdit(name)) {
            throw new Error('This Studio operation requires mapped RSX source or XAML.');
        }
        if (this.editing) throw new Error('A designer edit is already in progress.');
        const frame = this.frame;
        if (!frame || this.currentToken !== token) throw new Error('Read the current designer revision before editing.');
        const targets = this.studioSelection(selection);
        try {
            return await this.commitEdit(frame, client => client.studioEdit(frame.source, token, name, args, targets), signal, validate);
        } finally {
            if (name !== 'designer_spacing_edit' && name !== 'designer_wrap_selection_edit' && this.currentToken === token) this.schedule(0);
        }
    }

    private async spacing(message: SpacingRequest): Promise<Partial<SpacingResponse>> {
        const { frame, selection, operation } = message;
        if (!this.canUse(frame) || !this.frame?.canvas.spacingAvailable || !selection.length) {
            throw new Error('Select authored controls in the current preview to measure spacing.');
        }
        const accepted = this.selection;
        const validate = () => {
            if (this.currentToken !== frame.token || this.runtime || this.selection !== accepted ||
                accepted.length !== selection.length || accepted.some((id, index) => id !== selection[index])) {
                throw new Error('The preview or selection changed. Use the current spacing handles.');
            }
        };
        const client = await this.getClient();
        validate();
        if (operation.kind === 'measure') {
            const catalog = await client.studioQuery(frame.token, 'designer_spacing_helpers', {}, accepted) as SpacingCatalog;
            validate();
            return { catalog };
        }
        const { model, handle } = operation;
        let adjustment = operation.adjustment;
        if (operation.kind !== 'commit') {
            const draft = await client.studioQuery(frame.token, 'designer_spacing_draft', { model, handle, adjustment }, accepted) as SpacingDraft;
            validate();
            if (operation.kind === 'draft') return { draft };
            const { label, ownerLabel, value, minimum, maximum } = draft.handle;
            const input = await vscode.window.showInputBox({
                title: `${label} · ${ownerLabel}`,
                prompt: 'Enter a value in logical pixels',
                value: String(value),
                validateInput: entry => {
                    const number = Number(entry);
                    if (!entry.trim() || !Number.isFinite(number)) return 'Enter a finite number.';
                    if (number < minimum) return `Enter a value of at least ${minimum}.`;
                    if (maximum !== null && number > maximum) return `Enter a value of at most ${maximum}.`;
                },
            });
            if (input === undefined) return {};
            validate();
            adjustment = { value: Number(input) };
        }
        await this.studioEdit(frame.token, 'designer_spacing_edit', { model, handle, adjustment }, accepted, AbortSignal.timeout(15_000), validate);
        return {};
    }

    async studioPresent(token: number, name: string, args: object, selection: number[] | undefined, signal: AbortSignal): Promise<StudioPresentation | undefined> {
        const action = args as { action?: string; node?: number };
        const view = this.snapshot;
        if (name === 'designer_definition' && action.action === 'exit' && view?.rustTemplate) {
            if (!this.canUse({ session: this.session, token })) throw new Error('Read the current Rust template preview before returning.');
            signal.throwIfAborted();
            const scope = this.templateScopes.leave(this.document, view);
            await this.switchTemplateView(scope.previous, scope.owner, signal);
            if (scope.consumer && this.currentToken !== undefined) {
                const navigationSession = await this.onDefinitionNavigation(this.currentToken, scope.consumer, signal);
                if (navigationSession !== undefined) return { returnToConsumer: scope.consumer, navigationSession };
            }
            return;
        }
        const presentation = await this.present(token, signal, (client, next) => client.studioPresent(token, next, name, args, this.studioSelection(selection)), true,
            presentation => {
                if (name === 'designer_design_data_edit') this.rememberDesignData(args);
                if (name === 'designer_layer' && action.action === 'reset') this.layers.reset(view?.activeBlock ?? null);
                if (name === 'designer_program') {
                    this.timelinePose = presentation.timelinePosition?.pose ?? undefined;
                    if (presentation.timelinePosition?.elapsed !== undefined || !this.timelinePose) this.setAnimationRecording(false);
                }
                if (name === 'designer_state' || name === 'designer_definition' || name === 'designer_behavior') {
                    this.timelinePose = undefined;
                    this.setAnimationRecording(false);
                }
                if (presentation.timelineTargets !== undefined) this.timelineTargets = presentation.timelineTargets;
                if (presentation.timelineTargets !== undefined || name === 'designer_definition'
                    || name === 'designer_behavior'
                    || (name === 'designer_state' && (args as { action?: string }).action !== 'advance_preview')) this.animationCatalogEpoch++;
            });
        if (this.language === 'rust' && name === 'designer_item_template' && !presentation?.openRustTemplate) {
            throw new Error('Read the current Rust template slot before opening it.');
        }
        const template = presentation?.openRustTemplate;
        if (template) {
            const frame = this.snapshot;
            if (!frame || !this.canUse({ session: this.session, token: token + 1 })) {
                throw new Error('The Rust view changed before opening the template.');
            }
            signal.throwIfAborted();
            if (template.uri) {
                const navigationSession = await this.onDefinitionNavigation(token + 1, { ...template, uri: template.uri }, signal);
                return { ...presentation, navigationSession };
            }
            const owner = frame.nodes.some(node => node.id === action.node) ? action.node : this.selected;
            const body = this.templateScopes.enter(this.document, frame, template, owner);
            await this.switchTemplateView(body, undefined, signal);
            return presentation;
        }
        const navigation = presentation?.openDefinition ?? presentation?.returnToConsumer;
        if (navigation && presentation) {
            const navigationSession = await this.onDefinitionNavigation(token + 1, navigation, signal);
            return navigationSession === undefined
                ? { ...presentation, openDefinition: undefined, returnToConsumer: undefined }
                : { ...presentation, navigationSession };
        }
        return presentation;
    }

    async openCompanionRustTemplate(template: RustTemplateNavigation, consumer: string, version: number, signal: AbortSignal,
        ensureConsumer: () => void): Promise<void> {
        let snapshot = await this.waitForSnapshot(signal);
        if (this.language !== 'rust' || this.document.version !== version || this.currentToken === undefined) {
            throw new Error('The template source changed. Reopen Item Templates.');
        }
        if (!snapshot.image || !snapshot.activeBlock) {
            snapshot = await this.switchTemplateView(this.document.offsetAt(exactRange(this.document, template.range).start), undefined, signal);
        }
        ensureConsumer();
        const body = this.templateScopes.enter(this.document, snapshot, template, undefined, consumer);
        await this.switchTemplateView(body, undefined, signal);
    }

    private async switchTemplateView(offset: number, owner: number | undefined, signal: AbortSignal): Promise<Snapshot> {
        const document = this.document;
        const version = document.version;
        const start = document.positionAt(offset);
        this.caret = start;
        this.blockOffset = offset;
        this.timelinePose = undefined;
        this.timelineTargets = undefined;
        this.animationCatalogEpoch++;
        this.setAnimationRecording(false);
        this.schedule(0);
        const snapshot = await this.waitForSnapshot(signal);
        if (document !== this.document || version !== document.version || !snapshot.image || !snapshot.activeBlock
            || !asPosition(snapshot.activeBlock.start).isEqual(start)) {
            throw new Error('The selected Rust view changed while rendering.');
        }
        if (owner !== undefined) {
            const node = snapshot.nodes.find(node => document.offsetAt(asPosition(node.nameRange.start)) === owner);
            if (node) this.setSelection(node.id);
        }
        return snapshot;
    }

    async interact(token: number, action: CanvasInteraction, signal: AbortSignal): Promise<void> {
        if (!this.frame?.canvas.interactive) throw new Error('Open an interactive native preview first.');
        await this.present(token, signal, (client, next) => client.canvasInput(token, next, action), false);
    }

    private async present(token: number, signal: AbortSignal,
        update: (client: NativePreview, next: number) => Promise<CanvasPresentation>, syncSelection: boolean,
        accepted?: (presentation: CanvasPresentation) => void): Promise<CanvasPresentation | undefined> {
        const frame = this.frame;
        if (!frame || !this.canUse({ session: this.session, token })) throw new Error('Read the current designer revision before updating the preview.');
        this.editing = true;
        this.changed.fire();
        this.post({ type: 'pending' });
        try {
            const client = await this.getClient();
            signal.throwIfAborted();
            if (this.currentToken !== token) throw new Error('The source changed before the preview action.');
            const next = token + 1;
            const presentation = await update(client, next);
            if (this.currentToken !== token) return;
            const metrics = new Map(presentation.nodes.map(node => [node.id, node]));
            const nodes = frame.snapshot.nodes.map(node => {
                const change = metrics.get(node.id);
                return { ...node, bounds: change?.bounds ?? null, canMove: change?.canMove ?? false,
                    canInsert: change?.canInsert ?? false, selectable: change?.selectable };
            });
            accepted?.(presentation);
            const snapshot = { ...frame.snapshot, nodes, image: presentation.image, notices: presentation.notices,
                timelinePose: this.timelinePose, definitionScope: presentation.definitionScope ?? undefined,
                definitionSelection: presentation.definitionScope == null ? undefined : presentation.selection };
            const canvasNodes = nodes.map(({ id, label, name, bounds, canMove, selectable }) => ({ id, label, name, bounds, canMove, selectable }));
            const guideAnchor = this.guideAnchor(snapshot, presentation.definitionScope);
            this.frame = { ...frame, token: next, snapshot, guideAnchor, canvas: { ...frame.canvas, ...snapshot, nodes: canvasNodes } };
            this.generation = next;
            this.runtime = undefined;
            if (syncSelection) {
                this.selection = presentation.selection;
                this.selected = this.selection.at(-1);
            }
            if (presentation.layerView) this.layers.remember(presentation.layerView, frame.snapshot.activeBlock);
            this.syncCanvas();
            return presentation;
        } catch (error) {
            if (this.currentToken === token) this.schedule(0);
            throw error;
        } finally {
            this.editing = false;
            this.changed.fire();
        }
    }

    private rememberDesignData(args: object): void {
        const { action, name, enabled } = args as Record<string, unknown>;
        if (action === 'enabled' && typeof enabled === 'boolean') {
            this.designData = { ...this.designData, enabled };
        } else if (action === 'fixture') {
            this.designData = { enabled: true, fixture: typeof name === 'string' ? name : undefined };
        }
    }

    private studioSelection(selection?: number[]): number[] {
        return selection ?? [...this.selectedIds];
    }

    configure(viewport?: Viewport): void {
        if (viewport) this.viewport = viewport;
        this.schedule(0);
    }

    async selectNodes(target: FrameTarget, ids: number[]): Promise<void> {
        if (!this.canUse(target) || ids.length > 128 || new Set(ids).size !== ids.length
            || ids.some(id => !this.nodes.some(node => node.id === id))) throw new Error('Select distinct nodes from the current designer frame.');
        this.setSelection(ids.at(-1), ids);
    }

    private setSelection(id: number | undefined, selection = id === undefined ? [] : [id]): void {
        if (id === this.selected && !this.runtime && selection.length === this.selection.length
            && selection.every((node, i) => this.selection[i] === node)) return;
        this.runtime = undefined;
        this.selected = id;
        this.selection = selection;
        const token = this.currentToken;
        if (token !== undefined) this.post({ type: 'selection', token, selected: id, selection });
        this.rememberView();
        this.changed.fire();
    }

    private editorSelectionChanged(event: vscode.TextEditorSelectionChangeEvent): void {
        const { textEditor, kind, selections: [selection] } = event;
        if (textEditor.document !== this.document || !selection) return;
        const revealed = this.revealedSelections.get(textEditor);
        if (revealed?.version === this.document.version && kind !== vscode.TextEditorSelectionChangeKind.Mouse
            && kind !== vscode.TextEditorSelectionChangeKind.Keyboard) {
            const match = revealed.selections.findIndex(range => range.isEqual(selection));
            if (match !== -1) {
                revealed.selections.splice(0, match + 1);
                return;
            }
        }
        this.revealedSelections.delete(textEditor);
        this.updateCaret(selection.active);
    }

    private updateCaret(caret: vscode.Position, keepInvocation = false): void {
        this.caret = caret;
        const frame = this.frame;
        if (!frame || frame.version !== this.document.version) return;
        const active = frame.snapshot.activeBlock ? asRange(frame.snapshot.activeBlock) : undefined;
        const block = keepInvocation && active?.contains(caret) ? undefined : frame.snapshot.blocks.reduce<vscode.Range | undefined>((best, span) => {
            const range = asRange(span);
            return range.contains(caret) && (!best || best.contains(range)) ? range : best;
        }, undefined);
        if (block) {
            this.blockOffset = this.document.offsetAt(block.start);
            if (!active?.isEqual(block)) {
                this.templateScopes.clear();
                this.schedule(0);
                return;
            }
        }
        if (this.currentToken !== undefined) {
            const node = selectionAt(frame.snapshot, caret);
            if (node !== undefined && (node !== this.selected || this.runtime)) this.setSelection(node);
        }
    }

    private schedule(delay = 180): void {
        this.generation++;
        this.pending = true;
        this.problem = undefined;
        this.post({ type: 'pending' });
        this.rememberView();
        this.changed.fire();
        clearTimeout(this.timer);
        this.timer = setTimeout(() => { this.timer = undefined; void this.render(); }, delay);
    }

    dependencyChanged(uri: vscode.Uri): void {
        if (this.disposed) return;
        if (this.language === 'rust' && uri.scheme === 'file' && path.basename(uri.fsPath) === 'Cargo.toml'
            && isInside(path.dirname(uri.fsPath), this.document.uri.fsPath)) {
            this.rustProject = undefined;
            this.schedule();
        } else if (this.dependencies.change(uri)) this.schedule();
    }

    private async render(): Promise<void> {
        if (this.disposed || this.relocating || this.document.isClosed || !(this.inspected || this.readers || this.canvasReady)
            || !this.pending || this.rendering || this.timer) return;
        this.rendering = true;
        this.pending = false;
        const token = this.generation;
        const version = this.document.version;
        const requested = this.viewport;
        const caret = this.language === 'rust' && this.blockOffset !== undefined
            ? this.document.positionAt(this.blockOffset) : this.document.validatePosition(this.caret);
        const source = sourceOf(this.document, this.language, caret);
        const project = this.project;
        this.dependencies.start();
        try {
            const client = await this.getClient();
            const highContrastTheme = Boolean(client.capabilities.canvasHighContrast);
            const viewport = requested.theme && !highContrastTheme ? withViewportTheme(requested, 'dark') : requested;
            const input = await client.inProject(source, project);
            const folder = vscode.workspace.getWorkspaceFolder(this.document.uri);
            const rustProject = this.language === 'rust' && client.capabilities.rustModuleSources && vscode.workspace.isTrusted
                ? await (this.rustProject ??= rustModuleProject(this.document.uri).catch(error => {
                    this.output.appendLine(`Rust module discovery: ${errorMessage(error)}`);
                    return undefined;
                })) : undefined;
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            const { layerView, ...snapshot } = await client.render(input, viewport, token, {
                workspaceRoot: folder?.uri.scheme === 'file' ? folder.uri.fsPath : undefined,
                rustProject,
                localSources: dirtyPreviewSources([this.document.uri], this.document, Boolean(rustProject)),
                designData: this.designData, layerView: this.layers.input(caret), timelinePose: this.timelinePose,
                timelineTargets: this.timelineTargets,
            });
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            if (this.dependencies.accept(snapshot.dependencies ?? [])) { this.schedule(0); return; }
            this.viewport = viewport;
            snapshot.rustTemplate = this.templateScopes.accept(this.document, snapshot);
            if (layerView) this.layers.remember(layerView, snapshot.activeBlock);
            const anchor = snapshot.activeBlock?.start;
            const previousAnchor = this.frame?.snapshot.activeBlock?.start;
            const sameView = this.frame?.version === version && previousAnchor?.line === anchor?.line
                && previousAnchor?.character === anchor?.character;
            this.assets = [...client.assets, ...(snapshot.projectAssets ?? [])].filter(asset => this.language === 'xaml' || asset.rust);
            const nodes = snapshot.nodes.map(({ id, label, name, bounds, canMove, selectable }) => ({ id, label, name, bounds, canMove, selectable }));
            if (snapshot.image) this.timelinePose = snapshot.timelinePose;
            this.frame = { token, version, viewport, source: input, snapshot, timelineRecording: Boolean(client.capabilities.timelineRecording),
                guideCatalog: client.canvasGuides, guideAnchor: this.guideAnchor(snapshot, snapshot.definitionScope),
                columnAuthoring: Boolean(this.language === 'rust' ? client.capabilities.rsxColumnAuthoring : client.capabilities.columnAuthoring),
                behaviorAuthoring: Boolean(client.capabilities.behaviorAuthoring),
                libraryScopes: Boolean(client.capabilities.libraryScopes),
                itemTemplateAuthoring: Boolean(this.language === 'rust' ? client.capabilities.rsxItemTemplateAuthoring : client.capabilities.itemTemplateAuthoring),
                controlTemplateAuthoring: Boolean(client.capabilities.controlTemplateAuthoring),
                propertyTrace: Boolean(client.capabilities.propertyTrace),
                includedPropertyTrace: Boolean(client.capabilities.includedPropertyTrace),
                styleComposition: Boolean(client.capabilities.styleComposition),
                rustStyleComposition: Boolean(client.capabilities.rustStyleComposition),
                timelineGeometryRecording: Boolean(client.capabilities.timelineGeometryRecording),
                timelineKeyframeDrafts: Boolean(client.capabilities.timelineKeyframeDrafts),
                timelineKeyframeScale: Boolean(client.capabilities.timelineKeyframeScale),
                timelineKeyframeTransfer: Boolean(client.capabilities.timelineKeyframeTransfer),
                rsxKeyframeStructure: Boolean(client.capabilities.rsxKeyframeStructure),
                rsxAnimationPrograms: Boolean(client.capabilities.rsxAnimationPrograms),
                timelineCurves: Boolean(client.capabilities.timelineCurves),
                timelineCurveVelocity: Boolean(client.capabilities.timelineCurveVelocity),
                timelineCombinedPlayback: Boolean(client.capabilities.timelineCombinedPlayback),
                timelineClipPlacement: Boolean(client.capabilities.timelineClipPlacement),
                stateTransitionPreview: Boolean(client.capabilities.stateTransitionPreview),
                rsxStateAuthoring: Boolean(client.capabilities.rsxStateAuthoring),
                rsxStateStructure: Boolean(client.capabilities.rsxStateStructure),
                rsxStateTriggers: Boolean(client.capabilities.rsxStateTriggers),
                canvas: { ...snapshot, nodes, highContrastTheme, interactive: Boolean(client.capabilities.canvasInteraction && snapshot.image),
                    focusOrderAvailable: Boolean(client.capabilities.canvasFocusOrder && snapshot.image),
                    spacingAvailable: Boolean(client.capabilities.spacingHelpers && snapshot.image),
                    wrapAvailable: Boolean(client.capabilities.selectionWrapping && snapshot.image) } };
            this.runtime = undefined;
            const retained = this.retainedSelection?.version === version && snapshot.image
                ? ('ranges' in this.retainedSelection
                    ? this.retainedSelection.ranges.map(range => snapshot.nodes.find(node => asRange(node.nameRange).isEqual(asRange(range)))?.id)
                    : this.retainedSelection.starts.map(start => snapshot.nodes.find(node => node.layerStart === start)?.id))
                    .filter((id): id is number => id !== undefined)
                : [];
            this.retainedSelection = undefined;
            if (retained.length) {
                this.selection = retained;
                this.selected = retained.at(-1);
            } else if (snapshot.definitionSelection?.length) {
                this.selection = snapshot.definitionSelection;
                this.selected = this.selection.at(-1);
            } else {
                this.selected = (sameView ? this.selected : undefined) ?? selectionAt(snapshot, this.caret) ?? snapshot.nodes[0]?.id;
                this.selection = sameView && this.selection.length ? this.selection.filter(id => snapshot.nodes.some(node => node.id === id))
                    : this.selected === undefined ? [] : [this.selected];
            }
            if (anchor) this.blockOffset = this.document.offsetAt(asPosition(anchor));
            this.syncCanvas();
            this.changed.fire();
        } catch (error) {
            if (!this.disposed && token === this.generation) {
                this.problem = errorMessage(error);
                this.syncCanvas();
                this.changed.fire();
            }
        } finally {
            this.dependencies.stop();
            this.rendering = false;
            void this.render();
        }
    }

    private async message(message: FromDesigner): Promise<void> {
        if (this.disposed || !message) return;
        if (message.type === 'ready') {
            this.ready = true;
            this.syncCanvas();
            this.changed.fire();
            void this.render();
            return;
        }
        if (message.type === 'canvasResult') {
            this.canvasRequests.receive(message);
            return;
        }
        if (message.type === 'spacing') {
            const response: SpacingResponse = { type: 'spacingResult', request: message.request };
            try {
                Object.assign(response, await this.spacing(message));
            } catch (error) {
                response.error = errorMessage(error);
            }
            this.post(response);
            return;
        }
        if (message.type === 'guideEdit') {
            try {
                const update = await this.editCanvasGuides(message.frame, message.epoch, message.edit);
                this.post({ type: 'guideResult', frame: message.frame, request: message.request, selected: update.selected });
            } catch (error) {
                this.post({ type: 'guideResult', frame: message.frame, request: message.request, error: errorMessage(error) });
            }
            return;
        }
        if (message.type === 'previewResize') {
            let preview: Awaited<ReturnType<DesignerPanel['previewResize']>> | undefined;
            try { preview = await this.previewResize(message.frame, message); }
            catch { /* An outdated preview hint can retire independently of the gesture. */ }
            this.post({ type: 'resizeHint', request: message.request, preview });
            return;
        }
        if (message.type === 'refresh') {
            this.schedule(0);
            return;
        }
        if (message.type === 'leaveDefinition') {
            const snapshot = this.snapshot;
            if (this.currentToken === message.token && (snapshot?.definitionScope !== undefined || snapshot?.rustTemplate)) {
                try {
                    await this.studioPresent(message.token, 'designer_definition', { action: 'exit' }, undefined, AbortSignal.timeout(15_000));
                } catch (error) { this.post({ type: 'edit', error: errorMessage(error) }); }
            }
            return;
        }
        if (message.type === 'dock') {
            const commands = { outline: 'ferroui.showOutline', properties: 'ferroui.showProperties', toolbox: 'ferroui.showToolbox',
                events: 'ferroui.showEvents', resources: 'ferroui.showResources', assets: 'ferroui.showAssets',
                studio: 'ferroui.showStudio', timeline: 'ferroui.showTimeline', data: 'ferroui.showData', library: 'ferroui.showLibrary',
                columns: 'ferroui.showColumns', styles: 'ferroui.showStyles', behaviors: 'ferroui.showBehaviors' } as const;
            await vscode.commands.executeCommand(commands[message.view], this.session);
            return;
        }
        if (message.type === 'beginDrag' || message.type === 'endDrag' || message.type === 'drop' || message.type === 'previewDrop') {
            this.onDrag(message);
            return;
        }
        if (message.type === 'viewport') {
            if (isViewport(message.viewport)) this.configure(message.viewport);
            return;
        }
        const frame = this.frame;
        if (!frame || !('token' in message) || this.currentToken === undefined || this.currentToken !== message.token) {
            if (message.type === 'apply') this.post({ type: 'edit', error: 'The source changed. Use the current preview to retry.' });
            return;
        }
        try {
            if (message.type === 'wrapSelection') {
                await vscode.commands.executeCommand('ferroui.studio.wrapSelection', this.session, frame.token);
            } else if (message.type === 'focusOrder') {
                if (!frame.canvas.focusOrderAvailable) return;
                const client = await this.getClient();
                if (this.currentToken !== frame.token) return;
                const { overlay } = await client.studioQuery(frame.token, 'designer_focus_order', { overlay: true }) as { overlay: CanvasFocusOrder };
                if (this.currentToken === frame.token) this.post({ type: 'focusOrder', token: frame.token, overlay });
            } else if (message.type === 'select') {
                await this.select({ session: this.session, token: message.token, node: message.id }, message.openSource, message.extend);
            } else if (message.type === 'block') {
                await this.selectBlock({ session: this.session, token: message.token }, message.index);
            } else if (message.type === 'resize' && !this.editing && this.selection.length === 1 && this.selected === message.node) {
                const { node, handle, dx, dy, snap } = message;
                await this.edit(frame, { type: 'resize', frame: message.token, node, handle, dx, dy, snap });
            } else if (message.type === 'interaction') {
                await this.interact(message.token, message.action, AbortSignal.timeout(15_000));
            } else if (message.type === 'apply') {
                await this.edit(frame, { type: 'applyCounterpart', source: message.source });
                this.post({ type: 'edit' });
            }
        } catch (error) {
            if (message.type === 'focusOrder') {
                if (this.currentToken === frame.token) this.post({ type: 'focusOrder', token: frame.token, error: errorMessage(error) });
            } else this.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private async edit(frame: Frame, operation: Operation, signal?: AbortSignal, validate?: () => void): Promise<void> {
        const pose = this.recordingGeometryPose();
        const target = { session: this.session, token: frame.token };
        if (operation.type === 'place' && operation.placement.target.type === 'canvas' && operation.placement.target.snap) {
            const { placement } = operation;
            const snap = operation.placement.target.snap;
            const adjustment = await this.snapping.resolve(target, snap,
                { type: 'place', placement, recording: Boolean(pose && recordedMove(placement)) });
            this.snapping.validate(target, snap);
            operation = { ...operation, placement: snappedPlacement(placement, adjustment.delta) };
            if (pose !== this.recordingGeometryPose()) throw new Error('Animation recording changed during the drag.');
        } else if (operation.type === 'resize' && operation.snap) {
            const preview = await this.previewResize(target, operation);
            this.snapping.validate(target, operation.snap);
            const { node, handle, dx, dy } = preview;
            operation = { ...operation, node, handle, dx, dy, snap: undefined };
            if (pose !== this.recordingGeometryPose()) throw new Error('Animation recording changed during the resize.');
        }
        const geometry = pose && recordedGeometry(operation);
        if (pose && geometry) {
            if (!('frame' in operation) || operation.frame !== frame.token) throw new Error('Read the current recording frame.');
            const targets = geometry.action === 'move' ? geometry.nodes : [geometry.node];
            await this.studioEdit(frame.token, 'designer_record_geometry', { ...geometry, program: pose.program, time: pose.time },
                targets, signal ?? AbortSignal.timeout(15_000), validate);
            return;
        }
        await this.commitEdit(frame, client => client.edit(frame.source, operation, frame.token), signal, validate);
    }

    private recordingGeometryPose(): TimelinePreviewPose | undefined {
        if (this.animationRecordingTime !== undefined && this.supportsTimelineGeometryRecording) return this.timelinePose;
    }

    private async commitEdit(frame: Frame, plan: (client: NativePreview) => Promise<EditResult>, signal?: AbortSignal, validate?: () => void): Promise<EditResult> {
        signal?.throwIfAborted();
        if (this.editing) throw new Error('A designer edit is already in progress.');
        this.editing = true;
        this.changed.fire();
        try {
            const client = await this.getClient();
            const result = await plan(client);
            const { edits, selection, timelinePose, timelineProgramKey, retainedSelection, moduleEdit, layerChange } = result;
            signal?.throwIfAborted();
            if (this.currentToken !== frame.token) throw new Error('The preview changed. Use the current preview to retry.');
            validate?.();
            if (moduleEdit) {
                const { document, version } = await this.checkRustModule(frame.token, moduleEdit.uri);
                signal?.throwIfAborted();
                validate?.();
                await applyTextEdits(document, version, moduleEdit.edits);
                moduleEdit.version = document.version;
                return result;
            }
            const previous = this.timelinePose;
            const previousProgram = this.timelineProgramSelection;
            if (timelinePose) this.timelinePose = timelinePose;
            if (timelineProgramKey && edits.length) {
                this.timelineProgramSelection = { version: frame.version + 1, key: timelineProgramKey };
            }
            if (retainedSelection?.length && edits.length) {
                this.retainedSelection = { version: frame.version + 1, ranges: retainedSelection };
            }
            let clearLayerPlan: (() => void) | undefined;
            try {
                if (layerChange && edits.length) clearLayerPlan = this.layers.history.prepare(layerChange, edits);
                await applyTextEdits(this.document, frame.version, edits);
            } catch (error) {
                if (this.timelinePose === timelinePose) this.timelinePose = previous;
                this.timelineProgramSelection = previousProgram;
                this.retainedSelection = undefined;
                throw error;
            } finally {
                clearLayerPlan?.();
            }
            if (selection && edits.length && this.document.version === frame.version + 1) {
                const range = asRange(selection);
                this.caret = range.start;
                await this.reveal(range);
            }
            return result;
        } finally {
            this.editing = false;
            this.changed.fire();
        }
    }

    private retainLayerSelection(version: number, { block, positions }: LayerTransfer): void {
        const frame = this.frame;
        const active = frame?.snapshot.activeBlock;
        if (frame?.version !== version - 1 || this.retainedSelection?.version === version
            || (active ? !block || !asRange(active).start.isEqual(asRange(block).start) : block !== null)) return;
        const starts = this.selection.flatMap(id => {
            const start = frame.snapshot.nodes.find(node => node.id === id)?.layerStart;
            return start === undefined ? [] : positions.get(start) ?? [];
        });
        if (starts.length) this.retainedSelection = { version, starts };
    }

    private async reveal(range: vscode.Range, openSource = false, preserveView = false): Promise<void> {
        const version = this.document.version;
        let editor = vscode.window.visibleTextEditors.find(editor => editor.document === this.document);
        if (!editor && openSource) {
            editor = await vscode.window.showTextDocument(this.document, {
                viewColumn: this.panel.viewColumn === vscode.ViewColumn.One ? vscode.ViewColumn.Beside : vscode.ViewColumn.One,
                preserveFocus: true,
            });
        }
        if (!editor || this.disposed || version !== this.document.version) return;
        const selection = new vscode.Selection(range.start, range.end);
        if (editor.selections.length !== 1 || !editor.selection.isEqual(selection)) {
            if (this.language === 'rust') {
                let revealed = this.revealedSelections.get(editor);
                if (revealed?.version !== version) {
                    revealed = { version, selections: [] };
                    this.revealedSelections.set(editor, revealed);
                }
                revealed.selections.push(selection);
            }
            editor.selection = selection;
        }
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
        if (!preserveView) this.updateCaret(range.start, true);
    }

    private syncCanvas(): void {
        const guides = this.canvasGuides;
        this.rememberView();
        this.showRecording();
        const frame = this.frame;
        if (frame && this.currentToken !== undefined) {
            const anchor = frame.snapshot.activeBlock?.start;
            const sourceKey = `${frame.version}:${anchor?.line ?? 0}:${anchor?.character ?? 0}`;
            this.post({ type: 'snapshot', frame: { session: this.session, token: frame.token }, sourceKey, snapshot: frame.canvas, viewport: frame.viewport,
                selected: this.selected, selection: this.selection, runtime: this.runtime, guides });
        } else if (this.problem) {
            this.post({ type: 'error', message: this.problem });
        } else {
            this.post({ type: 'pending' });
        }
    }

    private guideAnchor(snapshot: Snapshot, definition?: number | null): GuideAnchor | undefined {
        if (!snapshot.image || snapshot.renderError) return;
        if (snapshot.activeBlock) return { kind: 'rsx', start: snapshot.activeBlock.start };
        if (this.language !== 'xaml') return;
        if (definition == null) return { kind: 'document' };
        const range = snapshot.nodes.find(node => node.id === definition)?.nameRange;
        if (range) return { kind: 'definition', start: range.start };
    }

    private showRecording(): void {
        this.post({ type: 'recording', time: this.recordingGeometryPose()?.time });
    }

    private rememberView(): void {
        const block = this.frame?.snapshot.activeBlock;
        const caret = this.blockOffset !== undefined && block && !asRange(block).contains(this.caret)
            ? this.document.positionAt(this.blockOffset) : this.document.validatePosition(this.caret);
        this.post({ type: 'viewState', state: { version: 1, uri: this.document.uri.toString(),
            caret: { line: caret.line, character: caret.character }, viewport: this.viewport, designData: this.designData,
            layerViews: this.layers.saved, guideViews: this.guides.saved } });
    }

    private async getClient(): Promise<NativePreview> {
        if (this.disposed) throw new Error('The designer was closed.');
        const existing = this.client;
        if (existing) {
            const client = await existing.catch(() => undefined);
            if (client && !client.closed) return client;
            if (this.client !== existing) return this.getClient();
        }
        if (this.disposed) throw new Error('The designer was closed.');
        this.client = NativePreview.start(this.context, this.output);
        return this.client;
    }

    private stopClient(): void {
        void this.client?.then(client => client.dispose(), () => {});
        this.client = undefined;
    }

    post(message: ToDesigner): void {
        if (!this.disposed && this.canvasReady) void this.panel.webview.postMessage(message);
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true;
        this.changed.fire();
        clearTimeout(this.timer);
        clearTimeout(this.sourceCloseTimer);
        this.stopClient();
        this.canvasRequests.dispose();
        this.dependencies.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}

const emptyNodes: readonly Snapshot['nodes'][number][] = [];

function selectionAt(snapshot: Snapshot, caret: vscode.Position): number | undefined {
    // The host emits a preorder outline. The last enclosing element is the deepest.
    return snapshot.nodes.findLast(node => asRange(node.range).contains(caret))?.id;
}
