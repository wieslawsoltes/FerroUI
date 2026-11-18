import type { DesignerPanel } from './designerPanel';
import { drivePreviewClock } from './previewClock';
import type { FrameTarget } from './protocol';
import type { StateTransitionControl, StateTransitionPosition, StateTransitionRoute } from './stateTransitionProtocol';

interface Preview {
    panel: DesignerPanel;
    version: number;
    position: StateTransitionPosition;
    requesting: boolean;
    interrupting: boolean;
    loop?: Promise<void>;
}

type NativeRequest = ({ action: 'preview_transition'; time: number } & StateTransitionRoute)
    | { action: 'advance_preview'; seconds: number }
    | { action: 'clear_preview' };

/** Retain one source-scoped native state transition in the Timeline dock. */
export class TimelineTransitions {
    private preview?: Preview;

    constructor(private readonly changed: () => void, private readonly report: (error: unknown) => void) {}

    get position(): StateTransitionPosition | undefined { return this.preview?.position; }

    follow(panel?: DesignerPanel): void {
        const preview = this.preview;
        if (!preview || (preview.panel === panel && panel.document.version === preview.version
            && panel.currentToken !== undefined && (preview.requesting || panel.currentToken === preview.position.revision))) return;
        this.preview = undefined;
        preview.position.playing = false;
        this.changed();
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: StateTransitionControl,
        signal: AbortSignal): Promise<StateTransitionPosition | undefined> {
        signal.throwIfAborted();
        if (control.action === 'open') {
            const { owner, group, from, to, time } = control;
            return this.open(panel, frame, { owner, group, from: from ?? null, to }, time ?? 0, signal);
        }
        const preview = this.preview;
        if (!preview || preview.panel !== panel || panel.document.version !== preview.version || frame.session !== panel.session) {
            throw new Error('Open a transition on the current designer frame.');
        }
        const token = preview.position.revision;
        const interrupt = control.action === 'pause' || control.action === 'stop';
        if (frame.token !== token && !(interrupt && preview.loop && frame.token === token - 1)) {
            throw new Error('Read the current transition frame.');
        }
        if (preview.interrupting || (preview.position.busy && (!interrupt || !preview.loop))) {
            throw new Error('Wait for the current transition frame.');
        }
        if (interrupt && preview.loop) {
            preview.interrupting = true;
            this.commit(preview, { playing: false });
            await preview.loop;
            preview.interrupting = false;
        }
        this.commit(preview, { busy: true });
        try {
            if (control.action === 'stop') {
                await this.render(preview, { action: 'clear_preview' }, signal);
                this.follow();
                return;
            }
            if (control.action === 'seek') {
                await this.seek(preview, control.time, signal);
            } else if (control.action === 'play' && preview.position.duration > 0) {
                if (preview.position.time >= preview.position.duration) await this.seek(preview, 0, signal);
                this.commit(preview, { playing: true });
                preview.loop = this.advance(preview);
            }
        } finally {
            if (!preview.loop) this.commit(preview, { busy: false });
        }
        return preview.position;
    }

    private async open(panel: DesignerPanel, frame: FrameTarget, route: StateTransitionRoute, time: number,
        signal: AbortSignal): Promise<StateTransitionPosition> {
        if (!panel.canUse(frame) || this.preview?.position.busy) throw new Error('Wait for the current transition frame.');
        const preview: Preview = { panel, version: panel.document.version, requesting: false, interrupting: false,
            position: { route, time: 0, duration: 0, playing: false, busy: true, revision: frame.token } };
        this.preview = preview;
        this.changed();
        try {
            await this.seek(preview, time, signal);
        } catch (error) {
            if (this.preview === preview) this.follow();
            throw error;
        }
        this.commit(preview, { busy: false });
        return preview.position;
    }

    private seek(preview: Preview, time: number, signal: AbortSignal): Promise<void> {
        return this.render(preview, { action: 'preview_transition', ...preview.position.route, time }, signal);
    }

    private async advance(preview: Preview): Promise<void> {
        try {
            await drivePreviewClock(() => this.preview === preview && preview.position.playing, async seconds => {
                await this.render(preview, { action: 'advance_preview', seconds }, AbortSignal.timeout(15_000));
                return preview.position.time < preview.position.duration;
            });
        } catch (error) {
            if (this.preview === preview && preview.position.playing) this.report(error);
        } finally {
            preview.loop = undefined;
            if (!preview.interrupting) this.commit(preview, { playing: false, busy: false });
        }
    }

    private async render(preview: Preview, request: NativeRequest, signal: AbortSignal): Promise<void> {
        const { panel, position: { revision } } = preview;
        if (this.preview !== preview || panel.document.version !== preview.version || panel.currentToken !== revision) {
            throw new Error('The designer changed during the transition preview.');
        }
        preview.requesting = true;
        try {
            const frame = await panel.studioPresent(revision, 'designer_state', request, undefined, signal);
            if (this.preview !== preview || panel.document.version !== preview.version || panel.currentToken !== revision + 1) {
                throw new Error('The designer changed during the transition preview.');
            }
            if (request.action !== 'clear_preview' && !frame?.transitionPosition) {
                throw new Error('The native helper did not return the transition position.');
            }
            this.commit(preview, { ...frame?.transitionPosition, revision: revision + 1 });
        } finally {
            preview.requesting = false;
        }
    }

    private commit(preview: Preview, change: Partial<StateTransitionPosition>): void {
        preview.position = { ...preview.position, ...change };
        if (this.preview === preview) this.changed();
    }
}
