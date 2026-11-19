import type {
    HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationPosition,
    HostedAnimationProgram, HostedAnimationRequest, HostedAnimationRestore, HostedAnimationState,
} from './hostedAnimationProtocol';
import type { HostedAnimationSample, HostedFrame } from './hostedProtocol';
import { drivePreviewClock } from './previewClock';
import { stepPreviewRange, validPreviewRange } from './timelinePreviewRange';

const empty: HostedAnimationCatalog = { programs: [], targets: [] };

type Bindings = Pick<HostedAnimationRequest, 'program' | 'targets' | 'clips'>;
type PlaybackAction = Extract<HostedAnimationControl, { action: 'seek' | 'play' | 'scrub_all' | 'play_all' | 'play_range' }>;

interface Playback {
    revision: number;
    position: HostedAnimationPosition;
    bindings: Bindings;
    loop?: Promise<void>;
}

/** One transport for the app preview toolbar and MCP, fenced to an accepted source frame. */
export class HostedAnimations {
    private catalog = empty;
    private frame?: HostedAnimationFrame;
    private previousSequence?: number;
    private playback?: Playback;
    private selection?: number;
    private overrides: Record<number, number[]> = {};
    private pending?: object;
    private restoration?: HostedAnimationRestore;

    constructor(private readonly render: (request: HostedAnimationRequest, signal: AbortSignal) => Promise<HostedAnimationSample>,
        private readonly present: (frame: HostedFrame) => void,
        private readonly changed: (state: HostedAnimationState) => void, private readonly error: (error: unknown) => void,
        private readonly supportsRanges = () => true) {}

    get state(): HostedAnimationState {
        return { ...this.catalog, frame: this.frame, selection: this.selection, position: this.playback?.position,
            overrides: this.overrides, busy: Boolean(this.pending), canPreviewRanges: this.supportsRanges() };
    }

    get driving(): boolean { return Boolean(this.pending || this.playback?.position.playing); }
    get restore(): HostedAnimationRestore | undefined { return this.restoration; }

    accept(frame: HostedFrame): void {
        let restored: HostedAnimationPosition | undefined;
        if (frame.revision !== this.frame?.revision) {
            this.reset();
            this.catalog = frame.animations ?? empty;
            const state = frame.animationPresentation;
            this.selection = state?.selection ?? state?.position?.program;
            this.overrides = state?.overrides ?? {};
            restored = state?.position;
        }
        this.previousSequence = this.frame?.sequence;
        this.frame = { revision: frame.revision, sequence: frame.sequence };
        if (restored) this.beginPlayback(frame.revision, restored);
        this.publish();
    }

    invalidate(): void {
        this.reset();
        this.publish();
    }

    beforeSourceUpdate(): void {
        const frame = this.frame;
        if (frame && (this.selection !== undefined || this.playback || Object.keys(this.overrides).length)) {
            // Keep the last accepted position through failed and superseded source revisions.
            const position = this.playback?.position;
            this.restoration = { frame, presentation: { selection: this.selection, overrides: this.overrides,
                position: position && { ...position } } };
        }
        this.retireFrame();
        this.publish();
    }

    private reset(): void {
        this.retireFrame();
        this.playback = this.selection = this.restoration = undefined;
        this.overrides = {};
    }

    private retireFrame(): void {
        if (this.playback) this.playback.position.playing = false;
        this.frame = this.pending = this.previousSequence = undefined;
    }

    program(frame: HostedAnimationFrame, id: number): HostedAnimationProgram {
        this.requireFrame(frame);
        const program = this.catalog.programs.find(program => program.id === id);
        if (!program) throw new Error('Choose an animation from the current application preview.');
        return program;
    }

    async control(frame: HostedAnimationFrame, action: HostedAnimationControl, signal: AbortSignal): Promise<HostedAnimationState> {
        signal.throwIfAborted();
        if (action.action === 'play_range' && !this.supportsRanges()) {
            throw new Error('Update the FerroUI preview helper to use preview ranges in App Preview.');
        }
        const interrupt = action.action === 'pause' || action.action === 'stop';
        this.requireFrame(frame, interrupt);
        if (interrupt && this.playback?.loop && this.playback.position.playing) {
            this.playback.position.playing = false;
            this.publish();
            await this.playback.loop;
            if (this.frame?.revision !== frame.revision) throw new Error('Read the current app-hosted preview frame.');
        }
        if (this.pending || this.playback?.position.playing) throw new Error('Pause the current animation first.');
        if (action.action === 'pause') return this.state;
        if (action.action === 'stop') {
            await this.sample(frame.revision, { elapsed: 0, restart: true }, signal, () => { this.playback = undefined; });
        } else if (action.action === 'play_range') {
            const programs = action.program === undefined ? this.playableClips() : [this.program(frame, action.program)];
            await this.start(frame.revision, programs, action, signal);
        } else if (action.action === 'play_all' || action.action === 'scrub_all') {
            await this.start(frame.revision, this.playableClips(), action, signal);
        } else {
            const program = this.program(frame, action.program);
            if (action.action === 'select') {
                this.selection = program.id;
            } else if (action.action === 'targets') {
                if (program.stateRoute) throw new Error('A state route uses its authored targets.');
                const targets = action.targets ?? [];
                if (new Set(targets).size !== targets.length || targets.some(id => !program.compatibleTargets.includes(id))) {
                    throw new Error('Choose distinct compatible preview controls.');
                }
                const assign = () => {
                    this.selection = program.id;
                    this.overrides = { ...this.overrides };
                    if (targets.length) this.overrides[program.id] = targets;
                    else delete this.overrides[program.id];
                };
                const position = this.playback?.position;
                if (position?.program === program.id || position?.clips?.includes(program.id)) {
                    await this.sample(frame.revision, { elapsed: 0, restart: true }, signal, () => {
                        this.playback = undefined;
                        assign();
                    });
                } else assign();
            } else {
                await this.start(frame.revision, [program], action, signal);
            }
        }
        this.publish();
        return this.state;
    }

