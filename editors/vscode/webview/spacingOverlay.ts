import type { FrameTarget, FromDesigner, Point } from '../src/protocol';
import type { SpacingCatalog, SpacingHandle, SpacingRequest, SpacingResponse } from '../src/spacingProtocol';
import type { Artboard } from './artboard';
import { create, element } from './dom';
import { startSpacingDrag } from './spacingDrag';

interface Context { frame: FrameTarget; selection: number[]; width: number; height: number }
interface Draft {
    handle: SpacingHandle;
    button: HTMLButtonElement;
    cancel: () => void;
    request?: number;
    latest?: Point;
}
const pixels = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
const gripSize = 24;
const gripOffsets = [0, 26, -26, 52, -52, 78, -78];

export class SpacingOverlay {
    readonly toggle = element<HTMLButtonElement>('spacing-button');
    private readonly overlay = element('spacing-overlay');
    private readonly bands = element<HTMLCanvasElement>('spacing-bands');
    private readonly grips = element('spacing-grips');
    private readonly value = element('spacing-value');
    private context?: Context;
    private catalog?: SpacingCatalog;
    private sequence = 0;
    private measuring?: number;
    private editing?: number;
    private drag?: Draft;
    private focus?: string;
    private suppressClick = false;
    enabled = false;

    constructor(private readonly artboard: Artboard, private readonly post: (message: FromDesigner) => void,
        private readonly report: (error: string) => void) {
        new ResizeObserver(() => this.positionHitTargets()).observe(artboard.element);
        artboard.element.addEventListener('pointerdown', () => { this.suppressClick = false; }, true);
        artboard.element.addEventListener('click', event => {
            if (!this.suppressClick || !event.detail) return;
            this.suppressClick = false;
            event.stopImmediatePropagation();
        }, true);
    }

    get hasHandles(): boolean { return Boolean(this.catalog?.handles.length); }

    setEnabled(enabled: boolean): void {
        this.enabled = enabled;
        this.toggle.setAttribute('aria-pressed', String(enabled));
    }

    update(supported: boolean, selecting: boolean, context?: Context): void {
        this.toggle.hidden = !supported;
        this.toggle.disabled = !context || !selecting;
        const next = supported && selecting && this.enabled && context?.selection.length ? context : undefined;
        if (same(next, this.context)) return;
        this.drag?.cancel();
        if (document.activeElement instanceof HTMLButtonElement && this.grips.contains(document.activeElement)) {
            this.focus = document.activeElement.dataset.handle;
        }
        this.overlay.hidden = true;
        this.context = next;
        this.catalog = undefined;
        this.editing = undefined;
        this.measuring = next && this.send({ kind: 'measure' });
    }

    receive(reply: SpacingResponse): void {
        if (reply.request === this.measuring) {
            this.measuring = undefined;
            if (reply.catalog) this.present(reply.catalog);
            else if (reply.error) this.report(reply.error);
            return;
        }
        if (reply.request === this.editing) {
            this.editing = undefined;
            if (reply.error) {
                this.report(reply.error);
                if (this.context) this.measuring = this.send({ kind: 'measure' });
            }
            return;
        }
        const drag = this.drag;
        if (drag?.request !== reply.request) return;
        drag.request = undefined;
        if (reply.error) {
            drag.cancel();
            this.report(reply.error);
            if (this.context) this.measuring = this.send({ kind: 'measure' });
            return;
        }
        if (reply.draft && this.context) {
            place(drag.button, reply.draft.handle.position, this.context);
            place(this.value, reply.draft.handle.position, this.context);
            this.value.textContent = `${pixels.format(reply.draft.handle.value)} px`;
            this.value.hidden = false;
        }
        this.sendDraft();
    }

    private present(catalog: SpacingCatalog): void {
        const context = this.context;
        if (!context) return;
        this.catalog = catalog;
        this.overlay.hidden = !catalog.handles.length;
        drawBands(this.bands, context, catalog);
        const buttons = catalog.handles.map(handle => this.grip(handle, context));
        const restore = document.activeElement === document.body || this.grips.contains(document.activeElement);
        this.grips.replaceChildren(...buttons);
        this.positionHitTargets();
        if (this.focus && restore) buttons.find(button => button.dataset.handle === this.focus)?.focus({ preventScroll: true });
        this.focus = undefined;
    }

    private positionHitTargets(): void {
        const { context, catalog } = this;
        if (!context || !catalog || this.drag) return;
        const { width, height } = this.artboard.element.getBoundingClientRect();
        if (!width || !height) return;
        const occupied: Point[] = [];
        catalog.handles.forEach((handle, index) => {
            const button = this.grips.children[index] as HTMLButtonElement;
            const origin = { x: handle.position.x * width / context.width, y: handle.position.y * height / context.height };
            const length = Math.hypot(handle.axis.x, handle.axis.y) || 1;
            const sideways = { x: -handle.axis.y / length, y: handle.axis.x / length };
            const candidates = gripOffsets.map(offset => ({ x: origin.x + offset * sideways.x, y: origin.y + offset * sideways.y }));
            const position = candidates.find(candidate => occupied.every(point =>
                Math.abs(candidate.x - point.x) >= gripSize || Math.abs(candidate.y - point.y) >= gripSize)) ?? origin;
            occupied.push(position);
            button.style.setProperty('--nudge-x', `${position.x - origin.x}px`);
            button.style.setProperty('--nudge-y', `${position.y - origin.y}px`);
        });
    }

