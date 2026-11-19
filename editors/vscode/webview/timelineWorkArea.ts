import type { TimelinePreviewRange } from '../src/timelineProtocol';
import { validPreviewRange } from '../src/timelinePreviewRange';
import { anchorPopover, element } from './dom';
import { clamp, timelinePercent, type TimelineViewport } from './timelineViewport';

export class TimelineWorkArea {
    private readonly button = element<HTMLButtonElement>('timeline-range-options');
    private readonly popup = element('timeline-preview-range');
    private readonly enable = element<HTMLInputElement>('timeline-range-enabled');
    private readonly fields = element<HTMLFieldSetElement>('timeline-range-fields');
    private readonly start = element<HTMLInputElement>('timeline-range-in');
    private readonly end = element<HTMLInputElement>('timeline-range-out');
    private readonly repeat = element<HTMLInputElement>('timeline-range-loop');
    private readonly band = element('timeline-range-band');
    private readonly status = element('timeline-range-feedback');
    private settings: TimelinePreviewRange = { start: 0, end: 0, loop: true };
    private playingRange?: TimelinePreviewRange;
    private duration = 0;

    constructor(grid: HTMLElement, private readonly viewport: () => TimelineViewport, private readonly time: () => number,
        private readonly changed: () => void) {
        anchorPopover(this.button, this.popup);
        this.popup.addEventListener('toggle', event => {
            this.button.setAttribute('aria-expanded', String(event.newState === 'open'));
        });
        this.enable.onchange = this.repeat.onchange = () => this.apply();
        this.start.onchange = this.end.onchange = () => this.apply();
        grid.addEventListener('keydown', event => this.setBoundary(event));
    }

    get range(): TimelinePreviewRange | undefined {
        if (!this.button.hidden && this.enable.checked) return this.settings;
    }

    follow(duration: number, reset: boolean): void {
        this.button.hidden = duration <= 0;
        if (this.button.hidden) { this.popup.hidePopover(); return; }
        if (reset) {
            this.enable.checked = false;
            this.settings = { start: 0, end: duration, loop: true };
        } else if (duration !== this.duration) {
            const settings = { ...this.settings, end: this.settings.end === this.duration ? duration : Math.min(duration, this.settings.end) };
            this.settings = validPreviewRange(settings, duration) ? settings : { start: 0, end: duration, loop: this.settings.loop };
        } else return;
        this.duration = duration;
        this.start.max = this.end.max = String(duration);
        this.fill();
        this.report();
    }

    show(disabled: boolean, playingRange?: TimelinePreviewRange): void {
        this.enable.disabled = this.fields.disabled = disabled;
        this.fields.hidden = !this.enable.checked;
        this.playingRange = playingRange;
        this.layout();
    }

    layout(): void {
        const range = this.playingRange ?? this.range;
        const axis = this.viewport();
        this.button.toggleAttribute('data-limited', Boolean(range));
        if (!range || !axis.span) { this.band.hidden = true; return; }
        const start = clamp(timelinePercent(axis, range.start), 0, 100);
        const end = clamp(timelinePercent(axis, range.end), 0, 100);
        this.band.hidden = end <= start;
        this.band.style.insetInlineStart = `${start}%`;
        this.band.style.inlineSize = `${end - start}%`;
    }

    private setBoundary(event: KeyboardEvent): void {
        if (this.button.hidden || this.enable.disabled || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
        const key = event.key.toLowerCase();
        if (key !== 'i' && key !== 'o') return;
        event.preventDefault();
        const time = this.time();
        const { start, end, loop } = this.settings;
        const candidate = key === 'i'
            ? { start: time, end: time < end ? end : this.duration, loop }
            : { start: start < time ? start : 0, end: time, loop };
        if (!validPreviewRange(candidate, this.duration)) return;
        this.enable.checked = true;
        this.settings = candidate;
        this.fill();
        this.report();
        this.changed();
    }

    private fill(): void {
        this.start.value = String(this.settings.start);
        this.end.value = String(this.settings.end);
        this.repeat.checked = this.settings.loop;
    }

    private apply(): void {
        const range = { start: this.start.valueAsNumber, end: this.end.valueAsNumber, loop: this.repeat.checked };
        if (validPreviewRange(range, this.duration)) {
            this.settings = range;
            this.report();
        } else {
            this.fill();
            this.report('Choose an In and Out time within the animation, with In before Out.');
        }
        this.changed();
    }

    private report(message?: string): void {
        this.status.textContent = message ?? '';
        this.status.hidden = !message;
    }
}
