import type { FrameTarget } from '../src/protocol';
import type { StateTransitionControl, StateTransitionPosition } from '../src/stateTransitionProtocol';
import { element, labelButton } from './dom';

export class TimelineStateTransition {
    private readonly section = element('timeline-transition');
    private readonly route = element('transition-route');
    private readonly close = element<HTMLButtonElement>('transition-close');
    private readonly play = element<HTMLButtonElement>('transition-play');
    private readonly source = element<HTMLButtonElement>('transition-source');
    private readonly slider = element<HTMLInputElement>('transition-seek');
    private readonly time = element<HTMLOutputElement>('transition-time');
    private readonly seconds = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
    private frame?: FrameTarget;
    private position?: StateTransitionPosition;
    private queued?: number;
    private submitted?: number;
    private pending = false;

    constructor(private readonly send: (frame: FrameTarget, control: StateTransitionControl) => void) {
        this.play.onclick = () => this.request({ action: this.position?.playing ? 'pause' : 'play' });
        this.close.onclick = () => this.request({ action: 'stop' });
        this.source.onclick = () => { if (this.frame && this.position?.sourceRange) this.send(this.frame, { action: 'reveal' }); };
        this.slider.oninput = () => {
            this.queued = this.slider.valueAsNumber;
            this.flush();
            this.paint();
        };
        this.section.addEventListener('keydown', event => {
            if (event.key === 'Escape' && !this.close.disabled) {
                event.preventDefault();
                event.stopPropagation();
                this.request({ action: 'stop' });
            }
        });
    }

    show(position?: StateTransitionPosition, frame?: FrameTarget): void {
        this.frame = frame;
        this.position = position;
        this.pending = Boolean(position?.busy);
        if (!this.pending) this.submitted = undefined;
        this.section.hidden = !position;
        if (!position) { this.queued = undefined; return; }
        const { route } = position;
        const title = `${route.from ?? 'Base values'} → ${route.to}`;
        this.route.textContent = title;
        this.route.title = `${route.group} · ${title}`;
        this.flush();
        this.paint();
    }

    resetRequest(): void {
        this.queued = this.submitted = undefined;
        this.pending = false;
        this.paint();
    }

    private flush(): void {
        if (!this.frame || this.queued === undefined || this.pending || this.position?.playing) return;
        const time = this.submitted = this.queued;
        this.queued = undefined;
        this.request({ action: 'seek', time });
    }

    private request(control: StateTransitionControl): void {
        if (!this.frame || !this.position) return;
        this.pending = true;
        this.send(this.frame, control);
        this.paint();
    }

    private paint(): void {
        const position = this.position;
        if (!position) return;
        const time = this.queued ?? this.submitted ?? position.time;
        this.slider.max = String(position.duration);
        this.slider.value = String(time);
        this.slider.setAttribute('aria-valuetext', `${this.seconds.format(time)} seconds into the transition`);
        this.time.value = `${this.seconds.format(time)} / ${this.seconds.format(position.duration)} s`;
        this.slider.disabled = !this.frame || position.playing || position.duration <= 0;
        this.play.disabled = !this.frame || position.duration <= 0 || (this.pending && !position.playing);
        this.close.disabled = !this.frame || (this.pending && !position.playing);
        this.source.hidden = !position.sourceRange;
        this.source.disabled = !this.frame || this.pending;
        labelButton(this.play, position.playing ? 'Pause state transition' : 'Play state transition', position.playing ? 'debug-pause' : 'play');
    }
}
