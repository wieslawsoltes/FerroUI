import type { DragGesture, DropTarget, FrameTarget, FromDesigner, PlacementResult } from '../src/protocol';
import { Artboard } from './artboard';
import { element } from './dom';
import { DragSource, finishDrop, matchDrag } from './drag';

export class CanvasDrag {
    private readonly label = element('drop-hint');
    private frame?: FrameTarget;
    private gesture?: DragGesture;
    private readonly source: DragSource;
    private queued?: { id: string; frame: FrameTarget; target: DropTarget; sequence: number };
    private sequence = 0;
    private requested?: string;
    private inFlight?: number;
    private timer?: ReturnType<typeof setTimeout>;

    constructor(private readonly artboard: Artboard, canDrag: () => boolean, private readonly send: (message: FromDesigner) => void) {
        const board = artboard.element;
        this.source = new DragSource(board, event => {
            const grab = pointerPosition(artboard, event);
            const node = artboard.hit(grab);
            if (grab && node?.canMove && this.frame && canDrag()) {
                const nodes = artboard.movableSelection(node.id);
                return { frame: this.frame, origin: nodes.length > 1
                    ? { type: 'nodes', nodes, anchor: node.id, grab } : { type: 'node', node: node.id, grab } };
            }
        }, send, gesture => this.setGesture(gesture));
        board.addEventListener('pointerdown', () => {
            if (this.gesture) {
                send({ type: 'endDrag', id: this.gesture.id });
                this.gesture = undefined;
                this.clearHint();
            }
        }, true);
        artboard.surface.addEventListener('pointermove', event => {
            if (!(event.buttons & 1)) {
                if (this.gesture) send({ type: 'endDrag', id: this.gesture.id });
                this.clearHint();
                return;
            }
            const point = pointerPosition(artboard, event);
            if (!canDrag() || !this.frame || !matchDrag(this.frame, this.gesture)) return;
            if (!point) {
                this.clearHint();
                this.feedback('Move onto the artboard to drop', 'outside');
                return;
            }
            board.classList.add('placing');
            if (this.label.hidden || this.label.dataset.state === 'outside') this.feedback('Finding drop target…', 'pending');
            const key = `${Math.round(point.x)}:${Math.round(point.y)}`;
            if (key !== this.requested) {
                this.requested = key;
                this.queued = { id: this.gesture.id, frame: this.frame, target: { type: 'canvas', ...point }, sequence: ++this.sequence };
                this.timer ??= setTimeout(() => this.flush(), 55);
            }
        });
        artboard.surface.addEventListener('pointerleave', () => this.clearHint());
        artboard.surface.addEventListener('pointerup', event => {
            const point = pointerPosition(artboard, event);
            if (event.button === 0 && canDrag() && point) {
                this.source.startIfMoved(event);
                finishDrop(this.frame, this.gesture, { type: 'canvas', ...point }, send);
            }
            this.clearHint();
        });
    }

    setFrame(frame?: FrameTarget): void {
        this.frame = frame;
        if (!frame) this.source.end();
        this.clearHint();
    }

    setGesture(gesture?: DragGesture): void {
        if (this.gesture?.id !== gesture?.id) this.clearHint();
        this.gesture = gesture;
        if (!gesture) this.source.end(false);
    }

    show(result: PlacementResult): void {
        if (result.id !== this.gesture?.id || result.sequence !== this.inFlight) return;
        this.inFlight = undefined;
        if (this.queued) this.timer ??= setTimeout(() => this.flush(), 55);
        if (result.sequence !== this.sequence) return;
        const state = result.error ? 'error' : 'valid';
        this.feedback(result.error ?? result.hint?.description ?? '', state);
        this.artboard.element.classList.toggle('invalid-placement', state === 'error');
        this.artboard.showDrop(result.hint?.bounds);
    }

    private flush(): void {
        this.timer = undefined;
        if (this.inFlight !== undefined || !this.queued) return;
        this.inFlight = this.queued.sequence;
        this.send({ type: 'previewDrop', ...this.queued });
        this.queued = undefined;
    }

    private clearHint(): void {
        clearTimeout(this.timer);
        this.timer = this.queued = this.requested = this.inFlight = undefined;
        this.sequence++;
        this.artboard.showDrop();
        this.artboard.element.classList.remove('placing', 'invalid-placement');
        this.label.hidden = true;
    }

    private feedback(text: string, state: 'pending' | 'outside' | 'valid' | 'error'): void {
        this.label.dataset.state = state;
        this.label.textContent = text;
        this.label.hidden = !text;
    }
}

function pointerPosition(artboard: Artboard, event: PointerEvent) {
    const point = artboard.point(event);
    return point && { x: Math.round(point.x), y: Math.round(point.y) };
}
