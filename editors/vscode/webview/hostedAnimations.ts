import type { HostedAnimationControl, HostedAnimationFrame, HostedAnimationState } from '../src/hostedAnimationProtocol';
import type { FromHosted } from '../src/hostedProtocol';
import { element, labelButton } from './dom';
import { HostedKeyframeEditor } from './hostedKeyframes';

export class HostedAnimationView {
    private readonly section = element('hosted-animations');
    private readonly programs = element<HTMLSelectElement>('hosted-animation-program');
    private readonly targets = element<HTMLButtonElement>('hosted-animation-targets');
    private readonly source = element<HTMLButtonElement>('hosted-animation-source');
    private readonly edit = element<HTMLButtonElement>('hosted-animation-keyframe');
    private readonly play = element<HTMLButtonElement>('hosted-animation-play');
    private readonly stop = element<HTMLButtonElement>('hosted-animation-stop');
    private readonly seek = element<HTMLInputElement>('hosted-animation-seek');
    private readonly time = element<HTMLOutputElement>('hosted-animation-time');
    private readonly number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
    private state: HostedAnimationState = { programs: [], targets: [], overrides: {}, busy: false };
    private catalog = '';
    private decoded?: HostedAnimationFrame;
    private queued?: number;
    private submitted?: number;
    private request = 0;
    private pending?: number;
    private documentVersion = 0;
    readonly keyframes: HostedKeyframeEditor;

    constructor(private readonly send: (message: FromHosted) => void) {
        this.keyframes = new HostedKeyframeEditor(send, () => {
            // Applying a key retires the animation toolbar until the next frame.
            const button = this.edit.disabled || this.edit.hidden ? element('hosted-refresh') : this.edit;
            button.focus({ preventScroll: true });
        });
        this.programs.onchange = () => {
            this.queued = undefined;
            this.keyframes.close();
            this.control({ action: 'select', program: Number(this.programs.value) });
        };
        this.targets.onclick = () => this.choose('animationTargets');
        this.source.onclick = () => this.choose('animationSource');
        this.edit.onclick = () => this.choose('chooseKeyframe');
        this.play.onclick = () => {
            if (this.state.position?.playing) this.control({ action: 'pause' }, true);
            else this.control({ action: 'play', program: Number(this.programs.value) });
        };
        this.stop.onclick = () => this.control({ action: 'stop' }, true);
        this.seek.oninput = () => {
            this.queued = this.seek.valueAsNumber;
            this.flush();
            this.paint();
        };
        this.section.addEventListener('keydown', event => {
            if (event.key === 'Escape' && !this.stop.disabled) {
                event.preventDefault();
                event.stopPropagation();
                this.control({ action: 'stop' }, true);
            }
        });
    }

    update(state: HostedAnimationState, documentVersion: number): void {
        if (state.frame?.revision !== this.state.frame?.revision) {
            this.queued = this.submitted = this.pending = undefined;
        }
        const selected = state.selection ?? state.position?.program;
        this.state = state;
        this.documentVersion = documentVersion;
        const catalog = JSON.stringify(state.programs.map(({ id, name }) => [id, name]));
        if (catalog !== this.catalog) {
            this.catalog = catalog;
            this.programs.replaceChildren(...state.programs.map(({ name, id }) => new Option(name, String(id))));
        }
        this.programs.value = String(state.programs.some(program => program.id === selected)
            ? selected : state.programs[0]?.id ?? '');
        this.flush();
        this.paint();
    }

    rendered(frame: HostedAnimationFrame): void {
        this.decoded = frame;
        this.flush();
        this.paint();
    }

    reply(request: number): void {
        if (request !== this.pending) return;
        this.pending = this.submitted = undefined;
        this.flush();
        this.paint();
    }

    private canUse(interrupt = false): boolean {
        const frame = this.state.frame;
        const decoded = this.decoded;
        return Boolean(frame && decoded && frame.revision === decoded.revision
            && (frame.sequence === decoded.sequence || (interrupt && this.state.position?.playing))
            && (interrupt || !this.state.busy) && this.pending === undefined);
    }

    private choose(type: 'animationTargets' | 'animationSource' | 'chooseKeyframe'): void {
        if (this.canUse() && this.decoded) this.send({ type, frame: this.decoded, program: Number(this.programs.value) });
    }

    private control(control: HostedAnimationControl, interrupt = false): void {
        const frame = interrupt ? this.state.frame : this.decoded;
        if (!frame || !this.canUse(interrupt)) return;
        const request = this.pending = ++this.request;
        this.send({ type: 'animation', frame, request, control });
        this.paint();
    }

    private flush(): void {
        if (this.queued === undefined || this.state.position?.playing || !this.canUse()) return;
        const elapsed = this.submitted = this.queued;
        this.queued = undefined;
        this.control({ action: 'seek', program: Number(this.programs.value), elapsed });
    }

    private paint(): void {
        const program = this.state.programs.find(program => program.id === Number(this.programs.value));
        const position = this.state.position;
        const playing = Boolean(position?.playing);
        const current = position?.program === program?.id ? position : undefined;
        const usable = this.canUse() && !playing;
        const selectedTargets = program && (this.state.overrides[program.id] ?? program.defaultTargets);
        const available = Boolean(selectedTargets?.length);
        this.section.hidden = !program;
        this.programs.disabled = !usable;
        this.programs.title = program?.customAnimator ? `${program.name} · Registered animator` : program?.name ?? '';
        this.targets.disabled = !usable || !program?.compatibleTargets.length;
        labelButton(this.targets, selectedTargets?.length ? `Choose preview targets (${selectedTargets.length} selected)` : 'Choose preview targets');
        this.source.disabled = !usable;
        this.edit.hidden = !program?.authoring;
        this.edit.disabled = !usable;
        this.keyframes.follow(usable ? this.state.frame : undefined, this.documentVersion, program?.id);
        this.play.disabled = playing ? !this.canUse(true) : !usable || !available;
        labelButton(this.play, playing ? 'Pause application animation' : current && !current.completed ? 'Resume application animation' : 'Play application animation',
            playing ? 'debug-pause' : 'play');
        this.stop.disabled = !position || !this.canUse(true);
        // Keep the slider focused while the next sample is decoded. Retain only the latest seek.
        this.seek.disabled = !this.state.frame || !available || playing;
        const elapsed = this.queued ?? this.submitted ?? current?.elapsed ?? 0;
        this.seek.max = String(program?.totalDuration ?? Math.max(program?.duration ?? 0, elapsed));
        this.seek.value = String(elapsed);
        this.seek.setAttribute('aria-valuetext', `${this.number.format(elapsed)} seconds elapsed`);
        const limit = program?.totalDuration;
        this.time.value = limit == null || elapsed > limit ? `${this.number.format(elapsed)} s elapsed`
            : `${this.number.format(elapsed)} / ${this.number.format(limit)} s`;
    }
}
