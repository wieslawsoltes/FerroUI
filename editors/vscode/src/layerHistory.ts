import { createHash } from 'node:crypto';
import * as vscode from 'vscode';
import type { LayerChange, LayerViewState } from './layerViews';
import type { Range, TextEdit } from './protocol';
import { asPosition, exactRange, offsetAfterChanges } from './textDocument';

type Change = Pick<vscode.TextDocumentContentChangeEvent, 'rangeOffset' | 'rangeLength' | 'text'>;
type LayerFlags = Pick<LayerViewState, 'hidden' | 'locked'>;
interface Scope { start: number; end: number }
interface HistorySide extends Scope {
    stamp: string;
    source: string;
    detached?: LayerFlags;
}
interface Step {
    scope: symbol;
    before: HistorySide;
    after: HistorySide;
    positions: Uint32Array;
    block: Range | null;
    forwardSignature?: string;
    reverseSignature?: string;
}
interface SourceChange { beforeSource: string; afterSource: string }
type Entry = Step | SourceChange;
interface Pending {
    scope: symbol;
    version: number;
    text: string;
    change: LayerChange;
    before: HistorySide;
    expectedSource: string;
    signature?: string;
}
export interface LayerTransfer {
    from: HistorySide;
    to: HistorySide;
    positions: ReadonlyMap<number, number>;
    block: Range | null;
}

const historyLimit = 256;
const documentScope = -1;

/** Follows VS Code's text history. Layer changes themselves stay out of that history. */
export class LayerHistory {
    private pending?: Pending;
    private source?: string;
    private readonly undo: Entry[] = [];
    private readonly redo: Entry[] = [];
    private scopes = new Map<number, symbol>();

    constructor(private readonly document: vscode.TextDocument) {}

    clearDetached(start: number): void {
        const scope = this.scopes.get(start);
        if (scope === undefined) return;
        for (const step of [...this.undo, ...this.redo]) {
            if ('positions' in step && step.scope === scope) {
                step.before.detached = undefined;
                step.after.detached = undefined;
            }
        }
    }

    prepare(change: LayerChange, edits: TextEdit[]): () => void {
        const document = this.document;
        const text = document.getText();
        const original = digest(text);
        const lineEnding = document.eol === vscode.EndOfLine.CRLF ? '\r\n' : '\n';
        const changes = edits.map(({ range, newText }) => {
            const checked = exactRange(document, range);
            const start = document.offsetAt(checked.start);
            return { rangeOffset: start, rangeLength: document.offsetAt(checked.end) - start,
                text: newText.replace(/\r\n|\r|\n/g, lineEnding) };
        }).sort(byOffset);
        const expectedSource = digest(applyChanges(text, changes));
        const before = change.block ? {
            start: document.offsetAt(asPosition(change.block.start)),
            end: document.offsetAt(asPosition(change.block.end)),
        } : { start: documentScope, end: documentScope };
        const pending = { scope: this.scopes.get(before.start) ?? Symbol(),
            version: document.version, text, change, expectedSource,
            before: { ...before, stamp: change.beforeStamp, source: original },
            signature: original === expectedSource ? signature(changes) : undefined };
        this.source = original;
        this.pending = pending;
        return () => { if (this.pending === pending) this.pending = undefined; };
    }

