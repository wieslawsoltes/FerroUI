import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type FrameTarget } from './protocol';
import type { StateTransitionControl, StateTransitionRoute } from './stateTransitionProtocol';
import { choose, StudioContext } from './studioContext';
import { chooseKeyframeGroupEdit, keyframeOptions } from './studioKeyframes';
import { chooseAnimationIssue, chooseAnimationTargets, revealAnimationProgram, showAnimationActions } from './studioPrograms';
import { authorTimelineKeyframe } from './timelineKeyframeAuthoring';
import { TimelineMarkers } from './timelineMarkers';
import { positionFromPose, TimelinePlayback } from './timelinePlayback';
import { canPreviewProgram, canPreviewRange, previewClock } from './timelinePrograms';
import { TimelineTransitions } from './timelineTransitions';
import type { AnimationSourceIssue, CurvePoint, FromTimeline, KeyframeLocation, TimelineControl, TimelineMarkerEdit, TimelineProgram, TimelineState, ToTimeline } from './timelineProtocol';
import { timelineHtml } from './view';

export const TIMELINE_VIEW = 'ferroui.timeline';

interface Catalog {
    key: string;
    session: number;
    token: number | undefined;
    pending: Promise<TimelineProgram[]>;
    programs?: TimelineProgram[];
    sourceIssues?: AnimationSourceIssue[];
    retained?: TimelineProgram[];
    error?: string;
}

export class TimelineView implements vscode.Disposable {
    readonly view: DockView<TimelineState, FromTimeline, ToTimeline>;
    private readonly playback: TimelinePlayback;
    private readonly transitions: TimelineTransitions;
    private readonly markers = new TimelineMarkers();
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

