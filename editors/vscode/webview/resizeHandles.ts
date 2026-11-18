import type { Bounds, CanvasNode, ResizeGesture } from '../src/protocol';
import type { Artboard } from './artboard';
import { create, element } from './dom';

const directions = ['top left', 'top', 'top right', 'right', 'bottom right', 'bottom', 'bottom left', 'left'];

export class ResizeHandles {
    private readonly container = element('resize-handles');
    private node?: CanvasNode;
    private cancel?: () => void;
    private suppressClick = false;

    constructor(private readonly artboard: Artboard, private readonly commit: (gesture: ResizeGesture) => void) {
        directions.forEach((direction, handle) => {
            const button = create('button');
            button.className = `resize-handle resize-${handle}`;
            button.title = `Resize ${direction}`;
            button.setAttribute('aria-label', `Resize ${direction}`);
            button.onpointerdown = event => this.pointer(button, handle, event);
            button.onclick = event => event.stopPropagation();
            button.onkeydown = event => {
                const steps: Record<string, readonly [number, number]> = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] };
                const step = steps[event.key];
                if (!step || !this.node) return;
                event.preventDefault();
                event.stopPropagation();
                const scale = event.shiftKey ? 10 : 1;
                this.commit({ node: this.node.id, handle, dx: step[0] * scale, dy: step[1] * scale });
            };
            this.container.append(button);
        });
        artboard.element.addEventListener('pointerdown', () => { this.suppressClick = false; }, true);
        artboard.element.addEventListener('click', event => {
            if (!this.suppressClick || !event.detail) return;
            this.suppressClick = false;
            event.stopImmediatePropagation();
        }, true);
    }

    update(node?: CanvasNode): void {
        const next = node?.canMove && node.bounds ? node : undefined;
        if (next?.id !== this.node?.id) this.cancel?.();
        this.node = next;
        this.container.hidden = !next;
    }

    private pointer(button: HTMLButtonElement, handle: number, event: PointerEvent): void {
        const node = this.node;
        const before = node?.bounds;
        if (!node || !before || event.button !== 0 || !event.isPrimary) return;
        event.preventDefault();
        event.stopPropagation();
        this.cancel?.();
        const origin = this.artboard.point(event, true);
        if (!origin) return;
        const { pointerId } = event;
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        button.focus({ preventScroll: true });
        const movement = (pointer: PointerEvent) => {
            const position = this.artboard.point(pointer, true);
            return position && { dx: Math.round(position.x - origin.x), dy: Math.round(position.y - origin.y) };
        };
        const finish = () => {
            this.cancel = undefined;
            listeners.abort();
            if (button.hasPointerCapture(pointerId)) button.releasePointerCapture(pointerId);
            this.artboard.showDrop();
        };
        this.cancel = finish;
        document.addEventListener('pointermove', move => {
            if (move.pointerId !== pointerId || !(move.buttons & 1)) return;
            const delta = movement(move);
            if (delta) this.artboard.showDrop(rectangle(before, handle, delta.dx, delta.dy));
        }, options);
        document.addEventListener('pointerup', up => {
            if (up.pointerId !== pointerId) return;
            const delta = movement(up);
            finish();
            if (delta && (delta.dx || delta.dy)) {
                this.suppressClick = true;
                this.commit({ node: node.id, handle, ...delta });
            }
        }, options);
        document.addEventListener('pointercancel', cancel => { if (cancel.pointerId === pointerId) finish(); }, options);
        button.addEventListener('lostpointercapture', finish, options);
        window.addEventListener('blur', finish, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') finish();
        }, options);
        button.setPointerCapture(pointerId);
    }
}

function rectangle(bounds: Bounds, handle: number, dx: number, dy: number): Bounds {
    const right = bounds.x + bounds.width;
    const bottom = bounds.y + bounds.height;
    const x = [0, 6, 7].includes(handle) ? Math.min(bounds.x + dx, right - 1) : bounds.x;
    const y = handle <= 2 ? Math.min(bounds.y + dy, bottom - 1) : bounds.y;
    return { x, y, width: handle >= 2 && handle <= 4 ? Math.max(1, right + dx - x) : right - x,
        height: handle >= 4 && handle <= 6 ? Math.max(1, bottom + dy - y) : bottom - y };
}
