import type * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { errorMessage, type FrameTarget } from './protocol';
import { choose, StudioContext } from './studioContext';
import { keyframeOptions } from './studioKeyframes';
import { TimelinePlayback } from './timelinePlayback';
import type { FromTimeline, TimelineControl, TimelineProgram, TimelineState, ToTimeline } from './timelineProtocol';
import { timelineHtml } from './view';

export const TIMELINE_VIEW = 'ferroui.timeline';

interface Catalog {
    key: string;
    token: number | undefined;
    pending: Promise<TimelineProgram[]>;
    programs?: TimelineProgram[];
    error?: string;
}

export class TimelineView implements vscode.Disposable {
    readonly view: DockView<TimelineState, FromTimeline, ToTimeline>;
    private readonly playback: TimelinePlayback;
    private panel?: DesignerPanel;
    private file = '';
    private catalog?: Catalog;
    private shown?: string;

    constructor(root: vscode.Uri, changed: () => void) {
        this.view = new DockView(root, timelineHtml, changed, message => { void this.receive(message); });
        this.playback = new TimelinePlayback(position => {
            this.view.post({ type: 'position', position, frame: this.frame() });
            if (!position?.busy) this.refresh();
        }, error => this.report(error));
    }

    update(panel: DesignerPanel | undefined, file: string): void {
        this.panel = panel;
        this.file = file;
        this.playback.follow(panel);
        this.refresh();
    }

    async control(panel: DesignerPanel, frame: FrameTarget, control: TimelineControl, signal: AbortSignal) {
        if (control.action === 'pause' && this.playback.position?.session === panel.session) {
            return this.playback.control(panel, frame, control, 0, signal);
        }
        if (!panel.canUse(frame)) throw new Error('Read the current designer revision.');
        const program = (await this.programs(panel).pending).find(entry => entry.id === control.program);
        if (!program?.timeline) throw new Error('Choose an authored animation program from the current document.');
        return this.playback.control(panel, frame, control, program.timeline.duration, signal);
    }

    private programs(panel: DesignerPanel): Catalog {
        const block = panel.snapshot?.activeBlock?.start;
        const key = `${panel.session}:${panel.document.version}:${block?.line ?? 0}:${block?.character ?? 0}`;
        if (this.catalog?.key === key && (!this.catalog.error || this.catalog.token === panel.currentToken)) return this.catalog;
        const pending = panel.studioQuery('designer_programs', { include_source: false, include_keyframe_text: true },
            undefined, AbortSignal.timeout(15_000)).then(result => (result as { programs: TimelineProgram[] }).programs);
        const catalog: Catalog = this.catalog = { key, token: panel.currentToken, pending };
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
        const position = this.playback.position;
        if (position?.busy && frame?.session === position.session) return;
        const key = `${panel?.session}:${frame?.token}:${panel?.status ?? ''}`;
        if (key === this.shown) return;
        this.shown = key;
        const catalog = panel && frame && !panel.status ? this.programs(panel) : undefined;
        const programs = catalog?.programs ?? [];
        const message = !panel ? 'Open a FerroUI designer.' : panel.status ?? catalog?.error
            ?? (catalog?.programs ? (programs.length ? undefined : 'This design has no authored animation programs.') : 'Loading animation programs…');
        const names = Object.fromEntries((panel?.nodes ?? []).map(node => [node.id, node.name ?? node.label]));
        this.view.show({ frame: panel?.status ? undefined : frame, programs, message, names, writable: panel?.language === 'xaml', position }, this.file);
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
                const timeline = program?.timeline;
                if (!program || !timeline) throw new Error('Choose a typed animation program from the current document.');
                context.ensure();
                if (message.type === 'recordKeyframe') {
                    await this.recordKeyframe(context, program, timeline, message.time);
                    return;
                }
                const track = timeline.tracks[message.track];
                const frame = track?.keyframes.find(entry => entry.time === message.time);
                if (!track || !frame) throw new Error('Select a keyframe from the current timeline.');
                if (message.type === 'moveKeyframe') {
                    if (!Number.isFinite(message.to) || message.to < 0 || message.to > timeline.duration) {
                        throw new Error('Choose a time within the animation duration.');
                    }
                    await context.edit('designer_program', { action: 'upsert_keyframe', program: program.id, target: track.target,
                        property: track.property, previous: frame.time, time: message.to, value: frame.value_text, easing: frame.easing_text });
                } else await keyframeOptions(context, program.id, timeline.duration, track, frame, async () => {
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
