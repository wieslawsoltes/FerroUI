import type { HostedAnimationControl, HostedAnimationFrame, HostedAnimationState } from '../src/hostedAnimationProtocol';
import type { FromHosted, ToHosted } from '../src/hostedProtocol';
import { element, labelButton } from './dom';
import { HostedCurves } from './hostedCurves';
import { HostedKeyframeEditor } from './hostedKeyframes';
import { TimelineClips } from './timelineClips';

export class HostedAnimationView {
    private readonly section = element('hosted-animations');
    private readonly programs = element<HTMLSelectElement>('hosted-animation-program');
    private readonly targets = element<HTMLButtonElement>('hosted-animation-targets');
    private readonly source = element<HTMLButtonElement>('hosted-animation-source');
    private readonly issues = element<HTMLButtonElement>('hosted-animation-issues');
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
    private readonly curves: HostedCurves;
    private readonly clips: TimelineClips<HostedAnimationFrame>;
    readonly keyframes: HostedKeyframeEditor;

    constructor(private readonly send: (message: FromHosted) => void, compare: boolean, rememberComparison: (enabled: boolean) => void) {
        this.keyframes = new HostedKeyframeEditor(send, () => {
            // Applying a key retires the animation toolbar until the next frame.
            const button = this.edit.disabled || this.edit.hidden ? element('hosted-refresh') : this.edit;
            button.focus({ preventScroll: true });
        });
        this.curves = new HostedCurves(send, this.keyframes);
        this.clips = new TimelineClips<HostedAnimationFrame>(compare, {
            changed: () => {
                this.queued = undefined;
                this.keyframes.close();
                rememberComparison(this.clips.enabled);
                this.paint();
            },
            seek: elapsed => this.queueSeek(elapsed),
            transport: action => action === 'play_all'
                ? this.control({ action }) : this.control({ action }, true),
            place: (frame, programs, start, gap) => {
                if (!this.canUse() || this.state.position?.playing) return;
                this.queued = undefined;
                this.send({ type: 'clipPlacement', frame, request: this.pending = ++this.request, draft: { programs, start, gap } });
                this.paint();
            },
            keys: program => {
                rememberComparison(false);
                this.control({ action: 'select', program });
            },
        });
        this.programs.onchange = () => {
            this.queued = undefined;
            this.keyframes.close();
            this.control({ action: 'select', program: Number(this.programs.value) });
        };
        this.targets.onclick = () => this.choose('animationTargets');
        this.source.onclick = () => this.choose('animationSource');
        this.issues.onclick = () => {
            if (this.canUse() && this.decoded) this.send({ type: 'animationIssues', frame: this.decoded });
        };
        this.edit.onclick = () => this.choose(this.selectedProgram?.stateRoute ? 'editStateRoute' : 'chooseKeyframe');
        this.play.onclick = () => {
            if (this.state.position?.playing) this.control({ action: 'pause' }, true);
            else this.control({ action: 'play', program: Number(this.programs.value) });
        };
        this.stop.onclick = () => this.control({ action: 'stop' }, true);
        this.seek.oninput = () => this.queueSeek(this.seek.valueAsNumber);
        this.section.addEventListener('keydown', event => {
            if (event.key === 'Escape' && this.state.position && this.canUse(true)
                && !this.section.querySelector(':popover-open, .clip-bar[data-dragging]')) {
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
        if (state.position?.clips && !this.state.position?.clips) this.clips.enabled = true;
        this.state = state;
        this.documentVersion = documentVersion;
        const catalog = JSON.stringify(state.programs.map(({ id, name, stateRoute }) => [id, name, Boolean(stateRoute)]));
        if (catalog !== this.catalog) {
            this.catalog = catalog;
            const choices = [[false, 'Clips'], [true, 'State routes']] as const;
            this.programs.replaceChildren(...choices.flatMap(([route, label]) => {
                const options = state.programs.filter(program => Boolean(program.stateRoute) === route);
                if (!options.length) return [];
                const group = document.createElement('optgroup');
                group.label = label;
                group.append(...options.map(({ name, id }) => new Option(name, String(id))));
                return [group];
            }));
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

    curve(reply: Extract<ToHosted, { type: 'curveReply' }>) { this.curves.receive(reply); }

    private canUse(interrupt = false): boolean {
        const frame = this.state.frame;
        const decoded = this.decoded;
        return Boolean(frame && decoded && frame.revision === decoded.revision
            && (frame.sequence === decoded.sequence || (interrupt && this.state.position?.playing))
            && (interrupt || !this.state.busy) && this.pending === undefined);
    }

    private get selectedProgram() {
        return this.state.programs.find(program => program.id === Number(this.programs.value));
    }

    private choose(type: 'animationTargets' | 'animationSource' | 'chooseKeyframe' | 'editStateRoute'): void {
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
        if (this.clips.visible) {
            const end = this.playableClips.reduce((end, program) => Math.max(end, program.totalDuration ?? Infinity), 0);
            this.control({ action: 'scrub_all', elapsed: Math.min(elapsed, end) });
        } else this.control({ action: 'seek', program: Number(this.programs.value), elapsed });
    }

    private queueSeek(elapsed: number): void {
        this.queued = elapsed;
        this.flush();
        this.paint();
    }

    private get playableClips() {
        return this.state.programs.filter(program => program.clip && (this.state.overrides[program.id] ?? program.defaultTargets).length);
    }

    private paint(): void {
        const program = this.selectedProgram;
        const position = this.state.position;
        const playing = Boolean(position?.playing);
        const current = !position?.clips && position?.program === program?.id ? position : undefined;
        const usable = this.canUse() && !playing;
        const selectedTargets = program && (this.state.overrides[program.id] ?? program.defaultTargets);
        const available = Boolean(selectedTargets?.length);
        this.section.hidden = !program;
        const comparisonPosition = position && { ...position, all: Boolean(position.clips) };
        this.clips.follow({
            frame: this.state.frame, scope: this.state.clipScope, position: comparisonPosition,
            reference: this.playableClips[0]?.id,
            programs: this.state.programs.flatMap(program => program.clip ? [{
                id: program.id, name: program.name, sourceKey: program.sourceKey, clip: program.clip, duration: program.duration, keys: true,
            }] : []),
        }, !this.canUse(), false, this.clips.enabled ? this.queued ?? this.submitted ?? (position?.clips ? position.elapsed : 0) : 0);
        this.programs.disabled = !usable;
        this.programs.title = program?.customAnimator ? `${program.name} · Registered animator` : program?.name ?? '';
        this.targets.hidden = Boolean(program?.stateRoute);
        this.targets.disabled = !usable || !program?.compatibleTargets.length || this.targets.hidden;
        labelButton(this.targets, selectedTargets?.length ? `Choose preview targets (${selectedTargets.length} selected)` : 'Choose preview targets');
        this.source.disabled = !usable || program?.sourceOffset == null;
        this.issues.hidden = !this.state.sourceIssues?.length;
        this.issues.disabled = !this.canUse();
        labelButton(this.source, program?.sourceOffset == null ? 'Animation source is unavailable'
            : program.stateRoute ? 'Reveal Rust state route in source' : 'Reveal animation in source');
        this.edit.hidden = !program?.authoring && !program?.stateRoute;
        this.edit.disabled = !usable;
        labelButton(this.edit, program?.stateRoute ? 'Edit state route timing' : 'Edit application keyframe');
        const keyframeUsable = usable && !this.clips.visible;
        this.keyframes.follow(keyframeUsable ? this.state.frame : undefined, this.documentVersion, program?.id);
        this.curves.follow(program, this.state.targets.filter(target => selectedTargets?.includes(target.id)),
            this.state.frame, this.documentVersion, keyframeUsable, Boolean(this.state.canShowVelocity));
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
