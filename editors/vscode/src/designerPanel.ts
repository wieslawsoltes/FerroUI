import * as vscode from 'vscode';
import { NativePreview } from './nativePreview';
import { type FromDesigner, type Language, type Operation, type Snapshot, type Source, type ToDesigner, type Viewport, errorMessage } from './protocol';
import { applyTextEdits, asPosition, asRange, sourceOf } from './textDocument';
import { designerHtml } from './view';

interface Frame {
    token: number;
    version: number;
    source: Source;
    snapshot: Snapshot;
}

export class DesignerPanel implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[] = [];
    private client?: Promise<NativePreview>;
    private frame?: Frame;
    private disposed = false;
    private ready = false;
    private timer?: ReturnType<typeof setTimeout>;
    private generation = 0;
    private pending = true;
    private rendering = false;
    private editing = false;
    private viewport: Viewport = { width: 800, height: 520, dark: false };
    private caret: vscode.Position;
    private selected?: number;
    private blockOffset?: number;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly output: vscode.OutputChannel,
        private readonly panel: vscode.WebviewPanel,
        private readonly document: vscode.TextDocument,
        private readonly language: Language,
        caret: vscode.Position,
        onClose: () => void,
    ) {
        this.caret = caret;
        this.subscriptions.push(
            panel.onDidDispose(() => { this.dispose(); onClose(); }),
            panel.onDidChangeViewState(() => { if (panel.visible) void this.render(); }),
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
        this.selected = selectionAt(frame.snapshot, caret) ?? this.selected;
        if (frame.token === this.generation) this.post({ type: 'selection', token: frame.token, selected: this.selected });
    }

    private schedule(delay = 180): void {
        this.generation++;
        this.pending = true;
        if (this.ready) this.post({ type: 'pending' });
        clearTimeout(this.timer);
        this.timer = setTimeout(() => { this.timer = undefined; void this.render(); }, delay);
    }

    private async render(): Promise<void> {
        if (this.disposed || !this.panel.visible || !this.ready || !this.pending || this.rendering || this.timer) return;
        this.rendering = true;
        this.pending = false;
        const token = this.generation;
        const version = this.document.version;
        const caret = this.language === 'rust' && this.blockOffset !== undefined
            ? this.document.positionAt(this.blockOffset) : this.document.validatePosition(this.caret);
        const input = sourceOf(this.document, this.language, caret);
        try {
            const snapshot = await (await this.getClient()).render(input, this.viewport);
            if (this.disposed || token !== this.generation || version !== this.document.version) return;
            this.frame = { token, version, source: input, snapshot };
            this.selected = selectionAt(snapshot, this.caret) ?? snapshot.nodes[0]?.id;
            const anchor = snapshot.activeBlock?.start;
            if (anchor) this.blockOffset = this.document.offsetAt(asPosition(anchor));
            const sourceKey = `${version}:${anchor?.line ?? 0}:${anchor?.character ?? 0}`;
            this.post({ type: 'snapshot', token, sourceKey, snapshot, selected: this.selected });
        } catch (error) {
            if (token === this.generation) this.post({ type: 'error', message: errorMessage(error) });
        } finally {
            this.rendering = false;
            void this.render();
        }
    }

    private async message(message: FromDesigner): Promise<void> {
        if (this.disposed || !message) return;
        if (message.type === 'ready' || message.type === 'refresh') {
            this.ready = true;
            this.schedule(0);
            return;
        }
        if (message.type === 'viewport') {
            const value = message.viewport;
            if ([value.width, value.height].some(n => !Number.isInteger(n) || n < 64 || n > 1536) || typeof value.dark !== 'boolean') return;
            this.viewport = value;
            this.schedule(0);
            return;
        }
        const frame = this.frame;
        if (!frame || !('token' in message) || frame.token !== message.token || frame.token !== this.generation || frame.version !== this.document.version) {
            if (message.type === 'edit') this.post({ type: 'edit', error: 'The source changed. Use the current preview to retry.' });
            return;
        }
        try {
            if (message.type === 'select') {
                const node = frame.snapshot.nodes.find(node => node.id === message.id);
                if (node) await this.reveal(asRange(node.nameRange));
            } else if (message.type === 'block') {
                const block = frame.snapshot.blocks[message.index];
                if (block) {
                    this.caret = asPosition(block.start);
                    this.blockOffset = this.document.offsetAt(this.caret);
                    this.schedule(0);
                    await this.reveal(new vscode.Range(this.caret, this.caret));
                }
            } else if (message.type === 'edit') {
                await this.edit(frame, message.operation);
            }
        } catch (error) {
            this.post({ type: 'edit', error: errorMessage(error) });
        }
    }

    private async edit(frame: Frame, operation: Operation): Promise<void> {
        if (this.editing) throw new Error('A designer edit is already in progress.');
        this.editing = true;
        try {
            const edits = await (await this.getClient()).edit(frame.source, operation);
            if (this.disposed || frame.token !== this.generation) throw new Error('The preview changed. Use the current preview to retry.');
            await applyTextEdits(this.document, frame.version, edits);
            this.post({ type: 'edit' });
        } finally {
            this.editing = false;
        }
    }

    private async reveal(range: vscode.Range): Promise<void> {
        const visible = vscode.window.visibleTextEditors.find(editor => editor.document === this.document);
        const editor = visible ?? await vscode.window.showTextDocument(this.document, {
            viewColumn: vscode.ViewColumn.One, preserveFocus: true,
        });
        editor.selection = new vscode.Selection(range.start, range.end);
        editor.revealRange(range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
        this.updateCaret(range.start);
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

    private post(message: ToDesigner): void {
        if (!this.disposed) void this.panel.webview.postMessage(message);
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true;
        clearTimeout(this.timer);
        this.stopClient();
        this.panel.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

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
