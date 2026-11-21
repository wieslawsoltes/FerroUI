import type { TextDocument, TextDocumentContentChangeEvent } from 'vscode';
import type { RustTemplateNavigation, Snapshot } from './protocol';
import { asPosition, asRange, offsetAfterChanges } from './textDocument';

interface Scope {
    consumer: number;
    body: number;
    owner?: number;
    property: RustTemplateNavigation['property'];
}

/** Return links follow original RSX token anchors through edits to the buffer. */
export class RustTemplateScopes {
    private readonly stack: Scope[] = [];

    clear(): void { this.stack.length = 0; }

    enter(document: TextDocument, snapshot: Snapshot, template: RustTemplateNavigation, owner?: number): number {
        const range = asRange(template.range);
        const consumer = snapshot.activeBlock && asRange(snapshot.activeBlock);
        if (!snapshot.image || !consumer || !consumer.contains(range) || !consumer.start.isBefore(range.start)
            || range.isEmpty || !snapshot.blocks.some(block => asRange(block).isEqual(range))) {
            throw new Error('The template body changed. Read the current Rust template slot.');
        }
        const body = document.offsetAt(range.start);
        const node = snapshot.nodes.find(node => node.id === owner);
        this.stack.push({ consumer: document.offsetAt(consumer.start), body, property: template.property,
            owner: node && document.offsetAt(asPosition(node.nameRange.start)) });
        return body;
    }

    leave(document: TextDocument, snapshot: Snapshot): Scope {
        if (!this.accept(document, snapshot)) throw new Error('Open an inline Rust template in the current preview.');
        return this.stack.pop()!;
    }

    sourceChanged(changes: readonly TextDocumentContentChangeEvent[]): void {
        const replaced = (offset: number) => changes.some(change =>
            change.rangeOffset <= offset && offset < change.rangeOffset + change.rangeLength);
        for (const scope of this.stack) {
            if (replaced(scope.consumer) || replaced(scope.body)) {
                this.clear();
                return;
            }
            scope.consumer = offsetAfterChanges(scope.consumer, changes);
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
            || this.stack.some(entry => !starts.has(entry.consumer) || !starts.has(entry.body))) {
            this.clear();
            return;
        }
        return scope.property;
    }
}
