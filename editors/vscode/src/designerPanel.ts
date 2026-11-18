import * as vscode from 'vscode';
import { recordedGeometry, recordedMove } from './animationGeometry';
import type { BuiltProject, BuiltProjects } from './builtProjects';
import { CanvasRequests } from './canvasRequests';
import { DesignerDependencies, dirtyXamlSources } from './designerDependencies';
import { LayerViews } from './layerViews';
import { NativePreview } from './nativePreview';
import { DEFAULT_VIEWPORT, errorMessage, isViewport, type Asset, type CanvasSnapshot, type DragMessage, type DropHint, type FrameTarget, type FromDesigner, type Language, type NodeOperation, type NodeTarget, type Operation, type Placement, type Snapshot, type Source, type ToDesigner, type Viewport } from './protocol';
import { applyTextEdits, asPosition, asRange, offsetAfterChanges, sourceOf } from './textDocument';
import type { TimelinePreviewPose } from './timelineProtocol';
import { designerHtml, webviewOptions } from './view';
import type { EditResult, Inspection, InspectionTarget, Range, ResourceRow, RuntimeRow, RuntimeTreeMode } from './protocol';
import type { CanvasAction, CanvasFocusOrder, CanvasInteraction, CanvasPresentation, DesignDataPresentation, DesignerViewState } from './protocol';

const mappedStudioEdits = new Set(['designer_selection_layout_edit', 'designer_property', 'designer_grid_track_edit']);
export type DesignerCanvasState = Pick<DesignerViewState, 'viewport' | 'designData' | 'layerViews'>;

interface Frame {
    token: number;
    version: number;
    viewport: Viewport;
    source: Source;
    snapshot: Snapshot;
    canvas: CanvasSnapshot;
    timelineRecording: boolean;
    timelineGeometryRecording: boolean;
    timelineKeyframeDrafts: boolean;
}

type PropertyEdit = Extract<NodeOperation, { type: 'setProperty' | 'resetProperty' }>;

