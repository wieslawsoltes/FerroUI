import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type FrameTarget } from './protocol';
import type { StateTransitionControl, StateTransitionRoute } from './stateTransitionProtocol';
import { choose, StudioContext } from './studioContext';
import { chooseKeyframeGroupEdit, keyframeOptions } from './studioKeyframes';
import { chooseAnimationTargets, showAnimationActions } from './studioPrograms';
import { authorTimelineKeyframe } from './timelineKeyframeAuthoring';
import { positionFromPose, TimelinePlayback } from './timelinePlayback';
import { canPreviewProgram, previewClock } from './timelinePrograms';
import { TimelineTransitions } from './timelineTransitions';
import type { FromTimeline, KeyframeLocation, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from './timelineProtocol';
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
    private readonly transitions: TimelineTransitions;
    private panel?: DesignerPanel;
    private file = '';
    private catalog?: Catalog;
    private shown?: string;

    constructor(root: vscode.Uri, private readonly changed: () => void) {
        this.view = new DockView(root, timelineHtml, changed, message => { void this.receive(message); });
        this.playback = new TimelinePlayback(position => {
            this.view.post({ type: 'position', position, frame: this.frame() });
            if (!position?.busy) this.refresh();
        }, error => this.report(error));
        this.transitions = new TimelineTransitions(() => {
            this.shown = undefined;
            this.refresh();
        }, error => this.report(error));
    }

    update(panel: DesignerPanel | undefined, file: string): void {
        if (this.panel !== panel) this.panel?.setAnimationRecording(false);
        this.panel = panel;
        this.file = file;
        if (panel?.snapshot?.image && !panel.authoredTimelinePose) panel.setAnimationRecording(false);
        this.playback.follow(panel);
        this.transitions.follow(panel);
        this.refresh();
    }

    recordingTime(panel: DesignerPanel | undefined): number | undefined {
        return panel?.animationRecordingTime;
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: TimelineControl, signal: AbortSignal) {
        if (this.transitions.position) throw new Error('Return to animations before controlling a program.');
        if ((control.action === 'pause' || control.action === 'stop')
            && this.playback.position?.session === panel.session && this.playback.position.program === control.program) {
            return this.playback.control(panel, frame, control, undefined, signal);
        }
        if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
        const program = (await this.programs(panel).pending).find(entry => entry.id === control.program);
        if (!program) throw new Error('Choose an authored animation program from the current document.');
        if (control.action === 'play_all' && !panel.supportsTimelineCombinedPlayback) throw new Error('Update the FerroUI preview helper to play all animations.');
        if (control.action === 'record') {
            if (typeof control.enabled !== 'boolean') throw new Error('Choose whether to record property edits.');
            if (!program.timeline || !panel.supportsTimelineRecording || this.panel !== panel) throw new Error('Use the current XAML designer to record property edits.');
            if (control.enabled) {
                const time = control.time ?? (panel.authoredTimelinePose?.program === program.id ? panel.authoredTimelinePose.time : 0);
                await this.playback.control(panel, frame, { action: 'seek', program: program.id, time }, program.timeline, signal);
                if (!panel.authoredTimelinePose || this.panel !== panel) throw new Error('The recording frame is no longer current.');
                panel.setAnimationRecording(true);
            } else {
                if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
                panel.setAnimationRecording(false);
            }
            this.shown = undefined;
            this.changed();
            return { session: panel.session, program: program.id, time: panel.authoredTimelinePose?.time ?? 0,
                playing: false, busy: false, recording: panel.animationRecordingTime !== undefined };
        }
        if ((control.action === 'play' || control.action === 'seek') && !canPreviewProgram(program)) {
            throw new Error('Choose live targets for this animation preview.');
        }
        return this.playback.control(panel, frame, control, previewClock(program), signal);
    }

    async openTransition(context: StudioContext, route: StateTransitionRoute): Promise<void> {
        context.ensure();
        await this.controlTransition(context.panel, { session: context.panel.session, token: context.token },
            { action: 'open', ...route }, AbortSignal.timeout(15_000));
        await vscode.commands.executeCommand(`${TIMELINE_VIEW}.focus`);
    }

    controlTransition(panel: DesignerPanel, frame: FrameTarget, control: StateTransitionControl, signal: AbortSignal) {
        if (this.panel !== panel || !panel.supportsStateTransitionPreview) throw new Error('Open a current designer with native transition preview.');
        if (control.action === 'open' && panel.canUse(frame)) this.playback.follow();
        return this.transitions.control(panel, frame, control, signal);
    }

    private programs(panel: DesignerPanel): Catalog {
        const block = panel.snapshot?.activeBlock?.start;
        const key = `${panel.session}:${panel.document.version}:${block?.line ?? 0}:${block?.character ?? 0}:${panel.timelineCatalogEpoch}`;
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
        const transition = this.transitions.position;
        if (transition && panel) {
            this.view.show({ frame: frame && { session: panel.session, token: transition.revision }, documentVersion: panel.document.version,
                transition, programs: [], names: {}, writable: false }, this.file);
            return;
        }
        const pose = panel?.snapshot?.timelinePose;
        const position = this.playback.position ?? (panel && pose ? positionFromPose(panel.session, pose) : undefined);
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
        this.view.show({ frame: catalog?.programs ? frame : undefined, documentVersion: panel?.document.version,
            programs, message, names, writable: panel?.language === 'xaml', position,
            canEditKeyframes: panel?.supportsTimelineKeyframeDrafts,
            canPlayAll: Boolean(panel?.supportsTimelineCombinedPlayback && programs.some(canPreviewProgram)),
            canRecord: panel?.supportsTimelineRecording, canRecordGeometry: panel?.supportsTimelineGeometryRecording,
            recording: this.recordingTime(panel) !== undefined }, this.file);
    }

    private async receive(message: FromTimeline): Promise<void> {
        const panel = this.panel;
        if (!panel || message.type === 'ready' || message.frame.session !== panel.session) return;
        try {
            if (message.type === 'control') {
                await this.control(panel, message.frame, message.control, AbortSignal.timeout(15_000));
            } else if (message.type === 'transitionControl') {
                await this.controlTransition(panel, message.frame, message.control, AbortSignal.timeout(15_000));
            } else {
                const context = new StudioContext(panel, message.frame.token, () => this.panel === panel);
                if (message.type === 'keyframeDraft') {
                    this.view.post(await authorTimelineKeyframe(context, message, () => this.programs(panel).pending));
                    return;
                }
                const program = (await this.programs(panel).pending).find(entry => entry.id === message.program);
                if (message.type === 'programTargets' || message.type === 'revealProgram') {
                    if (!program) throw new Error('Choose a current animation program.');
                    if (message.type === 'programTargets') await chooseAnimationTargets(context, program);
                    else await context.reveal(program.id);
                    return;
                }
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
                    const key = await this.recordKeyframe(context, program, timeline, message.time, message.track);
                    if (key) await panel.waitForSnapshot(AbortSignal.timeout(15_000));
                    this.view.post({ type: 'keyframesEdited', keys: key ? [key] : undefined, before: message.frame });
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
        time: number, index?: number): Promise<KeyframeLocation | undefined> {
        if (!context.writable) throw new Error('Recording into an authored program requires XAML.');
        if (!Number.isFinite(time) || time < 0 || time > timeline.duration) throw new Error('Choose a time within the animation duration.');
        const tracks = timeline.tracks;
        let track = tracks[index ?? 0];
        if (index !== undefined && !track) throw new Error('Choose a current animation track.');
        if (index === undefined && tracks.length > 1) {
            const nodes = new Map(context.panel.nodes.map(node => [node.id, node.name ?? node.label]));
            const picked = await choose('Record a keyframe at the playhead', tracks.map(candidate => ({
                label: candidate.property, description: nodes.get(candidate.target), track: candidate,
            })));
            if (!picked) return;
            track = picked.track;
        }
        const target = track?.target ?? context.selection.find(node => program.targets.includes(node)) ?? program.targets[0] ?? program.owner;
        const property = track?.property ?? 'Opacity';
        await context.edit('designer_program', { action: 'record_keyframe', program: program.id, target,
            property, time });
        return { target, property, time };
    }

    private report(error: unknown): void {
        this.view.post({ type: 'error', message: errorMessage(error) });
    }

    dispose(): void {
        this.playback.follow();
        this.transitions.follow();
        this.view.dispose();
    }
}