    sourceChanged(event: vscode.TextDocumentChangeEvent): LayerTransfer | undefined {
        if (!this.pending && !this.undo.length && !this.redo.length) return;
        const previous = this.source;
        this.source = digest(event.document.getText());
        const pending = this.pending;
        this.pending = undefined;
        const { contentChanges: changes, reason } = event;
        this.rebaseScopes(changes);
        if (reason === undefined) {
            this.redo.length = 0;
            if (!pending || event.document.version !== pending.version + 1
                || this.source !== pending.expectedSource || !matchesSignature(pending.signature, changes)) {
                if (previous) this.record({ beforeSource: previous, afterSource: this.source });
                return;
            }
            const after = pending.before.start === documentScope ? pending.before : {
                start: offsetAfterChanges(pending.before.start, changes),
                end: offsetAfterChanges(pending.before.end, changes),
            };
            const step: Step = {
                scope: pending.scope,
                before: pending.before,
                after: { start: after.start, end: after.end, stamp: pending.change.afterStamp, source: this.source },
                positions: Uint32Array.from(pending.change.positions.flat()),
                block: pending.change.block,
                forwardSignature: pending.signature,
                reverseSignature: pending.signature && signature(inverseChanges(pending.text, changes)),
            };
            this.record(step);
            this.rememberScope(after.start, step.scope);
            return transfer(step, false);
        }
        const undo = reason === vscode.TextDocumentChangeReason.Undo;
        const stack = undo ? this.undo : this.redo;
        const next = undo ? this.redo : this.undo;
        const step = stack.at(-1);
        if (!step) return;
        if (!('positions' in step)) {
            // Ordinary editor typing can undo several events together. Keep it
            // between designer operations even if it recreates the same text.
            moveSourceChanges(stack, next, undo, previous, this.source);
            return;
        }
        const from = undo ? step.after : step.before;
        const to = undo ? step.before : step.after;
        if (from.source !== previous || to.source !== this.source
            || !matchesSignature(undo ? step.reverseSignature : step.forwardSignature, changes)) return;
        stack.pop();
        next.push(step);
        this.rememberScope(to.start, step.scope);
        return transfer(step, undo);
    }

    private rebaseScopes(changes: readonly vscode.TextDocumentContentChangeEvent[]): void {
        if (!this.scopes.size || this.scopes.has(documentScope)) return;
        const next = new Map<number, symbol>();
        for (const [start, scope] of this.scopes) {
            if (changes.some(change => change.rangeOffset <= start
                && change.rangeOffset + change.rangeLength > start)) continue;
            next.set(offsetAfterChanges(start, changes), scope);
        }
        this.scopes = next;
    }

    private rememberScope(start: number, scope: symbol): void {
        this.scopes.delete(start);
        this.scopes.set(start, scope);
        if (this.scopes.size > historyLimit) {
            const oldest = this.scopes.keys().next().value;
            if (oldest !== undefined) this.scopes.delete(oldest);
        }
    }

    private record(entry: Entry): void {
        this.undo.push(entry);
        if (this.undo.length > historyLimit) this.undo.shift();
    }
}

function moveSourceChanges(stack: Entry[], next: Entry[], reverse: boolean, from: string | undefined, to: string): void {
    let source = from;
    for (let index = stack.length - 1; index >= 0; index--) {
        const entry = stack[index];
        if ('positions' in entry || source !== (reverse ? entry.afterSource : entry.beforeSource)) return;
        source = reverse ? entry.beforeSource : entry.afterSource;
        if (source === to) {
            next.push(...stack.splice(index).reverse());
            return;
        }
    }
}

function transfer(step: Step, reverse: boolean): LayerTransfer {
    const positions = new Map<number, number>();
    for (let index = 0; index < step.positions.length; index += 2) {
        const old = step.positions[index];
        const next = step.positions[index + 1];
        positions.set(reverse ? next : old, reverse ? old : next);
    }
    return { from: reverse ? step.after : step.before, to: reverse ? step.before : step.after,
        positions, block: step.block };
}

function applyChanges(source: string, changes: readonly Change[]): string {
    let cursor = 0;
    let edited = '';
    for (const change of changes) {
        edited += source.slice(cursor, change.rangeOffset) + change.text;
        cursor = change.rangeOffset + change.rangeLength;
    }
    return edited + source.slice(cursor);
}

function inverseChanges(source: string, changes: readonly Change[]): Change[] {
    let delta = 0;
    return [...changes].sort(byOffset).map(({ rangeOffset, rangeLength, text }) => {
        const inverse = { rangeOffset: rangeOffset + delta, rangeLength: text.length,
            text: source.slice(rangeOffset, rangeOffset + rangeLength) };
        delta += text.length - rangeLength;
        return inverse;
    });
}

function byOffset(a: Change, b: Change): number { return a.rangeOffset - b.rangeOffset; }
function digest(source: string): string { return createHash('sha256').update(source).digest('hex'); }
function signature(changes: readonly Change[]): string {
    return digest(JSON.stringify([...changes].sort(byOffset).map(({ rangeOffset, rangeLength, text }) =>
        [rangeOffset, rangeLength, text])));
}
function matchesSignature(expected: string | undefined, changes: readonly Change[]): boolean {
    return expected === undefined || signature(changes) === expected;
}
