import type { HostedAnimationControl, HostedAnimationFrame, HostedAnimationState } from '../src/hostedAnimationProtocol';
import type { HostedAnimationProgram } from '../src/hostedAnimationProtocol';
import type { FromHosted, ToHosted } from '../src/hostedProtocol';
import type { TimelineKeyframe, TimelineTrack } from '../src/timelineProtocol';
import { element, labelButton } from './dom';
import { HostedCurves } from './hostedCurves';
import { HostedKeyframeEditor } from './hostedKeyframes';
import { HostedKeyLanes } from './hostedKeyLanes';
import { clipRulerExtent, TimelineClips } from './timelineClips';
import { TimelineMarkerView } from './timelineMarkerView';
import { fitTimeline } from './timelineViewport';
import { TimelineWorkArea } from './timelineWorkArea';

export class HostedAnimationView {
    private readonly section = element('hosted-animations');
    private readonly programs = element<HTMLSelectElement>('hosted-animation-program');
    private readonly targets = element<HTMLButtonElement>('hosted-animation-targets');
    private readonly source = element<HTMLButtonElement>('hosted-animation-source');
    private readonly issues = element<HTMLButtonElement>('hosted-animation-issues');
    private readonly clipActions = element<HTMLButtonElement>('hosted-clip-actions');
    private readonly stateActions = element<HTMLButtonElement>('hosted-state-actions');
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
    readonly keyLanes: HostedKeyLanes;
    private readonly clips: TimelineClips<HostedAnimationFrame>;
    private readonly workArea: TimelineWorkArea;
    private readonly clipRange: TimelineWorkArea;
    private readonly programMarkers: TimelineMarkerView;
    private readonly clipMarkers: TimelineMarkerView;
    private rangeProgram?: string;
    private rangeScope?: string;
    readonly keyframes: HostedKeyframeEditor;

