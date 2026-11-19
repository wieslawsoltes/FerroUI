import { snapEnabled, type CanvasGuidesState, type Guide, type GuideAxis, type GuideEdit } from '../src/canvasGuidesProtocol';
import type { FromDesigner, ToDesigner } from '../src/protocol';
import { element } from './dom';
import { sameProjection, type GuideProjection } from './canvasRulers';
import { GuideEditor } from './guideEditor';
import { guideAt, GuideOverlay, rulerAt, rulerSize } from './guideOverlay';
import type { CanvasViewport } from './canvasViewport';
import type { Artboard } from './artboard';
import type { SnapCapture } from './snapFeedback';
import { SnappingEditor } from './snappingEditor';

interface Gesture {
    pointer: number; axis: GuideAxis; guide?: Guide; duplicate: boolean;
    x: number; y: number; view: GuideProjection; state: CanvasGuidesState;
    point?: number;
}

/** Keeps pointer capture and viewport projection local. Only the completed gesture reaches the host. */
export class CanvasGuides {
    private readonly editor = new GuideEditor((state, edit) => this.send(state, edit));
    private readonly snapping = new SnappingEditor((state, edit) => this.send(state, edit));
    private readonly overlay = new GuideOverlay();
    private state?: CanvasGuidesState;
    private enabled = false;
    private selecting = false;
    private gesture?: Gesture;
    private repaint?: number;
    private sequence = 0;
    private focus?: number;

    constructor(private readonly viewport: CanvasViewport, private readonly artboard: Artboard, private readonly panning: () => boolean,
        private readonly post: (message: FromDesigner) => void, private readonly report: (message: string) => void) {
        const canvas = viewport.element;
        canvas.addEventListener('pointerdown', event => this.start(event), { capture: true });
        canvas.addEventListener('pointermove', event => this.move(event));
        canvas.addEventListener('pointerup', event => this.finish(event));
        canvas.addEventListener('pointercancel', () => this.cancel());
        canvas.addEventListener('lostpointercapture', event => { if (this.gesture?.pointer === event.pointerId) this.cancel(); });
        canvas.addEventListener('pointerleave', () => { delete canvas.dataset.guideCursor; });
        canvas.addEventListener('scroll', () => this.schedule(), { passive: true });
        element('preview').addEventListener('keydown', event => this.key(event), { capture: true });
        window.addEventListener('blur', () => this.cancel());
        window.addEventListener('resize', () => this.schedule());
        const resize = new ResizeObserver(() => this.schedule());
        resize.observe(canvas);
        resize.observe(this.artboard.element);
        new MutationObserver(() => this.schedule()).observe(document.body, { attributes: true, attributeFilter: ['style', 'class'] });
    }

    present(state: CanvasGuidesState | undefined): void {
        if (this.state?.frame.token !== state?.frame.token || this.state?.frame.session !== state?.frame.session
            || this.state?.epoch !== state?.epoch) this.cancel();
        this.state = state;
        this.editor.update(this.enabled ? state : undefined);
        this.snapping.update(state, this.enabled);
        this.schedule();
    }

    setAvailable(enabled: boolean, selecting: boolean): void {
        if (enabled === this.enabled && selecting === this.selecting) return;
        this.enabled = enabled;
        this.selecting = selecting;
        if (!this.canDrag()) this.cancel();
        this.editor.update(enabled ? this.state : undefined);
        this.snapping.update(this.state, enabled);
        this.schedule();
    }

    result(message: Extract<ToDesigner, { type: 'guideResult' }>): void {
        if (this.editor.result(message.request, message.error) || this.snapping.result(message.request, message.error)
            || message.request !== this.sequence) return;
        if (message.frame.token !== this.state?.frame.token || message.frame.session !== this.state.frame.session) return;
        if (message.error) this.report(message.error);
        else if (message.selected != null && this.canDrag()) { this.focus = message.selected; this.schedule(); }
    }

    get snapCapture(): SnapCapture | undefined {
        const state = this.state;
        if (this.enabled && state?.snapping) return { frame: state.frame, epoch: state.epoch,
            zoom: Math.max(.01, this.viewport.state.scale), active: snapEnabled(state.settings) };
    }

    schedule(): void {
        if (this.repaint !== undefined) return;
        this.repaint = requestAnimationFrame(() => {
            this.repaint = undefined;
            const view = this.projection();
            if (this.gesture && (!view || !sameProjection(view, this.gesture.view))) this.cancel();
            const ghost = this.gesture?.point !== undefined ? { axis: this.gesture.axis, position: this.gesture.point } : undefined;
            this.overlay.draw(this.state, view, this.enabled && this.selecting, ghost);
            const snap = this.state?.snapping && this.state.settings;
            this.artboard.showGrid(view && snap && snap.grid_visible ? snap.grid_spacing * view.scale : undefined);
            if (this.focus !== undefined) { this.overlay.focus(this.focus); this.focus = undefined; }
        });
    }

    private projection(): GuideProjection | undefined {
        const canvas = this.viewport.element;
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        if (this.artboard.element.hidden || !width || !height) return;
        const { left, top } = canvas.getBoundingClientRect();
        const artboard = this.artboard.element.getBoundingClientRect();
        return { left, top, width, height, x: artboard.left - left, y: artboard.top - top, scale: this.viewport.state.scale };
    }

    private canDrag(): boolean { return this.enabled && this.selecting && !this.panning(); }

