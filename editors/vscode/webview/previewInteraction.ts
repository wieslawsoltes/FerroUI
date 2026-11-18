import type { CanvasInteraction, CanvasMode } from '../src/protocol';
import type { Artboard } from './artboard';
import { element } from './dom';

export class PreviewInteraction {
    private readonly select = element<HTMLButtonElement>('canvas-select');
    private readonly interact = element<HTMLButtonElement>('canvas-interact');
    private readonly kind = element<HTMLSelectElement>('canvas-input-kind');
    private readonly value = element<HTMLInputElement>('canvas-input-value');
    private readonly submit = element<HTMLButtonElement>('canvas-send-input');
    private readonly form = element<HTMLFormElement>('canvas-preview-input');
    private available = false;
    private confirmedMode: CanvasMode = 'select';

    constructor(artboard: Artboard, panning: () => boolean, send: (action: CanvasInteraction) => void, changed: () => void) {
        this.select.addEventListener('click', () => { if (this.setMode('select')) changed(); });
        this.interact.addEventListener('click', () => { if (this.setMode('interact')) changed(); });
        this.kind.addEventListener('change', () => {
            const key = this.kind.value === 'key';
            this.value.placeholder = key ? 'Enter a key such as Ctrl+A' : 'Click a control, then enter text';
            this.value.maxLength = key ? 64 : 8192;
        });
        this.form.addEventListener('submit', event => {
            event.preventDefault();
            if (!this.available || this.mode !== 'interact' || !this.value.value) return;
            send(this.kind.value === 'key' ? { kind: 'key', key: this.value.value } : { kind: 'text', text: this.value.value });
        });
        artboard.element.addEventListener('click', event => {
            if (!this.available || this.mode !== 'interact' || panning()) return;
            const point = artboard.normalizedPoint(event);
            if (point) send({ kind: 'click', ...point });
        });
        artboard.element.addEventListener('wheel', event => {
            if (this.mode !== 'interact' || panning() || event.ctrlKey || event.metaKey) return;
            event.preventDefault();
            if (!this.available) return;
            const point = artboard.normalizedPoint(event);
            if (!point) return;
            const scale = event.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? 60 : 1;
            const increment = (delta: number): number => Math.max(-64, Math.min(64, -delta / scale));
            send({ kind: 'scroll', ...point, dx: increment(event.deltaX), dy: increment(event.deltaY) });
        }, { passive: false });
    }

    get mode(): CanvasMode { return this.confirmedMode; }

    setMode(mode: CanvasMode): boolean {
        if (mode === 'interact' && !this.available) return false;
        this.confirmedMode = mode;
        this.select.setAttribute('aria-pressed', String(mode === 'select'));
        this.interact.setAttribute('aria-pressed', String(mode === 'interact'));
        this.form.hidden = mode !== 'interact';
        return true;
    }

    setAvailable(available: boolean): void {
        this.available = available;
        this.interact.disabled = !available;
        this.submit.disabled = !available;
    }
}
