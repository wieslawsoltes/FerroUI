import type { DragGesture, DragMessage, DragOrigin, DropTarget, FrameTarget } from '../src/protocol';

type Send = (message: DragMessage) => void;

interface Press {
    id: number;
    x: number;
    y: number;
    frame: FrameTarget;
    origin: DragOrigin;
}

/** Internal pointer gestures continue across the extension's views. VS Code's
 * native workbench drag redirects HTML DragEvents to its editor-drop overlay. */
export class DragSource {
    private press?: Press;
    private active?: DragGesture;
    private suppressClick = false;

    constructor(root: HTMLElement, candidate: (event: PointerEvent) => { frame: FrameTarget; origin: DragOrigin } | undefined,
        private readonly send: Send, private readonly started: (gesture: DragGesture) => void = () => {}) {
        root.addEventListener('pointerdown', event => {
            this.suppressClick = false;
            if (!event.isPrimary || event.button !== 0) return;
            this.end();
            const item = candidate(event);
            if (item) this.press = { ...item, id: event.pointerId, x: event.clientX, y: event.clientY };
        });
        const move = (event: PointerEvent) => {
            const press = this.press;
            if (!press || event.pointerId !== press.id) return;
            if (!(event.buttons & 1)) { if (this.active) this.end(); return; }
            this.startIfMoved(event);
        };
        window.addEventListener('pointermove', move);
        root.addEventListener('pointerleave', move);
        window.addEventListener('pointerup', event => { if (event.isPrimary) this.end(); });
        window.addEventListener('pointercancel', () => this.end());
        window.addEventListener('blur', () => { if (!this.active) this.end(); });
        window.addEventListener('keydown', event => { if (event.key === 'Escape') this.end(); });
        root.addEventListener('click', event => {
            if (this.suppressClick && event.detail) {
                this.suppressClick = false;
                event.preventDefault();
                event.stopImmediatePropagation();
            }
        }, true);
    }

    // An in-view drop can receive a final position before the next move sample.
    startIfMoved(event: PointerEvent): void {
        const press = this.press;
        if (!press || event.pointerId !== press.id || this.active || Math.hypot(event.clientX - press.x, event.clientY - press.y) < 5) return;
        const gesture = { id: crypto.randomUUID(), frame: press.frame, origin: press.origin };
        this.active = gesture;
        this.suppressClick = true;
        document.body.classList.add('drag-source');
        this.send({ type: 'beginDrag', gesture });
        this.started(gesture);
    }

    end(send = true): void {
        if (this.active && send) this.send({ type: 'endDrag', id: this.active.id });
        this.press = this.active = undefined;
        document.body.classList.remove('drag-source');
    }
}

export function matchDrag(frame: FrameTarget | undefined, gesture: DragGesture | undefined): gesture is DragGesture {
    return Boolean(frame && gesture && frame.session === gesture.frame.session && frame.token === gesture.frame.token);
}

export function finishDrop(frame: FrameTarget | undefined, gesture: DragGesture | undefined, target: DropTarget, send: Send): void {
    if (!frame || !matchDrag(frame, gesture)) return;
    send({ type: 'drop', id: gesture.id, frame, target });
}
