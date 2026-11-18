import { setTimeout as delay } from 'node:timers/promises';
import type { DesignerPanel } from './designerPanel';
import type { FrameTarget } from './protocol';
import type { TimelineControl, TimelinePosition } from './timelineProtocol';

interface Playback {
    panel: DesignerPanel;
    revision: number;
    token: number;
    duration: number;
    running: boolean;
    position: TimelinePosition;
}

/** One accepted native render at a time, including the final frame after Pause. */
export class TimelinePlayback {
    private playback?: Playback;

    constructor(private readonly update: (position: TimelinePosition | undefined) => void,
        private readonly reportError: (error: unknown) => void) {}

    get position(): TimelinePosition | undefined { return this.playback?.position; }

    follow(panel?: DesignerPanel): void {
        const playback = this.playback;
        if (!playback || (playback.panel === panel && panel.document.version === playback.revision)) return;
        playback.running = false;
        this.playback = undefined;
        this.update(undefined);
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: TimelineControl, duration: number, signal: AbortSignal): Promise<TimelinePosition> {
        signal.throwIfAborted();
        const existing = this.playback;
        if (control.action === 'pause' && existing?.panel === panel && existing.position.program === control.program) {
            if (frame.session !== panel.session || existing.revision !== panel.document.version) throw new Error('Read the current timeline first.');
            existing.running = false;
            return this.commit(existing, { playing: false });
        }
        if (!panel.canUse(frame)) throw new Error('Read the current timeline first.');
        if (!Number.isFinite(duration) || duration < 0) throw new Error('The program has no finite duration.');
        if (control.action === 'seek' && !Number.isFinite(control.time)) throw new Error('Supply a seek time in seconds.');
        if (existing?.position.busy) throw new Error('Wait for the current timeline frame.');
        if (existing) existing.running = false;
        const previous = existing?.panel === panel && existing.revision === panel.document.version
            && existing.position.program === control.program ? existing.position.time : 0;
        if (control.action === 'pause') return { session: panel.session, program: control.program, time: previous, playing: false, busy: false };
        let time = control.time ?? previous;
        if (!Number.isFinite(time) || time < 0 || time > duration) throw new Error('Choose a time within the animation duration.');
        const running = control.action === 'play' && duration > 0;
        if (running && time === duration) time = 0;
        const playback: Playback = {
            panel, revision: panel.document.version, token: frame.token, duration, running,
            position: { session: panel.session, program: control.program, time: previous, playing: running, busy: true },
        };
        this.playback = playback;
        this.update(playback.position);
        if (!running) {
            try { await this.render(playback, time, signal); }
            finally { this.commit(playback, { playing: false, busy: false }); }
            return playback.position;
        }
        void this.play(playback, time);
        return playback.position;
    }

    private async play(playback: Playback, initialTime: number): Promise<void> {
        const start = performance.now();
        let time = initialTime;
        try {
            while (playback.running) {
                await this.render(playback, time, AbortSignal.timeout(15_000));
                if (!playback.running || time === playback.duration) break;
                // Sample wall time after each accepted frame. Slow rendering drops
                // intermediate samples and never builds up a queue of old frames.
                await delay(16);
                time = Math.min(playback.duration, initialTime + (performance.now() - start) / 1000);
            }
        } catch (error) {
            if (this.playback === playback && playback.running) this.reportError(error);
        } finally {
            playback.running = false;
            this.commit(playback, { playing: false, busy: false });
        }
    }

    private async render(playback: Playback, time: number, signal: AbortSignal): Promise<TimelinePosition> {
        const { panel, token, revision } = playback;
        if (this.playback !== playback || panel.document.version !== revision || panel.currentToken !== token) {
            throw new Error('The design changed during animation playback.');
        }
        await panel.studioPresent(token, 'designer_program', { action: 'preview', programs: [playback.position.program], time },
            undefined, signal);
        if (panel.document.version !== revision || panel.currentToken !== token + 1) throw new Error('The preview changed during playback.');
        playback.token = token + 1;
        return this.commit(playback, { time });
    }

    private commit(playback: Playback, change: Partial<TimelinePosition>): TimelinePosition {
        playback.position = { ...playback.position, ...change };
        if (this.playback === playback) this.update(playback.position);
        return playback.position;
    }
}