    private playableClips(): HostedAnimationProgram[] {
        return this.catalog.programs.filter(program =>
            program.clip && (this.overrides[program.id] ?? program.defaultTargets).length);
    }

    private async start(revision: number, programs: HostedAnimationProgram[], action: PlaybackAction, signal: AbortSignal): Promise<void> {
        const first = programs[0];
        if (!first || !(this.overrides[first.id] ?? first.defaultTargets).length) {
            throw new Error('Choose controls to preview this animation.');
        }
        const clips = 'program' in action && action.program !== undefined ? undefined : programs.map(program => program.id);
        const end = programs.reduce((end, program) => Math.max(end, program.totalDuration ?? Infinity), 0);
        const range = action.action === 'play_range' ? action.range : undefined;
        if (action.action === 'play_range' && !validPreviewRange(range, end)) {
            throw new Error('Choose an In and Out time within the preview clock, with In before Out.');
        }
        const playing = action.action === 'play' || action.action === 'play_all' || range !== undefined;
        const previous = this.playback?.position;
        const sameClips = clips ? previous?.clips?.length === clips.length && clips.every((id, index) => id === previous.clips![index])
            : !previous?.clips;
        const prior = previous?.program === first.id && sameClips ? previous : undefined;
        const requested = 'elapsed' in action ? action.elapsed : undefined;
        const resume = !range && playing && requested === undefined && prior !== undefined
            && (prior.previewRange ? prior.elapsed < end : !prior.completed);
        const elapsed = range ? (prior && prior.elapsed >= range.start && prior.elapsed < range.end ? prior.elapsed : range.start)
            : requested ?? (resume ? prior.elapsed : 0);
        if ((!playing && requested === undefined) || !Number.isFinite(elapsed) || elapsed < 0 || (!resume && elapsed > end)) {
            throw new Error('Choose a time within the animation duration.');
        }
        const position = { program: first.id, ...(clips && { clips }), ...(range && { previewRange: range }) };
        await this.sample(revision, { ...this.bindings(position), elapsed, restart: !resume }, signal, point => {
            if (!clips) this.selection = first.id;
            const completed = range ? false : point.completed;
            this.beginPlayback(revision, { ...position, elapsed: point.elapsed, completed, playing: playing && !completed });
        });
    }

    private bindings({ program, clips }: Pick<HostedAnimationPosition, 'program' | 'clips'>): Bindings {
        return clips ? { clips: clips.map(program => ({ program, targets: this.overrides[program] })) }
            : { program, targets: this.overrides[program] };
    }

    private beginPlayback(revision: number, position: HostedAnimationPosition): void {
        const playback: Playback = this.playback = { revision, bindings: this.bindings(position), position: { ...position } };
        if (position.playing) playback.loop = this.run(playback);
    }

    private async run(playback: Playback): Promise<void> {
        try {
            await drivePreviewClock(() => playback.position.playing, async seconds => {
                const { previewRange: range, elapsed } = playback.position;
                const next = range && stepPreviewRange(range, elapsed, seconds);
                const wrap = Boolean(range?.loop && seconds >= range.end - elapsed);
                await this.sample(playback.revision, { ...playback.bindings,
                    elapsed: next?.time ?? elapsed + seconds, restart: wrap }, AbortSignal.timeout(15_000), point => {
                    const completed = next?.completed ?? point.completed;
                    playback.position = { ...playback.position, elapsed: point.elapsed, completed,
                        playing: playback.position.playing && !completed };
                });
                return playback.position.playing;
            });
        } catch (error) {
            if (this.playback === playback && this.frame?.revision === playback.revision) this.error(error);
        } finally {
            playback.position.playing = false;
            playback.loop = undefined;
            if (this.playback === playback) this.publish();
        }
    }

    private async sample(revision: number, request: Omit<HostedAnimationRequest, 'revision' | 'sequence'>,
        signal: AbortSignal, accepted: (sample: HostedAnimationSample) => void): Promise<HostedAnimationSample> {
        if (this.frame?.revision !== revision || this.pending) throw new Error('Read the current app-hosted preview frame.');
        const pending = this.pending = {};
        this.publish();
        try {
            const sample = await this.render({ ...this.frame, ...request }, signal);
            if (this.pending !== pending || this.frame?.revision !== revision) throw new Error('The app-hosted source changed during playback.');
            // Commit the clock and the corresponding frame together, before a source
            // edit can capture the previous time under the new frame's identity.
            accepted(sample);
            this.present(sample.frame);
            return sample;
        } finally {
            if (this.pending === pending) {
                this.pending = undefined;
                this.publish();
            }
        }
    }

    private requireFrame(frame: HostedAnimationFrame, interrupt = false): void {
        const position = this.playback?.position;
        // A one-shot can reach Out while the preceding frame's Pause is in flight.
        const previous = interrupt && (position?.playing || (position?.previewRange && position.completed))
            && frame.sequence === this.previousSequence;
        if (frame.revision !== this.frame?.revision || (frame.sequence !== this.frame.sequence && !previous)) {
            throw new Error('Read the current app-hosted preview frame.');
        }
    }

    private publish(): void { this.changed(this.state); }
}