export class DesignerPanel implements vscode.Disposable {
    private readonly canvasRequests = new CanvasRequests();
    private readonly dependencies = new DesignerDependencies();
    private readonly layers: LayerViews;
    private readonly subscriptions: vscode.Disposable[] = [];
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private client?: Promise<NativePreview>;
    private frame?: Frame;
    private disposed = false;
    private ready = false;
    private timer?: ReturnType<typeof setTimeout>;
    private generation = 0;
    private pending = true;
    private rendering = false;
    private editing = false;
    private viewport: Viewport;
    private designData?: DesignDataPresentation;
    private timelinePose?: TimelinePreviewPose;
    private animationRecording = false;
    private retainedSelection?: { version: number; ranges: Range[] };
    private inspected = false;
    private readers = 0;
    private problem?: string;
    private caret: vscode.Position;
    private selected?: number;
    private selection: number[] = [];
    private runtime?: RuntimeRow;
    private blockOffset?: number;
    private project?: BuiltProject;
    assets: readonly Asset[] = [];

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly output: vscode.OutputChannel,
        projects: BuiltProjects,
        readonly panel: vscode.WebviewPanel,
        readonly document: vscode.TextDocument,
        readonly session: number,
        readonly language: Language,
        caret: vscode.Position,
        onClose: () => void,
        private readonly onDrag: (message: DragMessage | Extract<FromDesigner, { type: 'previewDrop' }>) => void,
        restored?: DesignerCanvasState,
    ) {
        this.caret = caret;
        this.viewport = restored?.viewport ?? DEFAULT_VIEWPORT;
        this.designData = restored?.designData;
        this.layers = new LayerViews(document, language, restored?.layerViews);
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
                const project = projects.forDocument(document.uri);
                if (project === this.project) return;
                this.project = project;
                this.schedule(0);
            }),
            vscode.workspace.onDidChangeTextDocument(event => {
                if (!event.contentChanges.length) return;
                if (event.document !== document) { this.dependencyChanged(event.document.uri); return; }
                if (this.blockOffset !== undefined) {
                    this.blockOffset = offsetAfterChanges(this.blockOffset, event.contentChanges);
                }
                this.layers.sourceChanged(event.contentChanges);
                this.schedule();
            }),
            vscode.workspace.onDidCloseTextDocument(closed => {
                if (closed === document) panel.dispose();
                else if (closed.isDirty) this.dependencyChanged(closed.uri);
            }),
            vscode.window.onDidChangeTextEditorSelection(event => {
                if (event.textEditor.document === document && event.selections[0]) this.updateCaret(event.selections[0].active);
            }),
            vscode.workspace.onDidChangeConfiguration(event => {
                if (event.affectsConfiguration('ferroui.preview.path')) { this.stopClient(); this.schedule(0); }
            }),
        );
        panel.webview.options = webviewOptions(context.extensionUri);
        panel.webview.html = designerHtml(panel.webview, context.extensionUri, language);
    }

    show(caret: vscode.Position): void {
        this.updateCaret(caret);
        if (!this.frame || this.frame.version !== this.document.version) {
            this.blockOffset = this.document.offsetAt(caret);
            this.schedule(0);
        }
        this.panel.reveal(undefined, true);
    }

    get currentToken(): number | undefined {
        const frame = this.frame;
        if (frame && !this.disposed && frame.token === this.generation && frame.version === this.document.version) return frame.token;
        return undefined;
    }

    get selectedIds(): readonly number[] { return this.runtime ? [] : this.selection; }

    get nodes(): readonly Snapshot['nodes'][number][] { return this.frame?.snapshot.nodes ?? emptyNodes; }
    get selectedNode(): Snapshot['nodes'][number] | undefined { return this.runtime ? undefined : this.nodes.find(node => node.id === this.selected); }
    get runtimeSelection(): RuntimeRow | undefined { return this.runtime; }
    get dimensions(): Viewport { return this.viewport; }
    get authoredTimelinePose(): TimelinePreviewPose | undefined { return this.timelinePose; }
    get supportsTimelineRecording(): boolean { return Boolean(this.language === 'xaml' && this.frame?.timelineRecording); }
    get supportsTimelineGeometryRecording(): boolean { return Boolean(this.supportsTimelineRecording && this.frame?.timelineGeometryRecording); }
    get supportsTimelineKeyframeDrafts(): boolean { return Boolean(this.language === 'xaml' && this.frame?.timelineKeyframeDrafts); }
    get animationRecordingTime(): number | undefined { return this.animationRecording ? this.timelinePose?.time : undefined; }
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

    async revealSource(frame: FrameTarget, range: Range): Promise<void> {
        if (this.canUse(frame)) await this.reveal(this.document.validateRange(asRange(range)), true);
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
            return this.studioEdit(target.token, 'designer_property', { action: operation.type === 'setProperty' ? 'set' : 'reset',
                property: operation.name, nodes: target.selection, ...(operation.type === 'setProperty' ? { value: operation.value } : {}) },
            target.selection, AbortSignal.timeout(15_000));
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

    async place(target: FrameTarget, placement: Placement): Promise<void> {
        if (!this.canUse(target) || !this.frame?.snapshot.image) throw new Error('Wait for the current layout before placing controls.');
        await this.edit(this.frame, { type: 'place', frame: target.token, placement });
    }

    async previewPlacement(target: FrameTarget, placement: Placement): Promise<DropHint> {
        if (!this.canUse(target)) throw new Error('The design changed during the drag.');
        const recording = this.recordingGeometryPose() !== undefined && Boolean(recordedMove(placement));
        const client = await this.getClient();
        if (!this.canUse(target)) throw new Error('The design changed during the drag.');
        return client.placement(target.token, placement, recording);
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
        const { tools, xamlEdits, presentations } = await (await this.getClient()).studioTools();
        return { tools, presentations: presentations ?? [], edits: this.language === 'xaml' ? xamlEdits : xamlEdits.filter(tool =>
            mappedStudioEdits.has((tool as { name: string }).name)) };
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
        validate?: () => void): Promise<void> {
        if (this.language === 'rust' && !mappedStudioEdits.has(name)) {
            throw new Error('This Studio operation requires XAML or a mapped RSX layout operation.');
        }
        if (this.editing) throw new Error('A designer edit is already in progress.');
        const frame = this.frame;
        if (!frame || this.currentToken !== token) throw new Error('Read the current designer revision before editing.');
        const targets = this.studioSelection(selection);
        try {
            await this.commitEdit(frame, client => client.studioEdit(frame.source, token, name, args, targets), signal, validate);
        } finally {
            if (this.currentToken === token) this.schedule(0);
        }
    }

    async studioPresent(token: number, name: string, args: object, selection: number[] | undefined, signal: AbortSignal): Promise<CanvasPresentation | undefined> {
        return this.present(token, signal, (client, next) => client.studioPresent(token, next, name, args, this.studioSelection(selection)), true,
            presentation => {
                if (name === 'designer_design_data_edit') this.rememberDesignData(args);
                if (name === 'designer_program') this.timelinePose = presentation.timelinePosition?.pose ?? undefined;
            });
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
            const snapshot = { ...frame.snapshot, nodes, image: presentation.image, notices: presentation.notices, timelinePose: this.timelinePose };
            const canvasNodes = nodes.map(({ id, label, name, bounds, canMove, selectable }) => ({ id, label, name, bounds, canMove, selectable }));
            this.frame = { ...frame, token: next, snapshot, canvas: { ...frame.canvas, ...snapshot, nodes: canvasNodes } };
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

    private updateCaret(caret: vscode.Position): void {
        this.caret = caret;
        const frame = this.frame;
        if (!frame || frame.version !== this.document.version) return;
        const block = frame.snapshot.blocks.reduce<vscode.Range | undefined>((best, span) => {
            const range = asRange(span);
            return range.contains(caret) && (!best || best.contains(range)) ? range : best;
        }, undefined);
        if (block) {
            this.blockOffset = this.document.offsetAt(block.start);
            if (!frame.snapshot.activeBlock || !asRange(frame.snapshot.activeBlock).isEqual(block)) {
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
        if (!this.disposed && this.dependencies.change(uri)) this.schedule();
    }

    private async render(): Promise<void> {
        if (this.disposed || !(this.inspected || this.readers || this.canvasReady) || !this.pending || this.rendering || this.timer) return;
        this.rendering = true;
        this.pending = false;
        const token = this.generation;
        const version = this.document.version;
        const viewport = this.viewport;
        const caret = this.language === 'rust' && this.blockOffset !== undefined
            ? this.document.positionAt(this.blockOffset) : this.document.validatePosition(this.caret);
        const source = sourceOf(this.document, this.language, caret);
        const project = this.project;
        this.dependencies.start();
        try {
            const client = await this.getClient();
            const input = await client.inProject(source, project);
            const folder = vscode.workspace.getWorkspaceFolder(this.document.uri);
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            const snapshot = await client.render(input, viewport, token, {
                workspaceRoot: folder?.uri.scheme === 'file' ? folder.uri.fsPath : undefined,
                localSources: dirtyXamlSources([this.document.uri], this.document),
                designData: this.designData, layerView: this.layers.input(caret), timelinePose: this.timelinePose,
            });
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            if (this.dependencies.accept(snapshot.dependencies ?? [])) { this.schedule(0); return; }
            const anchor = snapshot.activeBlock?.start;
            const previousAnchor = this.frame?.snapshot.activeBlock?.start;
            const sameView = this.frame?.version === version && previousAnchor?.line === anchor?.line
                && previousAnchor?.character === anchor?.character;
            this.assets = [...client.assets, ...(snapshot.projectAssets ?? [])].filter(asset => this.language === 'xaml' || asset.rust);
            const nodes = snapshot.nodes.map(({ id, label, name, bounds, canMove, selectable }) => ({ id, label, name, bounds, canMove, selectable }));
            if (snapshot.image) this.timelinePose = snapshot.timelinePose;
            this.frame = { token, version, viewport, source: input, snapshot, timelineRecording: Boolean(client.capabilities.timelineRecording),
                timelineGeometryRecording: Boolean(client.capabilities.timelineGeometryRecording),
                timelineKeyframeDrafts: Boolean(client.capabilities.timelineKeyframeDrafts),
                canvas: { ...snapshot, nodes, interactive: Boolean(client.capabilities.canvasInteraction && snapshot.image),
                    focusOrderAvailable: Boolean(client.capabilities.canvasFocusOrder && snapshot.image) } };
            this.runtime = undefined;
            const retained = this.retainedSelection?.version === version && snapshot.image
                ? this.retainedSelection.ranges.flatMap(range => {
                    const node = snapshot.nodes.find(node => asRange(node.nameRange).isEqual(asRange(range)));
                    return node ? [node.id] : [];
                }) : [];
            this.retainedSelection = undefined;
            if (retained.length) {
                this.selection = retained;
                this.selected = retained.at(-1);
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
        if (message.type === 'refresh') {
            this.schedule(0);
            return;
        }
        if (message.type === 'dock') {
            const commands = { outline: 'ferroui.showOutline', properties: 'ferroui.showProperties', toolbox: 'ferroui.showToolbox',
                events: 'ferroui.showEvents', resources: 'ferroui.showResources', assets: 'ferroui.showAssets',
                studio: 'ferroui.showStudio', timeline: 'ferroui.showTimeline', data: 'ferroui.showData', library: 'ferroui.showLibrary' } as const;
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
            if (message.type === 'focusOrder') {
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
                const { node, handle, dx, dy } = message;
                await this.edit(frame, { type: 'resize', frame: message.token, node, handle, dx, dy });
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
        const geometry = pose && recordedGeometry(operation);
        if (pose && geometry) {
            if (!('frame' in operation) || operation.frame !== frame.token) throw new Error('Read the current recording frame.');
            const targets = geometry.action === 'move' ? geometry.nodes : [geometry.node];
            return this.studioEdit(frame.token, 'designer_record_geometry', { ...geometry, program: pose.program, time: pose.time },
                targets, signal ?? AbortSignal.timeout(15_000), validate);
        }
        return this.commitEdit(frame, client => client.edit(frame.source, operation), signal, validate);
    }

    private recordingGeometryPose(): TimelinePreviewPose | undefined {
        if (this.animationRecording && this.supportsTimelineGeometryRecording) return this.timelinePose;
    }

    private async commitEdit(frame: Frame, plan: (client: NativePreview) => Promise<EditResult>, signal?: AbortSignal, validate?: () => void): Promise<void> {
        signal?.throwIfAborted();
        if (this.editing) throw new Error('A designer edit is already in progress.');
        this.editing = true;
        this.changed.fire();
        try {
            const { edits, selection, timelinePose, retainedSelection } = await plan(await this.getClient());
            signal?.throwIfAborted();
            if (this.currentToken !== frame.token) throw new Error('The preview changed. Use the current preview to retry.');
            validate?.();
            const previous = this.timelinePose;
            if (timelinePose) this.timelinePose = timelinePose;
            if (retainedSelection?.length && edits.length) {
                this.retainedSelection = { version: frame.version + 1, ranges: retainedSelection };
            }
            try {
                await applyTextEdits(this.document, frame.version, edits);
            } catch (error) {
                if (this.timelinePose === timelinePose) this.timelinePose = previous;
                this.retainedSelection = undefined;
                throw error;
            }
            if (selection && edits.length && this.document.version === frame.version + 1) {
                const range = asRange(selection);
                this.caret = range.start;
                await this.reveal(range);
            }
        } finally {
            this.editing = false;
            this.changed.fire();
        }
    }

    private async reveal(range: vscode.Range, openSource = false): Promise<void> {
        const version = this.document.version;
        let editor = vscode.window.visibleTextEditors.find(editor => editor.document === this.document);
        if (!editor && openSource) {
            editor = await vscode.window.showTextDocument(this.document, {
                viewColumn: this.panel.viewColumn === vscode.ViewColumn.One ? vscode.ViewColumn.Beside : vscode.ViewColumn.One,
                preserveFocus: true,
            });
        }
        if (!editor || this.disposed || version !== this.document.version) return;
        editor.selection = new vscode.Selection(range.start, range.end);
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
        this.updateCaret(range.start);
    }

    private syncCanvas(): void {
        this.rememberView();
        this.showRecording();
        const frame = this.frame;
        if (frame && this.currentToken !== undefined) {
            const anchor = frame.snapshot.activeBlock?.start;
            const sourceKey = `${frame.version}:${anchor?.line ?? 0}:${anchor?.character ?? 0}`;
            this.post({ type: 'snapshot', frame: { session: this.session, token: frame.token }, sourceKey, snapshot: frame.canvas, viewport: frame.viewport,
                selected: this.selected, selection: this.selection, runtime: this.runtime });
        } else if (this.problem) {
            this.post({ type: 'error', message: this.problem });
        } else {
            this.post({ type: 'pending' });
        }
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
            layerViews: this.layers.saved } });
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
        this.stopClient();
        this.canvasRequests.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}

const emptyNodes: readonly Snapshot['nodes'][number][] = [];

function selectionAt(snapshot: Snapshot, caret: vscode.Position): number | undefined {
    // The host emits a preorder outline. The last enclosing element is the deepest.
    return snapshot.nodes.findLast(node => asRange(node.range).contains(caret))?.id;
}
