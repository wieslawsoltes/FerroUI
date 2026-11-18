import { CanvasViewport } from './canvasViewport';
import { element } from './dom';

interface Drag { id: number; x: number; y: number; left: number; top: number }

export class CanvasInput {
    private readonly pan = element<HTMLButtonElement>('pan');
    private tool = false;
    private space = false;
    private drag?: Drag;

    constructor(private readonly viewport: CanvasViewport, reveal: () => void) {
        const canvas = viewport.element;
        const updateCursor = () => { canvas.dataset.pan = this.drag ? 'dragging' : this.panning ? 'ready' : ''; };
        const resetSpace = () => { this.space = false; updateCursor(); };
        this.pan.addEventListener('click', () => this.setTool(!this.tool));
        canvas.addEventListener('keydown', event => {
            if (event.ctrlKey || event.metaKey || event.altKey) return;
            switch (event.key) {
                case ' ': this.space = true; updateCursor(); break;
                case '0': viewport.fit(); break;
                case '1': viewport.zoom(1); break;
                case '+': case '=': viewport.step(1); break;
                case '-': viewport.step(-1); break;
                case 'Enter': reveal(); break;
                default: return;
            }
            event.preventDefault();
        });
        canvas.addEventListener('keyup', event => { if (event.key === ' ') resetSpace(); });
        canvas.addEventListener('blur', resetSpace);
        window.addEventListener('blur', resetSpace);
        canvas.addEventListener('wheel', event => {
            if (!event.ctrlKey && !event.metaKey) return;
            event.preventDefault();
            const pixels = event.deltaY * (event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1);
            viewport.zoomBy(Math.exp(-pixels / 320), { x: event.clientX, y: event.clientY });
        }, { passive: false });
        canvas.addEventListener('pointerdown', event => {
            if (this.drag || (event.button !== 1 && !(event.button === 0 && this.panning))) return;
            event.preventDefault();
            canvas.focus({ preventScroll: true });
            this.drag = { id: event.pointerId, x: event.clientX, y: event.clientY, left: canvas.scrollLeft, top: canvas.scrollTop };
            canvas.setPointerCapture(event.pointerId);
            updateCursor();
        });
        canvas.addEventListener('pointermove', event => {
            const drag = this.drag;
            if (drag?.id !== event.pointerId) return;
            canvas.scrollTo(drag.left + drag.x - event.clientX, drag.top + drag.y - event.clientY);
        });
        canvas.addEventListener('lostpointercapture', event => {
            if (this.drag?.id === event.pointerId) {
                this.drag = undefined;
                updateCursor();
            }
        });
    }

    get panning(): boolean { return this.tool || this.space || this.drag !== undefined; }
    get panTool(): boolean { return this.tool; }

    setTool(enabled: boolean): void {
        this.tool = enabled;
        this.pan.setAttribute('aria-pressed', String(enabled));
        this.viewport.element.dataset.pan = this.drag ? 'dragging' : this.panning ? 'ready' : '';
    }
}
