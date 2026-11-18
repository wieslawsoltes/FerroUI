import * as vscode from 'vscode';
import { NativePreview } from './nativePreview';
import { DEFAULT_VIEWPORT, MAX_VIEWPORT_SIZE, MIN_VIEWPORT_SIZE, errorMessage, type Asset, type CanvasSnapshot, type DragMessage, type DropHint, type FrameTarget, type FromDesigner, type Language, type NodeOperation, type NodeTarget, type Operation, type Placement, type Snapshot, type Source, type ToDesigner, type Viewport } from './protocol';
import { applyTextEdits, asPosition, asRange, sourceOf } from './textDocument';
import { designerHtml } from './view';

interface Frame {
    token: number;
    version: number;
    viewport: Viewport;
    source: Source;
    snapshot: Snapshot;
    canvas: CanvasSnapshot;
}

export class DesignerPanel implements vscode.Disposable {
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
    private viewport = DEFAULT_VIEWPORT;
    private inspected = false;
    private problem?: string;
    private caret: vscode.Position;
    private selected?: number;
    private blockOffset?: number;
    assets: readonly Asset[] = [];

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly output: vscode.OutputChannel,
        readonly panel: vscode.WebviewPanel,
        readonly document: vscode.TextDocument,
        readonly session: number,
        readonly language: Language,
        caret: vscode.Position,
        onClose: () => void,
        private readonly onDrag: (message: DragMessage | Extract<FromDesigner, { type: 'previewDrop' }>) => void,
    ) {
        this.caret = caret;
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
            vscode.workspace.onDidChangeTextDocument(event => {
                if (event.document !== document || !event.contentChanges.length) return;
                if (this.blockOffset !== undefined) {
                    this.blockOffset = offsetAfterChanges(this.blockOffset, event.contentChanges);
                }
                this.schedule();
            }),
            vscode.workspace.onDidCloseTextDocument(closed => { if (closed === document) this.dispose(); }),
            vscode.window.onDidChangeTextEditorSelection(event => {
                if (event.textEditor.document === document && event.selections[0]) this.updateCaret(event.selections[0].active);
            }),
            vscode.workspace.onDidChangeConfiguration(event => {
                if (event.affectsConfiguration('ferroui.preview.path')) { this.stopClient(); this.schedule(0); }
            }),
        );
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

    get nodes(): readonly Snapshot['nodes'][number][] { return this.frame?.snapshot.nodes ?? emptyNodes; }
    get selectedNode(): Snapshot['nodes'][number] | undefined { return this.nodes.find(node => node.id === this.selected); }
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

    async select(target: NodeTarget, openSource = false): Promise<void> {
        if (!this.canUse(target)) return;
        const node = this.nodes.find(node => node.id === target.node);
        if (!node) return;
        const range = asRange(node.nameRange);
        this.caret = range.start;
        this.setSelection(node.id);
        await this.reveal(range, openSource);
    }

    async editNode(target: NodeTarget, operation: NodeOperation): Promise<void> {
        if (!this.canUse(target) || operation.node !== target.node || this.selected !== target.node) {
            throw new Error('The selection changed. Use the current Properties view to retry.');
        }
        await this.edit(this.frame!, operation);
    }

    canUse(target: FrameTarget): boolean {
        const token = this.currentToken;
        return token !== undefined && target?.session === this.session && target.token === token;
    }

    async place(target: FrameTarget, placement: Placement): Promise<void> {
        if (!this.canUse(target) || !this.frame?.snapshot.image) throw new Error('Wait for the current layout before placing controls.');
        await this.edit(this.frame, { type: 'place', frame: target.token, placement });
    }

    async previewPlacement(target: FrameTarget, placement: Placement): Promise<DropHint> {
        if (!this.canUse(target)) throw new Error('The design changed during the drag.');
        return (await this.getClient()).placement(target.token, placement);
    }

    private setSelection(id: number | undefined): void {
        if (id === this.selected) return;
        this.selected = id;
        const token = this.currentToken;
        if (token !== undefined) this.post({ type: 'selection', token, selected: id });
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
        if (this.currentToken !== undefined) this.setSelection(selectionAt(frame.snapshot, caret) ?? this.selected);
    }

    private schedule(delay = 180): void {
        this.generation++;
        this.pending = true;
        this.problem = undefined;
        this.post({ type: 'pending' });
        this.changed.fire();
        clearTimeout(this.timer);
        this.timer = setTimeout(() => { this.timer = undefined; void this.render(); }, delay);
    }

    private async render(): Promise<void> {
        if (this.disposed || !(this.inspected || this.panel.visible && this.ready) || !this.pending || this.rendering || this.timer) return;
        this.rendering = true;
        this.pending = false;
        const token = this.generation;
        const version = this.document.version;
        const viewport = this.viewport;
        const caret = this.language === 'rust' && this.blockOffset !== undefined
            ? this.document.positionAt(this.blockOffset) : this.document.validatePosition(this.caret);
        const input = sourceOf(this.document, this.language, caret);
        try {
            const client = await this.getClient();
            const snapshot = await client.render(input, viewport, token);
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            const anchor = snapshot.activeBlock?.start;
            const previousAnchor = this.frame?.snapshot.activeBlock?.start;
            const sameView = this.frame?.version === version && previousAnchor?.line === anchor?.line
                && previousAnchor?.character === anchor?.character;
            this.assets = client.assets.filter(asset => this.language === 'xaml' || asset.rust);
            const nodes = snapshot.nodes.map(({ id, label, name, bounds, canMove }) => ({ id, label, name, bounds, canMove }));
            this.frame = { token, version, viewport, source: input, snapshot, canvas: { ...snapshot, nodes } };
            this.selected = selectionAt(snapshot, this.caret) ?? (sameView ? this.selected : undefined) ?? snapshot.nodes[0]?.id;
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
            this.rendering = false;
            void this.render();
        }
    }

    private async message(message: FromDesigner): Promise<void> {
        if (this.disposed || !message) return;
        if (message.type === 'ready') {
            this.ready = true;
            this.syncCanvas();
            void this.render();
            return;
        }
        if (message.type === 'refresh') {
            this.schedule(0);
            return;
        }
        if (message.type === 'dock') {
            const commands = { outline: 'ferroui.showOutline', properties: 'ferroui.showProperties', toolbox: 'ferroui.showToolbox' } as const;
            await vscode.commands.executeCommand(commands[message.view], this.session);
            return;
        }
        if (message.type === 'beginDrag' || message.type === 'endDrag' || message.type === 'drop' || message.type === 'previewDrop') {
            this.onDrag(message);
            return;
        }
        if (message.type === 'viewport') {
            const value = message.viewport;
            if (!value || [value.width, value.height].some(n => !Number.isInteger(n) || n < MIN_VIEWPORT_SIZE || n > MAX_VIEWPORT_SIZE)
                || typeof value.dark !== 'boolean') return;
            this.viewport = value;
            this.schedule(0);
            return;
        }
        const frame = this.frame;
        if (!frame || !('token' in message) || this.currentToken === undefined || this.currentToken !== message.token) {
            if (message.type === 'apply') this.post({ type: 'edit', error: 'The source changed. Use the current preview to retry.' });
            return;
        }
        try {
            if (message.type === 'select') {
                await this.select({ session: this.session, token: message.token, node: message.id }, message.openSource);
            } else if (message.type === 'block') {
                const block = frame.snapshot.blocks[message.index];
                if (block) {
                    this.caret = asPosition(block.start);
                    this.blockOffset = this.document.offsetAt(this.caret);
                    this.schedule(0);
                    await this.reveal(new vscode.Range(this.caret, this.caret));
                }
            } else if (message.type === 'apply') {
                await this.edit(frame, { type: 'applyCounterpart', source: message.source });
                this.post({ type: 'edit' });
            }
        } catch (error) {
            this.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private async edit(frame: Frame, operation: Operation): Promise<void> {
        if (this.editing) throw new Error('A designer edit is already in progress.');
        this.editing = true;
        this.changed.fire();
        try {
            const edits = await (await this.getClient()).edit(frame.source, operation);
            if (this.currentToken !== frame.token) throw new Error('The preview changed. Use the current preview to retry.');
            await applyTextEdits(this.document, frame.version, edits);
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
        const frame = this.frame;
        if (frame && this.currentToken !== undefined) {
            const anchor = frame.snapshot.activeBlock?.start;
            const sourceKey = `${frame.version}:${anchor?.line ?? 0}:${anchor?.character ?? 0}`;
            this.post({ type: 'snapshot', frame: { session: this.session, token: frame.token }, sourceKey, snapshot: frame.canvas, viewport: frame.viewport, selected: this.selected });
        } else if (this.problem) {
            this.post({ type: 'error', message: this.problem });
        } else {
            this.post({ type: 'pending' });
        }
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
        if (!this.disposed && this.ready && this.panel.visible) void this.panel.webview.postMessage(message);
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true;
        clearTimeout(this.timer);
        this.stopClient();
        this.panel.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}

const emptyNodes: readonly Snapshot['nodes'][number][] = [];

function selectionAt(snapshot: Snapshot, caret: vscode.Position): number | undefined {
    // The host emits a preorder outline. The last enclosing element is the deepest.
    return snapshot.nodes.findLast(node => asRange(node.range).contains(caret))?.id;
}

function offsetAfterChanges(offset: number, changes: readonly vscode.TextDocumentContentChangeEvent[]): number {
    let shift = 0;
    for (const change of [...changes].sort((a, b) => a.rangeOffset - b.rangeOffset)) {
        if (change.rangeOffset > offset) break;
        if (offset < change.rangeOffset + change.rangeLength) return change.rangeOffset + shift;
        shift += change.text.length - change.rangeLength;
    }
    return offset + shift;
}
