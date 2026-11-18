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
    private lastError = false;
    private inFlight?: number;
    private timer?: ReturnType<typeof setTimeout>;

    constructor(private readonly artboard: Artboard, canDrag: () => boolean, private readonly send: (message: FromDesigner) => void) {
        const image = artboard.frame;
        this.source = new DragSource(image, event => {
            const grab = pointerPosition(artboard, event);
            const node = artboard.hit(grab);
            if (grab && node?.canMove && this.frame && canDrag()) {
                return { frame: this.frame, origin: { type: 'node', node: node.id, grab } };
            }
        }, send, gesture => this.setGesture(gesture));
        image.addEventListener('pointerdown', () => {
            if (this.gesture) {
                send({ type: 'endDrag', id: this.gesture.id });
                this.gesture = undefined;
                this.clearHint();
            }
        }, true);
        image.addEventListener('pointermove', event => {
            if (!(event.buttons & 1)) {
                if (this.gesture) send({ type: 'endDrag', id: this.gesture.id });
                this.clearHint();
                return;
            }
            const point = pointerPosition(artboard, event);
            if (!canDrag() || !this.frame || !point || !matchDrag(this.frame, this.gesture)) return;
            image.classList.add('placing');
            const key = `${Math.round(point.x)}:${Math.round(point.y)}`;
            if (key !== this.requested) {
                this.requested = key;
                this.lastError = false;
                image.classList.remove('invalid-placement');
                this.queued = { id: this.gesture.id, frame: this.frame, target: { type: 'canvas', ...point }, sequence: ++this.sequence };
                this.timer ??= setTimeout(() => this.flush(), 55);
            }
        });
        image.addEventListener('pointerleave', () => this.clearHint());
        image.addEventListener('pointerup', event => {
            const point = pointerPosition(artboard, event);
            if (event.button === 0 && canDrag() && point) finishDrop(this.frame, this.gesture, { type: 'canvas', ...point }, send);
            this.clearHint();
        });
    }

    setFrame(frame?: FrameTarget): void {
        this.frame = frame;
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
        this.lastError = Boolean(result.error);
        this.label.textContent = result.hint?.description ?? result.error ?? '';
        this.label.classList.toggle('error', this.lastError);
        this.artboard.frame.classList.toggle('invalid-placement', this.lastError);
        this.label.hidden = !this.label.textContent;
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
        this.lastError = false;
        this.artboard.showDrop();
        this.artboard.frame.classList.remove('placing', 'invalid-placement');
        this.label.hidden = true;
    }
}

function pointerPosition(artboard: Artboard, event: PointerEvent) {
    const point = artboard.point(event);
    return point && { x: Math.round(point.x), y: Math.round(point.y) };
}
