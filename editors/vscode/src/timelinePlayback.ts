import type { DesignerPanel } from './designerPanel';
import { drivePreviewClock } from './previewClock';
import type { FrameTarget } from './protocol';
import { stepPreviewRange, validPreviewRange } from './timelinePreviewRange';
import type { TimelineClock, TimelinePosition, TimelinePreviewPose, TimelineTransportControl } from './timelineProtocol';

interface Playback {
    panel: DesignerPanel;
    revision: number;
    token: number;
    position: TimelinePosition;
    loop?: Promise<void>;
    running: boolean;
    interrupting: boolean;
}

export function positionFromPose(session: number, pose: TimelinePreviewPose): TimelinePosition {
    return { session, program: pose.program, time: pose.time, playing: false, busy: false,
        elapsed: pose.playback?.elapsed, all: pose.playback?.all, completed: pose.playback?.completed };
}

/** The extension measures elapsed time while Studio owns the clock and native values. */
export class TimelinePlayback {
    private playback?: Playback;

    constructor(private readonly update: (position: TimelinePosition | undefined) => void,
        private readonly reportError: (error: unknown) => void) {}

    get position(): TimelinePosition | undefined { return this.playback?.position; }

    follow(panel?: DesignerPanel): void {
        const playback = this.playback;
        if (!playback || (playback.panel === panel && panel.document.version === playback.revision && panel.currentToken !== undefined)) return;
        playback.running = false;
        this.playback = undefined;
        this.update(undefined);
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: TimelineTransportControl, timeline: TimelineClock | undefined,
        signal: AbortSignal): Promise<TimelinePosition> {
        signal.throwIfAborted();
        const existing = this.playback;
        const current = existing?.panel === panel && existing.revision === panel.document.version
            && existing.position.program === control.program ? existing : undefined;
        if ((control.action === 'pause' || control.action === 'stop') && current?.loop) {
            // The button can still show the preceding frame while the next one is
            // crossing the webview boundary. Both belong to this native clock.
            if (frame.session !== panel.session || (frame.token !== current.token && frame.token !== current.token - 1)) {
                throw new Error('Read the current timeline first.');
            }
            return this.interrupt(current, control.action, signal);
        }
        if (control.action === 'pause' && current?.position.completed && current.position.previewRange
            && frame.session === panel.session && frame.token === current.token - 1
            && panel.canUse({ session: panel.session, token: current.token })) {
            // A one-shot range can reach Out while Pause is crossing the same clock's frame boundary.
            return current.position;
        }
        if (!panel.canUse(frame)) throw new Error('Read the current timeline first.');
        if (existing?.position.busy) throw new Error('Wait for the current timeline frame.');
        const idle = { session: panel.session, program: control.program, time: 0, playing: false, busy: false };
        const pose = panel.authoredTimelinePose;
        const previous = current?.token === frame.token ? current.position
            : pose?.program === control.program ? positionFromPose(panel.session, pose) : undefined;
        if (control.action === 'pause' && previous?.elapsed === undefined) return previous ?? idle;
        const playback: Playback = {
            panel, revision: panel.document.version, token: frame.token, position: previous ?? idle, running: false, interrupting: false,
        };
        this.playback = playback;
        this.commit(playback, { busy: true });
        try {
            if (control.action === 'stop') {
                await this.render(playback, { action: 'stop_program' }, signal);
            } else if (control.action === 'pause') {
                await this.render(playback, { action: 'pause_program', program: control.program }, signal);
            } else if (control.action === 'scrub_all') {
                if (!Number.isFinite(control.time) || control.time < 0) throw new Error('Choose a nonnegative elapsed playback time.');
                await this.render(playback, control, signal);
            } else if (control.action === 'seek' && timeline) {
                const limit = timeline.native ? timeline.total_duration : timeline.duration;
                if (!Number.isFinite(control.time) || control.time < 0 || (limit !== null && control.time > limit)) {
                    throw new Error('Choose a time within the animation duration.');
                }
                await this.render(playback, { action: 'scrub', program: control.program, time: control.time }, signal);
            } else if (control.action === 'play' && timeline) {
                await this.start(playback, control.time, previous?.all ? undefined : previous, timeline, signal);
            } else if (control.action === 'play_range' && timeline && !timeline.native) {
                if (!validPreviewRange(control.range, timeline.duration)) throw new Error('Choose an In and Out time within the animation, with In before Out.');
                const range = control.range;
                const prior = previous?.all ? range.start : previous?.time ?? range.start;
                const time = prior >= range.start && prior < range.end ? prior : range.start;
                panel.setAnimationRecording(false);
                const position = await this.render(playback, { action: 'scrub', program: control.program, time }, signal,
                    { previewRange: range, playing: true, completed: false });
                this.beginClock(playback, position);
            } else if (control.action === 'play_all') {
                if (control.time !== undefined && (!Number.isFinite(control.time) || control.time < 0)) {
                    throw new Error('Choose a nonnegative elapsed playback time.');
                }
                const time = control.time ?? (previous?.all && !previous.completed ? undefined : 0);
                const position = await this.render(playback, { action: 'play_all', program: control.program, time }, signal);
                this.beginClock(playback, position);
            } else {
                throw new Error('Choose an authored animation program from the current document.');
            }
        } finally {
            if (!playback.loop) this.commit(playback, { busy: false });
        }
        return playback.position;
    }