    async previewMarkers(panel: DesignerPanel, frame: FrameTarget, id: number, signal: AbortSignal, edit?: TimelineMarkerEdit) {
        signal.throwIfAborted();
        if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
        const program = (await this.programs(panel).pending).find(entry => entry.id === id);
        signal.throwIfAborted();
        if (!panel.canUse(frame) || !program || !canPreviewRange(program)) throw new Error('Choose a typed clip from the current frame.');
        if (edit) {
            if (this.playback.position?.session === panel.session && this.playback.position.busy) {
                throw new Error('Pause playback before changing markers.');
            }
            this.markers.edit(panel, program, edit);
            this.shown = undefined;
            this.refresh();
        }
        return { revision: frame.token, markers: this.markers.list(panel, program) };
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
        if (control.action === 'play_range' && !canPreviewRange(program)) {
            throw new Error('Choose a typed animation with live targets to preview a range.');
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
            undefined, AbortSignal.timeout(15_000)).then(result => {
                const authored = result as { programs: TimelineProgram[]; sourceIssues?: AnimationSourceIssue[] };
                catalog.sourceIssues = authored.sourceIssues;
                return authored.programs;
            });
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
        const selectedTargets = panel?.selectedIds;
        const key = `${panel?.session}:${frame?.token}:${panel?.status ?? ''}:${selectedTargets?.join(',') ?? ''}`;
        if (key === this.shown) return;
        this.shown = key;
        const catalog = panel && frame && !panel.status ? this.programs(panel) : undefined;
        const retained = panel && this.catalog?.session === panel.session ? this.catalog : undefined;
        const programs = catalog?.programs ?? retained?.programs ?? retained?.retained ?? [];
        const message = !panel ? 'Open a FerroUI designer.' : panel.status ?? catalog?.error
            ?? (catalog?.programs ? undefined : 'Loading animation programs…');
        const names = Object.fromEntries((panel?.nodes ?? []).map(node => [node.id, node.name ?? node.label]));
        const markers = panel && catalog?.programs
            ? Object.fromEntries(programs.map(program => [program.id, this.markers.list(panel, program)])) : undefined;
        this.view.show({ frame: catalog?.programs ? frame : undefined, documentVersion: panel?.document.version,
            programs, message, names, selectedTargets, markers, writable: panel?.language === 'xaml', position, sourceIssues: catalog?.sourceIssues,
            canEditKeyframes: panel?.supportsTimelineKeyframeDrafts,
            canShowCurves: panel?.supportsTimelineCurves,
            curveCatalog: panel?.supportsTimelineCurves && catalog?.programs ? catalog.key : undefined,
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
            } else if (message.type === 'markerEdit') {
                await this.previewMarkers(panel, message.frame, message.program, AbortSignal.timeout(15_000), message.edit);
            } else {
                const context = new StudioContext(panel, message.frame.token, () => this.panel === panel);
                if (message.type === 'sourceIssues') {
                    const catalog = this.programs(panel);
                    await catalog.pending;
                    context.ensure();
                    const issue = await chooseAnimationIssue(catalog.sourceIssues);
                    if (issue) {
                        context.ensure();
                        await panel.revealSource(message.frame, issue.range);
                    }
                    return;
                }
                if (message.type === 'keyframeDraft') {
                    this.view.post(await authorTimelineKeyframe(context, message, () => this.programs(panel).pending));
                    return;
                }
                if (message.type === 'curve') {
                    const { frame, request, program, target, property, window } = message;
                    const reply = { type: 'curveResult', frame, request } as const;
                    const stale = () => this.panel !== panel || !panel.canUse(frame);
                    try {
                        if (!panel.supportsTimelineCurves || stale()) {
                            this.view.post({ ...reply, stale: true });
                            return;
                        }
                        const { points } = await context.read<{ points: CurvePoint[] }>(
                            'designer_animation_curve', { program, target, property, window });
                        this.view.post(stale() ? { ...reply, stale: true } : { ...reply, points });
                    } catch (error) {
                        this.view.post(stale() ? { ...reply, stale: true } : { ...reply, error: errorMessage(error) });
                    }
                    return;
                }
                const program = (await this.programs(panel).pending).find(entry => entry.id === message.program);
                if (message.type === 'programTargets' || message.type === 'revealProgram') {
                    if (!program) throw new Error('Choose a current animation program.');
                    if (message.type === 'programTargets') await chooseAnimationTargets(context, program);
                    else await revealAnimationProgram(context, program);
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
                    const key = await this.recordKeyframe(context, program, timeline, message.time, message.track, message.candidates);
                    if (key) await panel.waitForSnapshot(AbortSignal.timeout(15_000));
                    this.view.post({ type: 'keyframesEdited', keys: key ? [key] : undefined, before: message.frame });
                    return;
                }
                if (message.type === 'keyframeGroup') {
                    const rust = panel.language === 'rust';
                    if (!message.keys.length || (rust && !program.sourceRange)) throw new Error('Select mapped keyframes to edit.');
                    const pose = panel.authoredTimelinePose;
                    const pivot = panel.supportsTimelineKeyframeScale && !program.preview?.native
                        ? pose?.program === program.id ? pose.time : 0 : undefined;
                    const edit = message.edit ?? await chooseKeyframeGroupEdit(message.keys.length, rust, pivot);
                    if (!edit) {
                        this.view.post({ type: 'keyframesEdited', before: message.frame });
                        return;
                    }
                    if (rust && edit.action !== 'move_keyframes' && edit.action !== 'scale_keyframes') throw new Error('Add or remove Rust keys in source.');
                    const version = panel.document.version;
                    await context.edit('designer_program', { ...edit, program: program.id, keyframes: message.keys });
                    if (panel.document.version === version) {
                        this.view.post({ type: 'keyframesEdited', before: message.frame });
                        return;
                    }
                    await panel.waitForSnapshot(AbortSignal.timeout(15_000));
                    const keys = edit.action === 'remove_keyframes' ? [] : message.keys.map(key => ({ ...key,
                        time: edit.action === 'scale_keyframes' ? edit.pivot + (key.time - edit.pivot) * edit.factor : key.time + edit.offset }));
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
        } catch (error) {
            if (message.type === 'keyframeGroup') this.view.post({ type: 'keyframesEdited', before: message.frame });
            this.report(error);
        }
    }

    private async recordKeyframe(context: StudioContext, program: TimelineProgram, timeline: NonNullable<TimelineProgram['timeline']>,
        time: number, index?: number, candidates?: number[]): Promise<KeyframeLocation | undefined> {
        if (!context.writable) throw new Error('Recording into an authored program requires XAML.');
        if (!Number.isFinite(time) || time < 0 || time > timeline.duration) throw new Error('Choose a time within the animation duration.');
        const tracks = candidates === undefined ? timeline.tracks : candidates.map(candidate => timeline.tracks[candidate]).filter(Boolean);
        if (candidates && !tracks.length) throw new Error('Choose an unlocked animation track.');
        let track = index === undefined ? tracks[0] : timeline.tracks[index];
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
