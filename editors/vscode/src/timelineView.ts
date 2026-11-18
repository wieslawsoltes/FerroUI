import type * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type FrameTarget } from './protocol';
import { choose, StudioContext } from './studioContext';
import { chooseKeyframeGroupEdit, keyframeOptions } from './studioKeyframes';
import { showAnimationActions } from './studioPrograms';
import { TimelinePlayback } from './timelinePlayback';
import type { FromTimeline, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from './timelineProtocol';
import { timelineHtml } from './view';

export const TIMELINE_VIEW = 'ferroui.timeline';

interface Catalog {
    key: string;
    session: number;
    token: number | undefined;
    pending: Promise<TimelineProgram[]>;
    programs?: TimelineProgram[];
    retained?: TimelineProgram[];
    error?: string;
}

export class TimelineView implements vscode.Disposable {
    readonly view: DockView<TimelineState, FromTimeline, ToTimeline>;
    private readonly playback: TimelinePlayback;
    private panel?: DesignerPanel;
    private recordingPanel?: DesignerPanel;
    private file = '';
    private catalog?: Catalog;
    private shown?: string;

    constructor(root: vscode.Uri, private readonly changed: () => void) {
        this.view = new DockView(root, timelineHtml, changed, message => { void this.receive(message); });
        this.playback = new TimelinePlayback(position => {
            this.view.post({ type: 'position', position, frame: this.frame() });
            if (!position?.busy) this.refresh();
        }, error => this.report(error));
    }

    update(panel: DesignerPanel | undefined, file: string): void {
        this.panel = panel;
        this.file = file;
        if (this.recordingPanel !== panel || (panel?.snapshot?.image && !panel.authoredTimelinePose)) this.recordingPanel = undefined;
        this.playback.follow(panel);
        this.refresh();
    }

    recordingTime(panel: DesignerPanel | undefined): number | undefined {
        return this.recordingPanel === panel ? panel?.authoredTimelinePose?.time : undefined;
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: TimelineControl, signal: AbortSignal) {
        if ((control.action === 'pause' || control.action === 'stop')
            && this.playback.position?.session === panel.session && this.playback.position.program === control.program) {
            return this.playback.control(panel, frame, control, undefined, signal);
        }
        if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
        const program = (await this.programs(panel).pending).find(entry => entry.id === control.program);
        if (!program?.timeline) throw new Error('Choose an authored animation program from the current document.');
        if (control.action === 'record') {
            if (typeof control.enabled !== 'boolean') throw new Error('Choose whether to record property edits.');
            if (!panel.supportsTimelineRecording || this.panel !== panel) throw new Error('Use the current XAML designer to record property edits.');
            if (control.enabled) {
                const time = control.time ?? (panel.authoredTimelinePose?.program === program.id ? panel.authoredTimelinePose.time : 0);
                await this.playback.control(panel, frame, { action: 'seek', program: program.id, time }, program.timeline, signal);
                if (!panel.authoredTimelinePose || this.panel !== panel) throw new Error('The recording frame is no longer current.');
                this.recordingPanel = panel;
            } else {
                if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
                this.recordingPanel = undefined;
            }
            this.shown = undefined;
            this.changed();
            return { session: panel.session, program: program.id, time: panel.authoredTimelinePose?.time ?? 0,
                playing: false, busy: false, recording: this.recordingPanel === panel };
        }
        return this.playback.control(panel, frame, control, program.timeline, signal);
    }

    private programs(panel: DesignerPanel): Catalog {
        const block = panel.snapshot?.activeBlock?.start;
        const key = `${panel.session}:${panel.document.version}:${block?.line ?? 0}:${block?.character ?? 0}`;
        if (this.catalog?.key === key && (!this.catalog.error || this.catalog.token === panel.currentToken)) return this.catalog;
        const pending = panel.studioQuery('designer_programs', { include_source: false, include_keyframe_text: true },
            undefined, AbortSignal.timeout(15_000)).then(result => (result as { programs: TimelineProgram[] }).programs);
        const previous = this.catalog?.session === panel.session ? this.catalog : undefined;
        const catalog: Catalog = this.catalog = { key, session: panel.session, token: panel.currentToken, pending,
            retained: previous?.programs ?? previous?.retained };
        void pending.then(programs => { catalog.programs = programs; this.catalogChanged(catalog); },
            error => { catalog.error = errorMessage(error); this.catalogChanged(catalog); });
        return catalog;
    }

    private catalogChanged(catalog: Catalog): void {
        if (this.catalog !== catalog) return;
        this.shown = undefined;
        this.refresh();
    }

    private frame(): FrameTarget | undefined {
        const panel = this.panel;
        const token = panel?.currentToken;
        if (panel && token !== undefined) return { session: panel.session, token };
    }

    private refresh(): void {
        if (!this.view.visible) { this.shown = undefined; return; }
        const panel = this.panel;
        const frame = this.frame();
        const pose = panel?.snapshot?.timelinePose;
        const position = this.playback.position ?? (panel && pose ? {
            session: panel.session, program: pose.program, time: pose.time, playing: false, busy: false,
        } : undefined);
        if (position?.busy && frame?.session === position.session) return;
        const key = `${panel?.session}:${frame?.token}:${panel?.status ?? ''}`;
        if (key === this.shown) return;
        this.shown = key;
        const catalog = panel && frame && !panel.status ? this.programs(panel) : undefined;
        const retained = panel && this.catalog?.session === panel.session ? this.catalog : undefined;
        const programs = catalog?.programs ?? retained?.programs ?? retained?.retained ?? [];
        const message = !panel ? 'Open a FerroUI designer.' : panel.status ?? catalog?.error
            ?? (catalog?.programs ? undefined : 'Loading animation programs…');
        const names = Object.fromEntries((panel?.nodes ?? []).map(node => [node.id, node.name ?? node.label]));
        this.view.show({ frame: catalog?.programs ? frame : undefined, programs, message, names, writable: panel?.language === 'xaml', position,
            canRecord: panel?.supportsTimelineRecording, recording: this.recordingTime(panel) !== undefined }, this.file);
    }

    private async receive(message: FromTimeline): Promise<void> {
        const panel = this.panel;
        if (!panel || message.type === 'ready' || message.frame.session !== panel.session) return;
        try {
            if (message.type === 'control') {
                await this.control(panel, message.frame, message.control, AbortSignal.timeout(15_000));
            } else {
                const context = new StudioContext(panel, message.frame.token, () => this.panel === panel);
                const program = (await this.programs(panel).pending).find(entry => entry.id === message.program);
                if (message.type === 'programActions') {
                    if (message.program !== undefined && !program) throw new Error('Choose a current animation program.');
                    context.ensure();
                    await showAnimationActions(context, program);
                    if (this.catalog?.session === panel.session) this.catalog.key = '';
                    this.shown = undefined;
                    this.refresh();
                    return;
                }
                const timeline = program?.timeline;
                if (!program || !timeline) throw new Error('Choose a typed animation program from the current document.');
                context.ensure();
                if (message.type === 'recordKeyframe') {
                    await this.recordKeyframe(context, program, timeline, message.time);
                    return;
                }
                if (message.type === 'keyframeGroup') {
                    if (panel.language !== 'xaml' || !message.keys.length) throw new Error('Select authored XAML keyframes to edit.');
                    const edit = message.edit ?? await chooseKeyframeGroupEdit(message.keys.length);
                    if (!edit) {
                        this.view.post({ type: 'keyframesEdited', before: message.frame });
                        return;
                    }
                    await context.edit('designer_program', { ...edit, program: program.id, keyframes: message.keys });
                    await panel.waitForSnapshot(AbortSignal.timeout(15_000));
                    const keys = edit.action === 'remove_keyframes' ? [] : message.keys.map(key => ({ ...key, time: key.time + edit.offset }));
                    this.view.post({ type: 'keyframesEdited', keys, before: message.frame });
                    return;
                }
                const track = timeline.tracks[message.track];
                const frame = track?.keyframes.find(entry => entry.time === message.time);
                if (!track || !frame) throw new Error('Select a keyframe from the current timeline.');
                await keyframeOptions(context, program.id, timeline.duration, track, frame, async () => {
                    await this.control(panel, message.frame, { action: 'seek', program: program.id, time: frame.time },
                        AbortSignal.timeout(15_000));
                });
            }
        } catch (error) { this.report(error); }
    }

    private async recordKeyframe(context: StudioContext, program: TimelineProgram, timeline: NonNullable<TimelineProgram['timeline']>,
        time: number): Promise<void> {
        if (!context.writable) throw new Error('Recording into an authored program requires XAML.');
        if (!Number.isFinite(time) || time < 0 || time > timeline.duration) throw new Error('Choose a time within the animation duration.');
        const tracks = timeline.tracks;
        let track = tracks[0];
        if (tracks.length > 1) {
            const nodes = new Map(context.panel.nodes.map(node => [node.id, node.name ?? node.label]));
            const picked = await choose('Record a keyframe at the playhead', tracks.map(candidate => ({
                label: candidate.property, description: nodes.get(candidate.target), track: candidate,
            })));
            if (!picked) return;
            track = picked.track;
        }
        const target = track?.target ?? context.selection.find(node => program.targets.includes(node)) ?? program.targets[0] ?? program.owner;
        await context.edit('designer_program', { action: 'record_keyframe', program: program.id, target,
            property: track?.property ?? 'Opacity', time });
    }

    private report(error: unknown): void {
        this.view.post({ type: 'error', message: errorMessage(error) });
    }

    dispose(): void {
        this.playback.follow();
        this.view.dispose();
    }
}