    private async start(playback: Playback, time: number | undefined, previous: TimelinePosition | undefined,
        timeline: TimelineClock, signal: AbortSignal): Promise<void> {
        const end = timeline.total_duration;
        if (time !== undefined && (!Number.isFinite(time) || time < 0 || (end !== null && time > end))) {
            throw new Error('Choose a time within the total playback duration.');
        }
        const resume = previous?.elapsed !== undefined && !previous.completed && (end === null || previous.elapsed < end);
        const limit = timeline.native ? end : timeline.duration;
        const fromPlayhead = previous?.elapsed === undefined && previous !== undefined && previous.time > 0
            && (limit === null || previous.time < limit);
        const request = { action: 'play_program', program: playback.position.program };
        const position = await this.render(playback, time !== undefined
            ? { ...request, time: time === end ? 0 : time }
            : resume ? request : { ...request, time: fromPlayhead ? previous.time : 0, from_playhead: fromPlayhead && !timeline.native }, signal);
        this.beginClock(playback, position);
    }

    private beginClock(playback: Playback, position: TimelinePosition): void {
        if (position.playing) {
            playback.running = true;
            playback.loop = this.advance(playback);
        }
    }

    private async advance(playback: Playback): Promise<void> {
        try {
            await drivePreviewClock(() => playback.running, async seconds => {
                const { previewRange: range, program, time } = playback.position;
                if (range) {
                    const next = stepPreviewRange(range, time, seconds);
                    const position = await this.render(playback, { action: 'scrub', program, time: next.time }, AbortSignal.timeout(15_000),
                        { previewRange: range, playing: !next.completed, completed: next.completed });
                    return position.playing;
                }
                const position = await this.render(playback,
                    { action: 'advance_program', program, seconds }, AbortSignal.timeout(15_000));
                return position.playing;
            });
        } catch (error) {
            if (this.playback === playback && playback.running) this.reportError(error);
        } finally {
            playback.running = false;
            playback.loop = undefined;
            if (!playback.interrupting) this.commit(playback, { playing: false, busy: false });
        }
    }

    private async interrupt(playback: Playback, action: 'pause' | 'stop', signal: AbortSignal): Promise<TimelinePosition> {
        if (playback.interrupting) throw new Error('Wait for the current timeline action.');
        playback.interrupting = true;
        playback.running = false;
        this.commit(playback, { playing: false });
        try {
            await playback.loop;
            signal.throwIfAborted();
            if (action === 'stop' || !playback.position.previewRange) {
                await this.render(playback, action === 'pause'
                    ? { action: 'pause_program', program: playback.position.program }
                    : { action: 'stop_program' }, signal);
            }
        } finally {
            playback.interrupting = false;
            this.commit(playback, { busy: false });
        }
        return playback.position;
    }

    private async render(playback: Playback, request: object, signal: AbortSignal,
        range?: Pick<TimelinePosition, 'previewRange' | 'completed' | 'playing'>): Promise<TimelinePosition> {
        const { panel, token, revision } = playback;
        if (this.playback !== playback || panel.document.version !== revision || panel.currentToken !== token) {
            throw new Error('The design changed during animation playback.');
        }
        const presentation = await panel.studioPresent(token, 'designer_program', request, undefined, signal);
        if (this.playback !== playback || panel.document.version !== revision || panel.currentToken !== token + 1) {
            throw new Error('The preview changed during playback.');
        }
        if (!presentation?.timelinePosition) throw new Error('The native preview did not report a timeline position.');
        playback.token = token + 1;
        const { elapsed, time, all, completed, playing } = presentation.timelinePosition;
        return this.commit(playback, { elapsed, time, all, completed, previewRange: undefined,
            ...range, playing: (range?.playing ?? playing) && !playback.interrupting });
    }

    private commit(playback: Playback, change: Partial<TimelinePosition>): TimelinePosition {
        playback.position = { ...playback.position, ...change };
        if (this.playback === playback) this.update(playback.position);
        return playback.position;
    }
}
