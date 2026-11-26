import type { TextDocument, TextDocumentContentChangeEvent } from 'vscode';
import type { RustTemplateNavigation, Snapshot } from './protocol';
import { asPosition, asRange, offsetAfterChanges } from './textDocument';

interface Scope {
    previous: number;
    consumer?: string;
    body: number;
    owner?: number;
    property: RustTemplateNavigation['property'];
}

/** Return links follow original RSX token anchors through edits to the buffer. */
export class RustTemplateScopes {
    private readonly stack: Scope[] = [];

    clear(): void { this.stack.length = 0; }

    enter(document: TextDocument, snapshot: Snapshot, template: RustTemplateNavigation, owner?: number, consumer?: string): number {
        const range = asRange(template.range);
        const active = snapshot.activeBlock && asRange(snapshot.activeBlock);
        if (!snapshot.image || !active || (!consumer && active.isEqual(range))
            || range.isEmpty || !snapshot.blocks.some(block => asRange(block).isEqual(range))) {
            throw new Error('The template body changed. Read the current Rust template slot.');
        }
        const body = document.offsetAt(range.start);
        if (this.stack.some(scope => scope.body === body || scope.previous === body)) {
            throw new Error('The template is already in the current navigation path.');
        }
        const node = snapshot.nodes.find(node => node.id === owner);
        this.stack.push({ previous: document.offsetAt(active.start), consumer, body, property: template.property,
            owner: node && document.offsetAt(asPosition(node.nameRange.start)) });
        return body;
    }

    leave(document: TextDocument, snapshot: Snapshot): Scope {
        if (!this.accept(document, snapshot)) throw new Error('Open a Rust template in the current preview.');
        return this.stack.pop()!;
    }

    sourceChanged(changes: readonly TextDocumentContentChangeEvent[]): void {
        const replaced = (offset: number) => changes.some(change =>
            change.rangeOffset <= offset && offset < change.rangeOffset + change.rangeLength);
        for (const scope of this.stack) {
            if (replaced(scope.previous) || replaced(scope.body)) {
                this.clear();
                return;
            }
            scope.previous = offsetAfterChanges(scope.previous, changes);
            scope.body = offsetAfterChanges(scope.body, changes);
            if (scope.owner !== undefined) {
                scope.owner = replaced(scope.owner) ? undefined : offsetAfterChanges(scope.owner, changes);
            }
        }
    }

    accept(document: TextDocument, snapshot: Snapshot): Scope['property'] | undefined {
        const scope = this.stack.at(-1);
        const active = snapshot.activeBlock?.start;
        if (!scope || !active) return;
        const starts = new Set(snapshot.blocks.map(block => document.offsetAt(asPosition(block.start))));
        if (document.offsetAt(asPosition(active)) !== scope.body
            || this.stack.some(entry => !starts.has(entry.previous) || !starts.has(entry.body))) {
            this.clear();
            return;
        }
        return scope.property;
    }
}
