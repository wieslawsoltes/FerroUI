import { element, labelButton } from './dom';
import { cubicEasing, drawEasingCurve, easingPlot, type CubicEasing } from './easingCurve';
import { clamp } from './timelineViewport';

type ControlPoint = 0 | 2;

const coordinate = (value: number) => Number(value.toFixed(3));

export class EasingHandles {
    private readonly canvas = element<HTMLCanvasElement>('keyframe-curve');
    private readonly handles = [
        element<HTMLButtonElement>('keyframe-handle-in'),
        element<HTMLButtonElement>('keyframe-handle-out'),
    ];
    private points?: CubicEasing;
    private samples: readonly number[] = [];
    private easing?: string;
    private cancelDrag?: () => void;

    constructor(private readonly stage: (easing: string) => void) {
        this.bind(this.handles[0], 0);
        this.bind(this.handles[1], 2);
    }

    show(easing: string, samples: readonly number[] = []): void {
        this.samples = samples.filter(Number.isFinite);
        if (easing !== this.easing) {
            this.cancelDrag?.();
            this.points = cubicEasing(easing);
            this.easing = easing;
        }
        if (!this.cancelDrag) this.render();
    }

    cancel(): void {
        this.cancelDrag?.();
    }

    private bind(handle: HTMLButtonElement, point: ControlPoint): void {
        handle.onpointerdown = event => this.drag(handle, point, event);
        handle.onkeydown = event => {
            if (!this.points || this.cancelDrag || event.altKey || event.ctrlKey || event.metaKey) return;
            const position = ['ArrowLeft', 'ArrowRight', 'ArrowDown', 'ArrowUp'].indexOf(event.key);
            if (position < 0) return;
            event.preventDefault();
            event.stopPropagation();
            const axis = position < 2 ? point : point + 1;
            const step = (position % 2 ? 1 : -1) * (event.shiftKey ? .1 : .01);
            const value = this.points[axis] + step;
            const next = coordinate(axis === point ? clamp(value, 0, 1) : value);
            if (next === this.points[axis]) return;
            this.points[axis] = next;
            this.stage(this.points.join(','));
        };
    }

    private drag(handle: HTMLButtonElement, point: ControlPoint, event: PointerEvent): void {
        this.cancelDrag?.();
        const points = this.points;
        const { width, height } = this.canvas.getBoundingClientRect();
        if (event.button !== 0 || !event.isPrimary || !points || !width || !height || handle.matches(':disabled')) return;
        event.preventDefault();
        handle.focus({ preventScroll: true });
        const { pointerId, clientX: startX, clientY: startY } = event;
        const before: CubicEasing = [...points];
        const plot = easingPlot(before, []);
        const listeners = new AbortController();
        const options = { capture: true, signal: listeners.signal };
        const move = (pointer: PointerEvent) => {
            const dx = pointer.clientX - startX;
            const dy = pointer.clientY - startY;
            const next: CubicEasing = [...before];
            if (Math.abs(dx) >= 1) next[point] = coordinate(plot.progress(plot.x(before[point]) + dx / width));
            if (Math.abs(dy) >= 1) next[point + 1] = coordinate(plot.value(plot.y(before[point + 1]) + dy / height));
            this.points = next;
            this.render(plot);
            return next;
        };
        const finish = () => {
            listeners.abort();
            this.cancelDrag = undefined;
            if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
        };
        const cancel = () => {
            finish();
            this.points = before;
            this.render();
        };
        this.cancelDrag = cancel;
        document.addEventListener('pointermove', pointer => { if (pointer.pointerId === pointerId) move(pointer); }, options);
        document.addEventListener('pointerup', pointer => {
            if (pointer.pointerId !== pointerId) return;
            if (handle.matches(':disabled')) { cancel(); return; }
            const next = move(pointer);
            finish();
            if (next.some((value, index) => value !== before[index])) this.stage(next.join(','));
        }, options);
        document.addEventListener('pointercancel', pointer => { if (pointer.pointerId === pointerId) cancel(); }, options);
        document.addEventListener('keydown', key => {
            if (key.key === 'Escape') { key.preventDefault(); key.stopPropagation(); cancel(); }
        }, options);
        handle.addEventListener('lostpointercapture', cancel, options);
        window.addEventListener('blur', cancel, options);
        handle.setPointerCapture(pointerId);
    }

    private render(plot = easingPlot(this.points, this.samples)): void {
        const points = this.points;
        drawEasingCurve(this.canvas, points, this.samples, plot);
        this.handles.forEach((handle, index) => {
            handle.hidden = !points;
            if (!points) return;
            const x = points[index * 2];
            const y = points[index * 2 + 1];
            handle.style.left = `${plot.x(x) * 100}%`;
            handle.style.top = `${plot.y(y) * 100}%`;
            labelButton(handle, `${index === 0 ? 'First' : 'Second'} easing control point, X ${x}, Y ${y}`);
        });
    }
}
