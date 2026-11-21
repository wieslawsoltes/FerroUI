import { createHash } from 'node:crypto';
import type * as vscode from 'vscode';
import type { Language, Position, Range } from './protocol';
import { asPosition, offsetAfterChanges } from './textDocument';

export interface LayerViewState {
    source_stamp: string;
    locked: unknown[];
    hidden: unknown[];
    isolation: unknown[];
}

interface Layers { start: number; end: number; state: LayerViewState }
export interface LayerViewsState { stamp?: string; entries: Layers[] }
export interface LayerViewInput { block?: Position; state: LayerViewState }
const documentScope = -1;

/** Retains local layer state by authored view. Live edits rebase Rust invocation ranges. */
export class LayerViews {
    private entries: Layers[] = [];
    private stamp?: { version: number; hash: string };

    constructor(private readonly document: vscode.TextDocument, private readonly language: Language, restored: unknown) {
        if (!restored || typeof restored !== 'object') return;
        const { stamp, entries } = restored as Partial<LayerViewsState>;
        if (!Array.isArray(entries) || entries.length > 24 || (language === 'rust' && stamp !== this.sourceStamp())) return;
        this.entries = entries.filter(entry => entry && Number.isSafeInteger(entry.start) && Number.isSafeInteger(entry.end)
            && entry.start >= documentScope && entry.end >= entry.start && isLayerView(entry.state));
    }

    remember(state: LayerViewState, block: Range | null): void {
        if (this.language === 'rust' && !block) return;
        const start = block ? this.document.offsetAt(asPosition(block.start)) : documentScope;
        const end = block ? this.document.offsetAt(asPosition(block.end)) : documentScope;
        this.entries = this.entries.filter(entry => entry.start !== start);
        if (state.locked.length || state.hidden.length || state.isolation.length) this.entries.unshift({ start, end, state });
        this.entries = this.entries.slice(0, 24);
    }

    sourceChanged(changes: readonly vscode.TextDocumentContentChangeEvent[]): void {
        if (this.language === 'xaml') return;
        this.entries = this.entries.filter(entry => !changes.some(change => change.rangeOffset <= entry.start
            && change.rangeOffset + change.rangeLength > entry.start)).map(entry => ({
            ...entry, start: offsetAfterChanges(entry.start, changes), end: offsetAfterChanges(entry.end, changes),
        }));
    }

    input(caret: vscode.Position): LayerViewInput | undefined {
        if (this.language === 'xaml') {
            const saved = this.entries.find(entry => entry.start === documentScope);
            if (saved) return { state: saved.state };
            return;
        }
        const offset = this.document.offsetAt(caret);
        const saved = this.entries.filter(entry => entry.start <= offset && entry.end >= offset)
            .sort((a, b) => b.start - a.start)[0];
        if (!saved) return;
        const { line, character } = this.document.positionAt(saved.start);
        return { block: { line, character }, state: saved.state };
    }

    get saved(): LayerViewsState | undefined {
        if (this.entries.length) return { entries: this.entries, stamp: this.language === 'rust' ? this.sourceStamp() : undefined };
    }

    private sourceStamp(): string {
        if (this.stamp?.version !== this.document.version) {
            this.stamp = { version: this.document.version, hash: createHash('sha256').update(this.document.getText()).digest('hex') };
        }
        return this.stamp.hash;
    }
}

function isLayerView(value: unknown): value is LayerViewState {
    if (!value || typeof value !== 'object') return false;
    const { source_stamp, locked, hidden, isolation } = value as Partial<LayerViewState>;
    return typeof source_stamp === 'string' && /^[0-9a-f]{16}$/.test(source_stamp)
        && [locked, hidden, isolation].every(items => Array.isArray(items) && items.length <= 4096);
}
