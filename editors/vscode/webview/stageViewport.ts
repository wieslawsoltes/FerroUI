import { fitStageCamera, type StageCamera } from '../src/stagesProtocol';
import { create, icon, iconButton, labelButton } from './dom';

export class StageViewport {
    readonly element = create('div');
    readonly tools = create('div');
    private readonly zoom = create('button');
    private readonly zoomValue = create('span', 'Fit');
    private readonly pan = iconButton('move', 'Pan preview', () => {
        this.setPan(!this.panning);
        if (this.panning) this.element.focus({ preventScroll: true });
    });
    private readonly resize = new ResizeObserver(() => this.place());
    private camera = fitStageCamera;
    private width = 0;
    private height = 0;
    private scroll = { x: 0, y: 0 };
    private drag?: { pointer: number; startX: number; startY: number; left: number; top: number };

    constructor(private readonly image: HTMLImageElement, choose: () => void,
        private readonly changed: (camera: StageCamera) => void, private readonly panChanged: () => void) {
        this.tools.className = 'stage-camera-tools';
        this.tools.hidden = true;
        this.zoom.type = 'button';
        this.zoom.className = 'metric stage-setting';
        this.zoom.append(icon('zoom-in'), this.zoomValue, icon('chevron-down'));
        this.zoom.onclick = choose;
        this.pan.setAttribute('aria-pressed', 'false');
        this.tools.append(this.zoom, this.pan);
        this.element.className = 'stage-viewport';
        this.element.append(image);
        this.element.addEventListener('scroll', () => this.scrolled(), { passive: true });
        this.element.addEventListener('pointerdown', event => this.begin(event), { capture: true });
        this.element.onpointermove = event => this.move(event);
        this.element.onpointerup = this.element.onpointercancel = this.element.onlostpointercapture = event => {
            if (event.pointerId === this.drag?.pointer) this.endDrag();
        };
        this.element.onkeydown = event => {
            if (event.key !== 'Escape' || !this.panning || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
            event.preventDefault();
            event.stopPropagation();
            this.setPan(false);
            this.pan.focus({ preventScroll: true });
        };
        this.resize.observe(this.element);
    }

    get panning(): boolean { return this.element.hasAttribute('data-pan'); }

    update(name: string, camera: StageCamera | undefined): void {
        this.tools.hidden = !camera;
        const selection = camera?.zoom_percent;
        const fixed = typeof selection === 'number';
        const label = fixed ? `${selection}%` : 'Fit';
        if (this.zoomValue.textContent !== label) this.zoomValue.textContent = label;
        labelButton(this.zoom, `Zoom ${name} · ${label}`);
        labelButton(this.pan, `Pan ${name}`);
        this.pan.disabled = !fixed;
        if (!fixed) this.setPan(false);
        this.element.toggleAttribute('data-fixed-zoom', fixed);
        if (fixed) {
            this.element.tabIndex = 0;
            this.element.setAttribute('role', 'region');
            this.element.setAttribute('aria-label', `${name} preview viewport`);
        } else {
            if (document.activeElement === this.element) this.zoom.focus({ preventScroll: true });
            this.element.removeAttribute('tabindex');
            this.element.removeAttribute('role');
            this.element.removeAttribute('aria-label');
        }
        const next = camera ?? fitStageCamera;
        if (this.camera.zoom_percent === next.zoom_percent && this.camera.x === next.x && this.camera.y === next.y) return;
        this.endDrag();
        this.camera = next;
        this.place();
    }

    present(width: number, height: number): void {
        if (this.width === width && this.height === height) return;
        this.width = width;
        this.height = height;
        this.place();
    }

    dispose(): void {
        this.endDrag();
        this.resize.disconnect();
    }

    endDrag(): void {
        const drag = this.drag;
        this.drag = undefined;
        this.element.removeAttribute('data-dragging');
        if (drag && this.element.hasPointerCapture(drag.pointer)) this.element.releasePointerCapture(drag.pointer);
    }

    private place(): void {
        const { zoom_percent, x, y } = this.camera;
        this.image.style.inlineSize = zoom_percent === null ? '' : `${this.width * zoom_percent / 100}px`;
        this.image.style.blockSize = zoom_percent === null ? '' : `${this.height * zoom_percent / 100}px`;
        if (zoom_percent === null || !this.element.clientWidth) return;
        this.element.scrollLeft = Math.min(x, Math.max(0, this.element.scrollWidth - this.element.clientWidth));
        this.element.scrollTop = Math.min(y, Math.max(0, this.element.scrollHeight - this.element.clientHeight));
        this.scroll = { x: this.element.scrollLeft, y: this.element.scrollTop };
    }

    private scrolled(): void {
        if (this.camera.zoom_percent === null || !this.element.clientWidth) return;
        const x = this.element.scrollLeft;
        const y = this.element.scrollTop;
        if (x === this.scroll.x && y === this.scroll.y) return;
        this.scroll = { x, y };
        this.camera = { zoom_percent: this.camera.zoom_percent, x, y };
        this.changed(this.camera);
    }

    private setPan(enabled: boolean): void {
        if (this.panning === enabled) return;
        this.element.toggleAttribute('data-pan', enabled);
        this.pan.setAttribute('aria-pressed', String(enabled));
        if (!enabled) this.endDrag();
        this.panChanged();
    }

    private begin(event: PointerEvent): void {
        if (!this.panning || event.button !== 0 || !event.isPrimary || event.target !== this.image) return;
        event.preventDefault();
        event.stopPropagation();
        this.element.setPointerCapture(event.pointerId);
        this.drag = { pointer: event.pointerId, startX: event.clientX, startY: event.clientY,
            left: this.element.scrollLeft, top: this.element.scrollTop };
        this.element.setAttribute('data-dragging', '');
    }

    private move(event: PointerEvent): void {
        const drag = this.drag;
        if (!drag || event.pointerId !== drag.pointer) return;
        this.element.scrollLeft = drag.left + drag.startX - event.clientX;
        this.element.scrollTop = drag.top + drag.startY - event.clientY;
    }
}
