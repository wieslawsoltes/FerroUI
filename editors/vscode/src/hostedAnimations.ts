import type {
    HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationPosition,
    HostedAnimationProgram, HostedAnimationRequest, HostedAnimationRestore, HostedAnimationState,
} from './hostedAnimationProtocol';
import type { HostedAnimationSample, HostedFrame } from './hostedProtocol';
import { drivePreviewClock } from './previewClock';

const empty: HostedAnimationCatalog = { programs: [], targets: [] };

interface Playback {
    revision: number;
    position: HostedAnimationPosition;
    targets?: number[];
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
        private readonly changed: (state: HostedAnimationState) => void, private readonly error: (error: unknown) => void) {}

    get state(): HostedAnimationState {
        return { ...this.catalog, frame: this.frame, selection: this.selection, position: this.playback?.position,
            overrides: this.overrides, busy: Boolean(this.pending) };
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
        if (restored) this.beginPlayback(frame.revision, restored, this.overrides[restored.program]);
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
        } else {
            const program = this.program(frame, action.program);
            if (action.action === 'select') {
                this.selection = program.id;
            } else if (action.action === 'targets') {
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
                if (this.playback?.position.program === program.id) {
                    await this.sample(frame.revision, { elapsed: 0, restart: true }, signal, () => {
                        this.playback = undefined;
                        assign();
                    });
                } else assign();
            } else {
                await this.start(frame.revision, program, action, signal);
            }
        }
        this.publish();
        return this.state;
    }

    private async start(revision: number, program: HostedAnimationProgram,
        action: Extract<HostedAnimationControl, { action: 'seek' | 'play' }>, signal: AbortSignal): Promise<void> {
        const targets = this.overrides[program.id];
        if (!(targets ?? program.defaultTargets).length) throw new Error('Choose controls to preview this animation.');
        const previous = this.playback?.position.program === program.id ? this.playback.position : undefined;
        const resume = action.action === 'play' && action.elapsed === undefined && previous && !previous.completed;
        const elapsed = action.elapsed ?? (resume ? previous.elapsed : 0);
        if ((action.action === 'seek' && action.elapsed === undefined) || !Number.isFinite(elapsed) || elapsed < 0
            || (!resume && program.totalDuration !== null && elapsed > program.totalDuration)) {
            throw new Error('Choose a time within the animation duration.');
        }
        await this.sample(revision, { program: program.id, targets, elapsed, restart: !resume }, signal, point => {
            this.selection = program.id;
            this.beginPlayback(revision, { program: program.id, elapsed: point.elapsed, completed: point.completed,
                playing: action.action === 'play' && !point.completed }, targets);
        });
    }

    private beginPlayback(revision: number, position: HostedAnimationPosition, targets?: number[]): void {
        const playback: Playback = this.playback = { revision, targets, position: { ...position } };
        if (position.playing) playback.loop = this.run(playback);
    }

    private async run(playback: Playback): Promise<void> {
        try {
            await drivePreviewClock(() => playback.position.playing, async seconds => {
                const point = await this.sample(playback.revision, { program: playback.position.program, targets: playback.targets,
                    elapsed: playback.position.elapsed + seconds, restart: false }, AbortSignal.timeout(15_000), point => {
                    playback.position = { ...playback.position, elapsed: point.elapsed, completed: point.completed,
                        playing: playback.position.playing && !point.completed };
                });
                return !point.completed;
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
        const previous = interrupt && this.playback?.position.playing && frame.sequence === this.previousSequence;
        if (frame.revision !== this.frame?.revision || (frame.sequence !== this.frame.sequence && !previous)) {
            throw new Error('Read the current app-hosted preview frame.');
        }
    }

    private publish(): void { this.changed(this.state); }
}
