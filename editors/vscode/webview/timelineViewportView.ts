import { create, element } from './dom';
import type { RenderedKeyframe } from './timelineKeyDrag';
import { clamp, fitTimeline, minimumTimelineSpan, panTimeline, timelinePercent, visibleTime, zoomTimeline, type TimelineViewport } from './timelineViewport';

export class TimelineViewportView {
    readonly grid = element('timeline-grid');
    private readonly ruler = element('timeline-ruler');
    private readonly controls = element('timeline-zoom');
    private readonly zoomIn = element<HTMLButtonElement>('timeline-zoom-in');
    private readonly zoomOut = element<HTMLButtonElement>('timeline-zoom-out');
    private readonly fit = element<HTMLButtonElement>('timeline-fit');
    private readonly pan = element<HTMLInputElement>('timeline-pan');
    private readonly panRow = element('timeline-pan-row');
    private viewport = fitTimeline(0);

    constructor(private readonly slider: HTMLInputElement, private readonly keys: () => readonly RenderedKeyframe[]) {
        this.zoomIn.onclick = () => this.zoom(1.25);
        this.zoomOut.onclick = () => this.zoom(.8);
        this.fit.onclick = () => this.apply(fitTimeline(this.range.duration));
        this.pan.oninput = () => this.apply(panTimeline(this.range, this.pan.valueAsNumber));
        this.grid.addEventListener('wheel', event => this.wheel(event), { passive: false });
        this.grid.addEventListener('keydown', event => this.key(event));
    }

    get range(): TimelineViewport { return this.viewport; }

    follow(duration: number | undefined, reset: boolean): boolean {
        this.controls.hidden = duration === undefined;
        if (duration === undefined || (!reset && this.range.duration === duration)) return false;
        this.viewport = fitTimeline(duration);
        return true;
    }

    layout(): void {
        const { duration, start, span } = this.range;
        const zoomed = span < duration;
        const focused = document.activeElement;
        this.grid.style.setProperty('--timeline-scale', String(span > 0 ? duration / span : 1));
        this.grid.style.setProperty('--timeline-shift', String(span > 0 ? -start / span : 0));
        this.zoomIn.disabled = span <= Math.min(duration, minimumTimelineSpan);
        this.zoomOut.disabled = this.fit.disabled = !zoomed;
        this.panRow.hidden = !zoomed;
        this.pan.max = String(duration - span);
        this.pan.value = String(start);
        if ((!zoomed && focused === this.pan)
            || (focused instanceof HTMLButtonElement && focused.disabled && this.controls.contains(focused))) {
            this.slider.focus({ preventScroll: true });
        }

        const precision = span > 0 ? clamp(Math.ceil(-Math.log10(span / 4)) + 1, 0, 7) : 0;
        const format = new Intl.NumberFormat(undefined, { maximumFractionDigits: precision });
        this.pan.setAttribute('aria-valuetext', `${format.format(start)} to ${format.format(start + span)} seconds`);
        const marks = span > 0 ? [0, .25, .5, .75, 1] : [0];
        this.ruler.replaceChildren(...marks.map(fraction => {
            const mark = create('span', format.format(start + fraction * span));
            mark.style.left = `${fraction * 100}%`;
            return mark;
        }));
        for (const { button, key } of this.keys()) {
            const visible = visibleTime(this.range, key.time);
            if (!visible && document.activeElement === button) this.slider.focus({ preventScroll: true });
            button.hidden = !visible;
            if (visible) button.style.left = `${clamp(timelinePercent(this.range, key.time), 0, 100)}%`;
        }
        this.position(this.slider.valueAsNumber);
    }

    position(time: number): void {
        this.grid.style.setProperty('--timeline-position', `${clamp(timelinePercent(this.range, time), 0, 100)}%`);
        this.grid.toggleAttribute('data-playhead-outside', !visibleTime(this.range, time));
    }

    private apply(view: TimelineViewport): void {
        if (this.range.start === view.start && this.range.span === view.span) return;
        this.viewport = view;
        this.layout();
    }

    private zoom(factor: number, time = this.slider.valueAsNumber): void {
        const pivot = visibleTime(this.range, time) ? time : this.range.start + this.range.span / 2;
        this.apply(zoomTimeline(this.range, factor, pivot));
    }

    private wheel(event: WheelEvent): void {
        const bounds = this.ruler.getBoundingClientRect();
        if (bounds.width <= 0 || this.range.span <= 0) return;
        if ((event.ctrlKey || event.metaKey) && event.deltaY) {
            const fraction = clamp((event.clientX - bounds.left) / bounds.width, 0, 1);
            this.zoom(event.deltaY < 0 ? 1.25 : .8, this.range.start + fraction * this.range.span);
        } else if (this.range.span < this.range.duration && (event.shiftKey || Math.abs(event.deltaX) > Math.abs(event.deltaY))) {
            const delta = event.shiftKey && !event.deltaX ? event.deltaY : event.deltaX;
            const units = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? bounds.width : 1;
            this.apply(panTimeline(this.range, this.range.start + delta * units * this.range.span / bounds.width));
        } else return;
        event.preventDefault();
    }

    private key(event: KeyboardEvent): void {
        if (event.altKey || event.ctrlKey || event.metaKey || this.range.span <= 0) return;
        const time = this.keys().find(({ button }) => button === event.target)?.key.time;
        if (event.key === '+' || event.key === '=') this.zoom(1.25, time);
        else if (event.key === '-') this.zoom(.8, time);
        else if (event.key === '0') this.apply(fitTimeline(this.range.duration));
        else if (event.key === 'PageUp' || event.key === 'PageDown') {
            const step = this.range.span * (event.key === 'PageUp' ? -.8 : .8);
            this.apply(panTimeline(this.range, this.range.start + step));
        } else return;
        event.preventDefault();
        event.stopPropagation();
    }
}