    private grip(handle: SpacingHandle, context: Context): HTMLButtonElement {
        const button = create('button');
        button.type = 'button';
        button.className = 'spacing-grip';
        button.dataset.handle = handle.id;
        button.dataset.property = handle.property;
        button.setAttribute('aria-disabled', String(Boolean(handle.disabledReason)));
        const label = `${handle.label} · ${handle.ownerLabel} · ${pixels.format(handle.value)} px`;
        button.title = handle.disabledReason ? `${label} · ${handle.disabledReason}` : label;
        button.setAttribute('aria-label', button.title);
        const angle = Math.atan2(handle.axis.y, handle.axis.x);
        const quadrant = (Math.round(angle / (Math.PI / 4)) + 8) % 4;
        button.style.setProperty('--axis', `${angle}rad`);
        button.style.setProperty('--grip-cursor', ['ew-resize', 'nwse-resize', 'ns-resize', 'nesw-resize'][quadrant]);
        place(button, handle.position, context);
        button.onclick = event => {
            event.stopPropagation();
            if (!event.detail) this.apply(handle, { value: handle.value }, 'exact');
        };
        button.ondblclick = event => {
            event.stopPropagation();
            this.apply(handle, { value: handle.value }, 'exact');
        };
        button.onkeydown = event => {
            const sign = ['ArrowUp', 'ArrowLeft'].includes(event.key) ? -1
                : ['ArrowDown', 'ArrowRight'].includes(event.key) ? 1 : 0;
            if (!sign || this.drag) return;
            event.preventDefault();
            event.stopPropagation();
            const step = event.shiftKey ? 10 : event.ctrlKey || event.metaKey ? 0.1 : 1;
            const value = handle.value + sign * step;
            this.apply(handle, { value: Math.max(handle.minimum, Math.min(handle.maximum ?? Infinity, value)) }, 'commit');
        };
        button.onpointerdown = event => {
            if (handle.disabledReason || this.editing) {
                event.stopPropagation();
                return;
            }
            this.drag?.cancel();
            const cancel = startSpacingDrag(this.artboard, button, event, delta => {
                if (this.drag) this.drag.latest = delta;
                this.sendDraft();
            }, delta => {
                this.suppressClick = true;
                this.apply(handle, { delta }, 'commit');
            }, () => {
                this.drag = undefined;
                this.value.hidden = true;
                place(button, handle.position, context);
            });
            if (cancel) this.drag = { handle, button, cancel };
        };
        return button;
    }

    private apply(handle: SpacingHandle, adjustment: { value: number } | { delta: Point }, kind: 'commit' | 'exact'): void {
        if (this.context && this.catalog && !this.measuring && !handle.disabledReason && !this.editing) {
            this.editing = this.send({ kind, model: this.catalog.model, handle: handle.id, adjustment });
        }
    }

    private sendDraft(): void {
        const drag = this.drag;
        if (!drag?.latest || drag.request !== undefined || !this.catalog) return;
        drag.request = this.send({ kind: 'draft', model: this.catalog.model, handle: drag.handle.id, adjustment: { delta: drag.latest } });
        drag.latest = undefined;
    }

    private send(operation: SpacingRequest['operation']): number {
        const { frame, selection } = this.context!;
        const request = ++this.sequence;
        this.post({ type: 'spacing', frame, selection, operation, request });
        return request;
    }
}

function same(a?: Context, b?: Context): boolean {
    if (!a || !b) return a === b;
    return a.frame.session === b.frame.session && a.frame.token === b.frame.token &&
        a.selection.length === b.selection.length && a.selection.every((id, index) => id === b.selection[index]);
}

function place(element: HTMLElement, point: Point, context: Context): void {
    element.style.left = `${point.x / context.width * 100}%`;
    element.style.top = `${point.y / context.height * 100}%`;
}

function drawBands(canvas: HTMLCanvasElement, context: Context, catalog: SpacingCatalog): void {
    canvas.width = context.width;
    canvas.height = context.height;
    const draw = canvas.getContext('2d')!;
    const style = getComputedStyle(canvas);
    const gap = style.getPropertyValue('--spacing-gap');
    const inset = style.getPropertyValue('--spacing-inset');
    for (const { points, filled, padding } of catalog.bands) {
        draw.beginPath();
        draw.moveTo(points[0].x, points[0].y);
        for (const { x, y } of points.slice(1)) draw.lineTo(x, y);
        if (points.length > 2) draw.closePath();
        draw.fillStyle = draw.strokeStyle = padding ? inset : gap;
        draw.globalAlpha = filled ? 0.18 : 0.55;
        if (filled) draw.fill();
        else draw.stroke();
    }
}