    private start(event: PointerEvent): void {
        const state = this.state;
        const view = this.projection();
        if (!state || !view || !this.canDrag() || this.gesture || event.button !== 0 || !event.isPrimary || overCanvasControl(event)) return;
        const rail = rulerSize(state, view);
        const guide = guideAt(state, view, rail, event);
        const axis = guide?.axis ?? rulerAt(view, rail, event);
        if (!axis) return;
        event.preventDefault();
        event.stopImmediatePropagation();
        const canvas = this.viewport.element;
        canvas.setPointerCapture(event.pointerId);
        if (!guide || !this.overlay.focus(guide.id)) canvas.focus({ preventScroll: true });
        this.gesture = { pointer: event.pointerId, axis, guide, duplicate: Boolean(guide && event.altKey),
            x: event.clientX, y: event.clientY, view, state };
    }

    private move(event: PointerEvent): void {
        const view = this.projection();
        const state = this.state;
        if (!view || !state || !this.canDrag()) { this.cancel(); return; }
        const gesture = this.gesture;
        if (gesture?.pointer === event.pointerId) {
            if (!sameProjection(view, gesture.view)) { this.cancel(); return; }
            gesture.point = moved(event, gesture) ? position(event, gesture) : undefined;
            this.schedule();
        } else if (!gesture) {
            const rail = rulerSize(state, view);
            const axis = overCanvasControl(event) ? undefined : guideAt(state, view, rail, event)?.axis ?? rulerAt(view, rail, event);
            if (axis) this.viewport.element.dataset.guideCursor = axis;
            else delete this.viewport.element.dataset.guideCursor;
        }
    }

    private finish(event: PointerEvent): void {
        const drag = this.gesture;
        if (!drag || event.pointerId !== drag.pointer) return;
        const view = this.projection();
        this.cancel();
        if (!view || !this.canDrag() || !sameProjection(view, drag.view) || !moved(event, drag)) return;
        const { state, guide, axis, duplicate } = drag;
        const rail = rulerSize(state, view);
        const x = event.clientX - view.left;
        const y = event.clientY - view.top;
        if (x < 0 || y < 0 || x >= view.width || y >= view.height) return;
        if (x < rail || y < rail) {
            if (guide && !duplicate && rulerAt(view, rail, event) === axis) this.send(state, { action: 'remove', id: guide.id });
            return;
        }
        const point = position(event, drag);
        if (guide) this.send(state, { action: duplicate ? 'duplicate' : 'move', id: guide.id, position: point });
        else if (state.settings.guides_visible) this.send(state, { action: 'add', axis, position: point });
        else this.send(state, { action: 'apply', rulers: true, guides: true, lines: [...state.settings.guides, { axis, position: point }] });
    }

    private key(event: KeyboardEvent): void {
        if (event.key === 'Escape' && this.gesture) { event.preventDefault(); this.cancel(); return; }
        const state = this.state;
        if (!state || !this.canDrag() || event.ctrlKey || event.metaKey || event.altKey) return;
        const key = event.key.toLowerCase();
        if (event.target === this.viewport.element && event.shiftKey && (key === 'r' || key === 'g')) {
            this.send(state, key === 'r' ? { action: 'visibility', rulers: !state.settings.rulers_visible }
                : { action: 'visibility', guides: !state.settings.guides_visible });
        } else {
            const id = Number((event.target as HTMLElement | null)?.dataset.guide);
            const guide = state.settings.guides.find(line => line.id === id);
            if (!guide) return;
            if (key === 'delete' || key === 'backspace') {
                this.send(state, { action: 'remove', id });
                this.viewport.element.focus({ preventScroll: true });
            } else {
                const [before, after] = guide.axis === 'Horizontal' ? ['arrowup', 'arrowdown'] : ['arrowleft', 'arrowright'];
                if (key !== before && key !== after) return;
                const step = event.shiftKey ? state.settings.large_nudge : state.settings.small_nudge;
                this.send(state, { action: 'move', id, position: guide.position + (key === before ? -step : step) });
            }
        }
        event.preventDefault();
        event.stopPropagation();
    }

    private cancel(): void {
        const gesture = this.gesture;
        this.gesture = undefined;
        const canvas = this.viewport.element;
        delete canvas.dataset.guideCursor;
        if (!gesture) return;
        if (canvas.hasPointerCapture(gesture.pointer)) canvas.releasePointerCapture(gesture.pointer);
        this.schedule();
    }

    private send(state: CanvasGuidesState, edit: GuideEdit): number {
        const request = ++this.sequence;
        this.post({ type: 'guideEdit', frame: state.frame, epoch: state.epoch, edit, request });
        return request;
    }
}

function moved(event: PointerEvent, drag: Gesture): boolean {
    return Math.hypot(event.clientX - drag.x, event.clientY - drag.y) >= 3;
}

function overCanvasControl(event: PointerEvent): boolean {
    return event.target instanceof Element && Boolean(event.target.closest('.resize-handle, .focus-marker'));
}

function position(event: PointerEvent, drag: Gesture): number {
    const { view, guide, x, y } = drag;
    const horizontal = drag.axis === 'Horizontal';
    const pixel = horizontal ? event.clientY : event.clientX;
    const origin = horizontal ? view.top + view.y : view.left + view.x;
    const value = guide ? guide.position + (pixel - (horizontal ? y : x)) / view.scale : (pixel - origin) / view.scale;
    const limit = drag.state.maxPosition;
    return Math.max(-limit, Math.min(limit, Math.round(value * 1000) / 1000));
}
