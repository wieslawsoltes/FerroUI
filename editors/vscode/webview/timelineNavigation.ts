import { element } from './dom';
import { clamp, stepTimelineFrame } from './timelineViewport';

const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });

export class TimelineNavigation {
    private readonly clock = element<HTMLFormElement>('timeline-time-editor');
    private readonly input = element<HTMLInputElement>('timeline-time-input');
    private readonly length = element('timeline-duration');
    private readonly previous = element<HTMLButtonElement>('timeline-previous-key');
    private readonly next = element<HTMLButtonElement>('timeline-next-key');
    private time = 0;
    private duration?: number;
    private keys: readonly number[] = [];
    private drafting = false;

    constructor(grid: HTMLElement, private readonly slider: HTMLInputElement, frameRate: () => number,
        private readonly seek: (time: number) => void) {
        this.previous.onclick = () => this.go(this.adjacent(false));
        this.next.onclick = () => this.go(this.adjacent(true));
        this.input.oninput = () => { this.drafting = true; };
        this.input.onblur = () => this.resetInput();
        this.input.onkeydown = event => {
            if (event.key === 'Escape') { event.stopPropagation(); this.resetInput(); }
        };
        this.clock.onsubmit = event => {
            event.preventDefault();
            const time = this.input.valueAsNumber;
            this.resetInput();
            if (Number.isFinite(time)) this.go(time);
        };
        grid.addEventListener('keydown', event => this.key(event, frameRate()));
    }

    setKeys(times: readonly number[], reset: boolean): void {
        this.keys = [...new Set(times)].sort((first, second) => first - second);
        if (reset) this.drafting = false;
    }

    show(time: number, duration: number | undefined, showClock: boolean, elapsedLabel: string): void {
        const enabled = !this.slider.disabled;
        const focused = document.activeElement;
        this.time = time;
        if (this.duration !== duration) {
            this.length.textContent = duration === undefined ? '' : `/ ${number.format(duration)} s`;
        }
        this.duration = duration;
        this.clock.hidden = !showClock;
        this.clock.title = elapsedLabel;
        this.previous.hidden = this.next.hidden = duration === undefined;
        this.input.disabled = !enabled;
        this.previous.disabled = !enabled || (this.keys[0] ?? Infinity) >= time;
        this.next.disabled = !enabled || (this.keys.at(-1) ?? -Infinity) <= time;
        if (!this.drafting || !enabled) this.resetInput();
        if (enabled && ((focused === this.previous && this.previous.disabled) || (focused === this.next && this.next.disabled))) {
            this.slider.focus({ preventScroll: true });
        }
    }

    private key(event: KeyboardEvent, frameRate: number): void {
        if (this.duration === undefined || event.altKey || event.metaKey || event.ctrlKey || this.slider.disabled
            || (event.target instanceof HTMLInputElement && event.target !== this.slider)) return;
        let time: number | undefined;
        switch (event.key) {
            case 'Home': time = 0; break;
            case 'End': time = this.duration; break;
            case '[': time = this.adjacent(false); break;
            case ']': time = this.adjacent(true); break;
            case ',':
            case '.':
            case '<':
            case '>': {
                const frames = (event.key === ',' || event.key === '<' ? -1 : 1) * (event.shiftKey ? 10 : 1);
                time = stepTimelineFrame(this.time, frames, frameRate || 30, this.duration);
                break;
            }
            default: return;
        }
        event.preventDefault();
        event.stopPropagation();
        this.go(time);
    }

    private adjacent(forward: boolean): number | undefined {
        return forward ? this.keys.find(time => time > this.time) : this.keys.findLast(time => time < this.time);
    }

    private go(time: number | undefined): void {
        if (time !== undefined && this.duration !== undefined && !this.slider.disabled) this.seek(clamp(time, 0, this.duration));
    }

    private resetInput(): void {
        this.drafting = false;
        const formatted = String(Number(this.time.toFixed(6)));
        if (this.input.value !== formatted) this.input.value = formatted;
    }
}
