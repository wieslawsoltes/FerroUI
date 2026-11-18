import type { DragGesture, DropTarget, FrameTarget, FromDesigner, PlacementResult } from '../src/protocol';
import { Artboard } from './artboard';
import { element } from './dom';
import { allowDrop, finishDrop, matchDrag, startDrag } from './drag';

export class CanvasDrag {
    private readonly label = element('drop-hint');
    private frame?: FrameTarget;
    private gesture?: DragGesture;
    private owned?: DragGesture;
    private queued?: { id: string; frame: FrameTarget; target: DropTarget; sequence: number };
    private sequence = 0;
    private requested?: string;
    private lastError = false;
    private timer?: ReturnType<typeof setTimeout>;

    constructor(private readonly artboard: Artboard, canDrag: () => boolean, private readonly send: (message: FromDesigner) => void) {
        const image = artboard.frame;
        image.addEventListener('dragstart', event => {
            const grab = artboard.point(event);
            const node = artboard.hit(grab);
            if (!grab || !node?.canMove || !this.frame || !canDrag()) { event.preventDefault(); return; }
            this.owned = startDrag(event, this.frame, { type: 'node', node: node.id, grab }, send, node.name ?? node.label);
            this.gesture = this.owned;
        });
        image.addEventListener('dragend', () => {
            if (this.owned) send({ type: 'endDrag', id: this.owned.id });
            this.owned = undefined;
            this.setGesture(undefined);
        });
        image.addEventListener('dragover', event => {
            const point = artboard.point(event);
            if (!canDrag() || !this.frame || !point || !matchDrag(event, this.frame, this.gesture)) return;
            allowDrop(event, this.gesture);
            const key = `${Math.round(point.x)}:${Math.round(point.y)}`;
            if (key !== this.requested) {
                this.requested = key;
                this.lastError = false;
                this.queued = { id: this.gesture.id, frame: this.frame, target: { type: 'canvas', ...point }, sequence: ++this.sequence };
                this.timer ??= setTimeout(() => this.flush(), 55);
            }
            if (this.lastError && event.dataTransfer) event.dataTransfer.dropEffect = 'none';
        });
        image.addEventListener('dragleave', () => this.clearHint());
        image.addEventListener('drop', event => {
            const point = artboard.point(event);
            if (canDrag() && point) finishDrop(event, this.frame, this.gesture, { type: 'canvas', ...point }, send);
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
    }

    show(result: PlacementResult): void {
        if (result.id !== this.gesture?.id || result.sequence !== this.sequence) return;
        this.lastError = Boolean(result.error);
        this.label.textContent = result.hint?.description ?? result.error ?? '';
        this.label.classList.toggle('error', this.lastError);
        this.label.hidden = !this.label.textContent;
        this.artboard.showDrop(result.hint?.bounds);
    }

    private flush(): void {
        this.timer = undefined;
        if (this.queued) this.send({ type: 'previewDrop', ...this.queued });
        this.queued = undefined;
    }

    private clearHint(): void {
        clearTimeout(this.timer);
        this.timer = this.queued = this.requested = undefined;
        this.sequence++;
        this.lastError = false;
        this.artboard.showDrop();
        this.label.hidden = true;
    }
}