    constructor(private readonly send: (message: FromHosted) => void, compare: boolean, rememberComparison: (enabled: boolean) => void) {
        this.keyframes = new HostedKeyframeEditor(send, () => {
            // Applying a key retires the animation toolbar until the next frame.
            const button = this.edit.disabled || this.edit.hidden ? element('hosted-refresh') : this.edit;
            button.focus({ preventScroll: true });
        });
        this.curves = new HostedCurves(send, this.keyframes);
        this.keyLanes = new HostedKeyLanes(send, () => { this.flush(); this.paint(); },
            (track, key) => this.openLaneKey(track, key));
        this.workArea = new TimelineWorkArea('hosted', element('hosted-animation-clock'),
            () => fitTimeline(Number(this.seek.max)), () => this.seek.valueAsNumber, () => this.paint());
        this.clipRange = new TimelineWorkArea('clip', element('timeline-clips'),
            () => this.clips.viewport, () => this.clips.elapsed, () => this.paint());
        this.clips = new TimelineClips<HostedAnimationFrame>(compare, {
            changed: () => {
                this.queued = undefined;
                this.keyframes.close();
                rememberComparison(this.clips.enabled);
                this.paint();
            },
            seek: elapsed => this.queueSeek(elapsed),
            transport: action => {
                if (action !== 'play_all') this.control({ action }, true);
                else if (this.clipRange.range) this.control({ action: 'play_range', range: this.clipRange.range });
                else this.control({ action });
            },
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
            layout: () => { this.clipRange.layout(); this.clipMarkers.layout(); },
        });
        this.programMarkers = new TimelineMarkerView('hosted', this.seek,
            () => fitTimeline(Number(this.seek.max)), () => this.seek.valueAsNumber,
            marker => this.control({ action: 'marker', program: Number(this.programs.value), marker }));
        this.clipMarkers = new TimelineMarkerView('clip', element('clip-seek'), () => this.clips.viewport,
            () => this.clips.elapsed, (marker, time) => {
                if (time !== undefined) this.clips.reveal(time);
                this.control({ action: 'marker', marker });
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
        this.clipActions.onclick = () => this.openActions('clipActions');
        this.stateActions.onclick = () => this.openActions('stateActions');
        this.edit.onclick = () => this.choose(this.selectedProgram?.stateRoute ? 'editStateRoute' : 'chooseKeyframe');
        this.play.onclick = () => {
            if (this.state.position?.playing) this.control({ action: 'pause' }, true);
            else if (this.selectedProgram?.stateRoute?.endpoint) this.control({ action: 'seek', program: Number(this.programs.value), elapsed: 0 });
            else if (this.workArea.range) this.control({ action: 'play_range', program: Number(this.programs.value), range: this.workArea.range });
            else this.control({ action: 'play', program: Number(this.programs.value) });
        };
        this.stop.onclick = () => this.control({ action: 'stop' }, true);
        this.seek.oninput = () => this.queueSeek(this.seek.valueAsNumber);
        this.section.addEventListener('keydown', event => {
            if (event.key === 'Escape' && this.state.position && this.canUse(true)
                && !(event.target instanceof HTMLElement && event.target.closest('#hosted-key-lanes'))
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
        if (state.position?.previewRange && state.position.playing && !this.state.position?.playing) {
            this.clips.enabled = Boolean(state.position.clips);
        }
        this.state = state;
        this.documentVersion = documentVersion;
        const catalog = JSON.stringify(state.programs.map(program => [program.id, program.name, animationGroup(program)]));
        if (catalog !== this.catalog) {
            this.catalog = catalog;
            const choices = ['Clips', 'States', 'State routes'] as const;
            this.programs.replaceChildren(...choices.flatMap(label => {
                const options = state.programs.filter(program => animationGroup(program) === label);
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
            && (interrupt || !this.state.busy) && this.pending === undefined && !this.keyLanes.editing);
    }

    private openLaneKey(track: TimelineTrack, keyframe: TimelineKeyframe): void {
        const program = this.selectedProgram;
        const frame = this.state.frame;
        if (!program || !frame || !this.canUse()) return;
        this.keyframes.open({
            frame, version: this.documentVersion, program: program.id, duration: program.duration,
            target: track.target, property: track.property, keyframe,
            targetName: this.state.targets.find(target => target.id === track.target)?.label ?? `Target ${track.target}`,
        });
    }

    private get selectedProgram() {
        return this.state.programs.find(program => program.id === Number(this.programs.value));
    }

    private choose(type: 'animationTargets' | 'animationSource' | 'chooseKeyframe' | 'editStateRoute'): void {
        if (this.canUse() && this.decoded) this.send({ type, frame: this.decoded, program: Number(this.programs.value) });
    }

    private openActions(type: 'clipActions' | 'stateActions'): void {
        if (!this.canUse() || !this.decoded || this.state.position?.playing) return;
        const program = this.clips.visible ? undefined : this.selectedProgram?.id;
        this.send({ type, frame: this.decoded, request: this.pending = ++this.request, program });
        this.paint();
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
        const endpoint = Boolean(program?.stateRoute?.endpoint);
        const position = this.state.position;
        const playing = Boolean(position?.playing);
        const current = !position?.clips && position?.program === program?.id ? position : undefined;
        const usable = this.canUse() && !playing;
        const selectedTargets = program && (this.state.overrides[program.id] ?? program.defaultTargets);
        const available = Boolean(selectedTargets?.length);
        this.section.hidden = !program;
        const comparisonPosition = position && { ...position, all: Boolean(position.clips) };
        const clipRows = this.state.programs.flatMap(program => program.clip ? [{
            id: program.id, name: program.name, sourceKey: program.sourceKey, clip: program.clip, duration: program.duration, keys: true,
        }] : []);
        const playable = this.playableClips;
        const groupEnd = playable.reduce((end, program) => Math.max(end, program.totalDuration ?? Infinity), 0);
        const clipScope = this.state.clipScope;
        const clipPosition = position?.clips ? position : undefined;
        const clipMarkers = this.state.markers?.shared ?? [];
        const markerEnd = clipMarkers.at(-1)?.time ?? 0;
        this.clipRange.follow(this.state.canPreviewRanges ? groupEnd : 0, this.rangeScope !== clipScope, clipRulerExtent(clipRows));
        this.rangeScope = clipScope;
        this.clipRange.adopt(clipPosition?.previewRange, clipPosition?.playing === true);
        const clipElapsed = this.clips.enabled ? this.queued ?? this.submitted ?? clipPosition?.elapsed ?? 0 : 0;
        this.clips.follow({
            frame: this.state.frame, scope: clipScope, position: comparisonPosition, programs: clipRows,
            reference: playable[0]?.id, rangeEnabled: Boolean(this.clipRange.range),
            minimumDuration: Math.max(this.clipRange.range?.end ?? 0, markerEnd),
        }, !this.canUse(), false, clipElapsed);
        this.clipRange.show(!usable, playing ? clipPosition?.previewRange : undefined);
        this.clipMarkers.show(clipMarkers, !usable || !playable.length, Boolean(this.state.markers));
        this.programs.disabled = !usable;
        this.programs.title = program?.customAnimator ? `${program.name} · Registered animator` : program?.name ?? '';
        this.targets.hidden = Boolean(program?.stateRoute);
        this.targets.disabled = !usable || !program?.compatibleTargets.length || this.targets.hidden;
        labelButton(this.targets, selectedTargets?.length ? `Choose preview targets (${selectedTargets.length} selected)` : 'Choose preview targets');
        this.source.disabled = !usable || program?.sourceOffset == null;
        this.issues.hidden = !this.state.sourceIssues?.length;
        this.issues.disabled = !this.canUse();
        const authoring = this.state.clipAuthoring;
        this.clipActions.hidden = !authoring?.owners.length && !authoring?.programs.length;
        this.clipActions.disabled = !usable;
        const states = this.state.stateAuthoring;
        this.stateActions.hidden = !states?.owners.some(owner => owner.canCreate) && !states?.groups.length;
        this.stateActions.disabled = !usable;
        labelButton(this.source, program?.sourceOffset == null ? 'Animation source is unavailable'
            : program.stateRoute ? 'Reveal visual state in source' : 'Reveal animation in source');
        this.edit.hidden = endpoint || (!program?.authoring && !program?.stateRoute);
        this.edit.disabled = !usable;
        labelButton(this.edit, program?.stateRoute ? 'Edit state route timing' : 'Edit application keyframe');
        const keyframeUsable = usable && !this.clips.visible;
        this.keyframes.follow(keyframeUsable ? this.state.frame : undefined, this.documentVersion, program?.id);
        this.keyLanes.follow(program, this.state.targets, this.state.markerScope,
            keyframeUsable ? this.state.frame : undefined, Boolean(this.state.keyframeSelection) && !this.clips.visible, this.documentVersion);
        this.curves.follow(program, this.state.targets.filter(target => selectedTargets?.includes(target.id)),
            this.state.frame, this.documentVersion, keyframeUsable, Boolean(this.state.canShowVelocity));
        this.play.disabled = playing ? !this.canUse(true) : !usable || !available;
        this.stop.disabled = !position || !this.canUse(true);
        // Keep the slider focused while the next sample is decoded. Retain only the latest seek.
        this.seek.disabled = endpoint || !this.state.frame || !available || playing;
        element('hosted-animation-clock').hidden = endpoint;
        const elapsed = this.queued ?? this.submitted ?? current?.elapsed ?? 0;
        const programMarkers = program ? this.state.markers?.programs[program.id] ?? [] : [];
        const horizon = Math.max(program?.duration ?? 0, program?.clip?.start ?? 0, elapsed, programMarkers.at(-1)?.time ?? 0);
        const rangeProgram = program && (program.sourceKey ?? String(program.id));
        this.workArea.follow(this.state.canPreviewRanges && available && !endpoint ? program?.totalDuration ?? Infinity : 0,
            this.rangeProgram !== rangeProgram, horizon);
        this.rangeProgram = rangeProgram;
        this.workArea.adopt(current?.previewRange, current?.playing === true);
        this.seek.max = String(program?.totalDuration ?? Math.max(horizon, this.workArea.range?.end ?? 0));
        this.workArea.show(!usable, playing ? current?.previewRange : undefined);
        const playLabel = this.workArea.range ? 'Play preview range'
            : current && !current.completed ? 'Resume application animation' : 'Play application animation';
        labelButton(this.play, endpoint ? 'Preview state' : playing ? current?.previewRange ? 'Pause preview range' : 'Pause application animation' : playLabel,
            endpoint ? 'preview' : playing ? 'debug-pause' : 'play');
        this.seek.value = String(elapsed);
        this.programMarkers.show(programMarkers, !usable || !available, Boolean(this.state.markers));
        this.seek.setAttribute('aria-valuetext', `${this.number.format(elapsed)} seconds elapsed`);
        const limit = program?.totalDuration;
        this.time.value = limit == null || elapsed > limit ? `${this.number.format(elapsed)} s elapsed`
            : `${this.number.format(elapsed)} / ${this.number.format(limit)} s`;
    }
}

function animationGroup(program: HostedAnimationProgram): 'Clips' | 'States' | 'State routes' {
    if (program.stateRoute?.endpoint) return 'States';
    return program.stateRoute ? 'State routes' : 'Clips';
}
