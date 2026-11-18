import type {
    HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationPosition,
    HostedAnimationProgram, HostedAnimationRequest, HostedAnimationState,
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
    private overrides: Record<number, number[]> = {};
    private pending?: object;

    constructor(private readonly render: (request: HostedAnimationRequest, signal: AbortSignal) => Promise<HostedAnimationSample>,
        private readonly changed: (state: HostedAnimationState) => void, private readonly error: (error: unknown) => void) {}

    get state(): HostedAnimationState {
        return { ...this.catalog, frame: this.frame, position: this.playback?.position, overrides: this.overrides, busy: Boolean(this.pending) };
    }

    get driving(): boolean { return Boolean(this.pending || this.playback?.position.playing); }

    accept(frame: HostedFrame): void {
        if (frame.revision !== this.frame?.revision) {
            this.reset();
            this.catalog = frame.animations ?? empty;
        }
        this.previousSequence = this.frame?.sequence;
        this.frame = { revision: frame.revision, sequence: frame.sequence };
        this.publish();
    }

    invalidate(): void {
        this.reset();
        this.publish();
    }

    private reset(): void {
        if (this.playback) this.playback.position.playing = false;
        this.frame = this.playback = this.pending = this.previousSequence = undefined;
        this.overrides = {};
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
        }
        if (this.pending || this.playback?.position.playing) throw new Error('Pause the current animation first.');
        if (action.action === 'pause') return this.state;
        if (action.action === 'stop') {
            await this.sample(frame.revision, { elapsed: 0, restart: true }, signal);
            this.playback = undefined;
        } else {
            const program = this.program(frame, action.program);
            if (action.action === 'targets') {
                const targets = action.targets ?? [];
                if (new Set(targets).size !== targets.length || targets.some(id => !program.compatibleTargets.includes(id))) {
                    throw new Error('Choose distinct compatible preview controls.');
                }
                if (this.playback?.position.program === program.id) {
                    await this.sample(frame.revision, { elapsed: 0, restart: true }, signal);
                    this.playback = undefined;
                }
                this.overrides = { ...this.overrides };
                if (targets.length) this.overrides[program.id] = targets;
                else delete this.overrides[program.id];
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
        const point = await this.sample(revision, { program: program.id, targets, elapsed, restart: !resume }, signal);
        const playing = action.action === 'play' && !point.completed;
        const playback: Playback = this.playback = { revision, targets,
            position: { program: program.id, elapsed: point.elapsed, completed: point.completed, playing } };
        if (playing) playback.loop = this.run(playback);
    }

    private async run(playback: Playback): Promise<void> {
        try {
            await drivePreviewClock(() => playback.position.playing, async seconds => {
                const point = await this.sample(playback.revision, { program: playback.position.program, targets: playback.targets,
                    elapsed: playback.position.elapsed + seconds, restart: false }, AbortSignal.timeout(15_000));
                playback.position = { ...playback.position, elapsed: point.elapsed, completed: point.completed };
                this.publish();
                return !point.completed;
            });
        } catch (error) {
            if (this.playback === playback) this.error(error);
        } finally {
            playback.position.playing = false;
            playback.loop = undefined;
            if (this.playback === playback) this.publish();
        }
    }

    private async sample(revision: number, request: Omit<HostedAnimationRequest, 'revision' | 'sequence'>,
        signal: AbortSignal): Promise<HostedAnimationSample> {
        if (this.frame?.revision !== revision || this.pending) throw new Error('Read the current app-hosted preview frame.');
        const pending = this.pending = {};
        this.publish();
        try {
            const sample = await this.render({ ...this.frame, ...request }, signal);
            if (this.pending !== pending || this.frame?.revision !== revision) throw new Error('The app-hosted source changed during playback.');
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
