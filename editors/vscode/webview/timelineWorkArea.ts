import type { TimelinePreviewRange } from '../src/timelineProtocol';
import { validPreviewRange } from '../src/timelinePreviewRange';
import { anchorPopover, element } from './dom';
import { clamp, timelinePercent, type TimelineViewport } from './timelineViewport';

export class TimelineWorkArea {
    private readonly button: HTMLButtonElement;
    private readonly popup: HTMLElement;
    private readonly enable: HTMLInputElement;
    private readonly fields: HTMLFieldSetElement;
    private readonly start: HTMLInputElement;
    private readonly end: HTMLInputElement;
    private readonly repeat: HTMLInputElement;
    private readonly band: HTMLElement;
    private readonly status: HTMLElement;
    private settings: TimelinePreviewRange = { start: 0, end: 0, loop: true };
    private playingRange?: TimelinePreviewRange;
    private accepted?: TimelinePreviewRange;
    private duration = 0;

    constructor(prefix: string, grid: HTMLElement, private readonly viewport: () => TimelineViewport, private readonly time: () => number,
        private readonly changed: () => void) {
        this.button = element(`${prefix}-range-options`);
        this.popup = element(`${prefix}-preview-range`);
        this.enable = element(`${prefix}-range-enabled`);
        this.fields = element(`${prefix}-range-fields`);
        this.start = element(`${prefix}-range-in`);
        this.end = element(`${prefix}-range-out`);
        this.repeat = element(`${prefix}-range-loop`);
        this.band = element(`${prefix}-range-band`);
        this.status = element(`${prefix}-range-feedback`);
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

    follow(duration: number, reset: boolean, visibleEnd = duration): void {
        this.button.hidden = duration <= 0;
        if (this.button.hidden) { this.popup.hidePopover(); return; }
        const fitted = Math.min(duration, visibleEnd);
        if (reset || this.duration === 0) {
            this.enable.checked = false;
            this.accepted = undefined;
            this.settings = { start: 0, end: fitted, loop: true };
            this.popup.hidePopover();
        } else if (duration !== this.duration) {
            const end = this.settings.end === this.duration ? fitted : Math.min(duration, this.settings.end);
            const settings = { ...this.settings, end };
            if (validPreviewRange(settings, duration)) this.settings = settings;
            else {
                this.enable.checked = false;
                this.settings = { start: 0, end: fitted, loop: this.settings.loop };
            }
        } else return;
        this.duration = duration;
        this.start.max = this.end.max = Number.isFinite(duration) ? String(duration) : '';
        this.fill();
        this.report();
    }

    adopt(range: TimelinePreviewRange | undefined, playing: boolean): void {
        const previous = this.accepted;
        this.accepted = range;
        if (!validPreviewRange(range, this.duration)
            || (previous?.start === range.start && previous.end === range.end && previous.loop === range.loop
                && (!playing || this.enable.checked))) return;
        this.enable.checked = true;
        this.settings = range;
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
            ? { start: time, end: time < end ? end : this.viewport().duration, loop }
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
