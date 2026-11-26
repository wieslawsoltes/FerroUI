import { createHash } from 'node:crypto';
import type * as vscode from 'vscode';
import { LayerHistory, type LayerTransfer } from './layerHistory';
import type { Language, Position, Range } from './protocol';
import { asPosition, offsetAfterChanges } from './textDocument';

interface Locator { start: number; anchor?: unknown }
interface Isolation { scope: Locator | null; roots: Locator[] }
export interface LayerViewState {
    source_stamp: string;
    locked: Locator[];
    hidden: Locator[];
    isolation: Isolation[];
}

export interface LayerChange {
    beforeStamp: string;
    afterStamp: string;
    block: Range | null;
    positions: [number, number][];
}

interface Layers { start: number; end: number; state: LayerViewState }
export interface LayerViewsState { stamp?: string; entries: Layers[] }
export interface LayerViewInput { block?: Position; state: LayerViewState }
const documentScope = -1;

/** Retains local layer state by authored view. Live edits rebase Rust invocation ranges. */
export class LayerViews {
    private entries: Layers[] = [];
    private stamp?: { version: number; hash: string };
    readonly history: LayerHistory;

    constructor(private readonly document: vscode.TextDocument, private readonly language: Language, restored: unknown) {
        this.history = new LayerHistory(document);
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
        if (hasFlags(state)) this.entries.unshift({ start, end, state });
        this.entries = this.entries.slice(0, 24);
    }

    reset(block: Range | null): void {
        if (this.language === 'rust' && !block) return;
        this.history.clearDetached(block ? this.document.offsetAt(asPosition(block.start)) : documentScope);
    }

    sourceChanged(event: vscode.TextDocumentChangeEvent): LayerTransfer | undefined {
        const transfer = this.history.sourceChanged(event);
        const previous = transfer && this.entries.find(entry => entry.start === transfer.from.start);
        const changes = event.contentChanges;
        if (this.language === 'rust') {
            this.entries = this.entries.filter(entry => !changes.some(change => change.rangeOffset <= entry.start
                && change.rangeOffset + change.rangeLength > entry.start)).map(entry => ({
                ...entry, start: offsetAfterChanges(entry.start, changes), end: offsetAfterChanges(entry.end, changes),
            }));
        }
        if (transfer && (!previous || previous.state.source_stamp === transfer.from.stamp)) {
            const state = moveState(previous?.state, transfer);
            const { start, end } = transfer.to;
            this.entries = this.entries.filter(entry => entry.start !== start);
            if (hasFlags(state)) this.entries.unshift({ start, end, state });
            this.entries = this.entries.slice(0, 24);
        }
        return transfer;
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

function moveState(state: LayerViewState | undefined, { from, to, positions }: LayerTransfer): LayerViewState {
    const locate = (loc: Locator): Locator | undefined => {
        const start = positions.get(loc.start);
        // The new native frame captures fresh source anchors after applying the map.
        return start === undefined ? undefined : { start };
    };
    const mapped = (layers: Locator[]) => layers.flatMap(loc => locate(loc) ?? []);
    const omitted = (layers: Locator[]) => layers.filter(loc => !positions.has(loc.start));
    const hidden = state?.hidden ?? [];
    const locked = state?.locked ?? [];
    const detached = { hidden: omitted(hidden), locked: omitted(locked) };
    from.detached = detached.hidden.length || detached.locked.length ? detached : undefined;
    const isolation = (state?.isolation ?? []).flatMap(entry => {
        const scope = entry.scope && locate(entry.scope);
        if (entry.scope && !scope) return [];
        const roots = mapped(entry.roots);
        return roots.length ? [{ scope: scope ?? null, roots }] : [];
    });
    return { source_stamp: to.stamp, locked: [...mapped(locked), ...(to.detached?.locked ?? [])],
        hidden: [...mapped(hidden), ...(to.detached?.hidden ?? [])], isolation };
}

function hasFlags(state: LayerViewState): boolean {
    return !!(state.locked.length || state.hidden.length || state.isolation.length);
}

function isLayerView(value: unknown): value is LayerViewState {
    if (!value || typeof value !== 'object') return false;
    const { source_stamp, locked, hidden, isolation } = value as Partial<LayerViewState>;
    return typeof source_stamp === 'string' && /^[0-9a-f]{16}$/.test(source_stamp)
        && locators(locked) && locators(hidden) && Array.isArray(isolation) && isolation.length <= 4096
        && isolation.every(entry => entry && (entry.scope === null || locator(entry.scope)) && locators(entry.roots));
}

function locators(items: unknown): items is Locator[] {
    return Array.isArray(items) && items.length <= 4096 && items.every(locator);
}

function locator(value: unknown): value is Locator {
    if (!value || typeof value !== 'object') return false;
    const { start } = value as Locator;
    return Number.isSafeInteger(start) && start >= 0;
}
