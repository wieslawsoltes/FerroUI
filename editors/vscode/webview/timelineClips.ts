import type { FrameTarget } from '../src/protocol';
import type { TimelineProgram, TimelineState } from '../src/timelineProtocol';
import { canPreviewProgram } from '../src/timelinePrograms';
import { create, element, labelButton } from './dom';
import { bindClipDrag } from './timelineClipDrag';
import { TimelineClipPlacement } from './timelineClipPlacement';
import { clamp, fitTimeline, minimumTimelineSpan, panTimeline, timelinePercent, visibleTime, zoomTimeline } from './timelineViewport';

type Clip = TimelineProgram & { clip: NonNullable<TimelineProgram['clip']> };
interface Row { key: string; program: Clip; label: HTMLButtonElement; order: HTMLElement; bar: HTMLButtonElement }

interface ClipActions {
    changed(): void;
    seek(time: number): void;
    transport(action: 'play_all' | 'pause' | 'stop', program: number): void;
    place(frame: FrameTarget, programs: number[], start: number | undefined, gap: number): void;
    keys(program: Clip): void;
}

const number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });

export class TimelineClips {
    private readonly toggle = element<HTMLButtonElement>('timeline-compare');
    private readonly pane = element('timeline-clips');
    private readonly tools = element('timeline-clip-tools');
    private readonly rows = element('clip-rows');
    private readonly seek = element<HTMLInputElement>('clip-seek');
    private readonly time = element<HTMLInputElement>('clip-time');
    private readonly play = element<HTMLButtonElement>('clip-play');
    private readonly stop = element<HTMLButtonElement>('clip-stop');
    private readonly keys = element<HTMLButtonElement>('clip-keys');
    private readonly pan = element<HTMLInputElement>('clip-pan');
    private readonly zoomIn = element<HTMLButtonElement>('clip-zoom-in');
    private readonly zoomOut = element<HTMLButtonElement>('clip-zoom-out');
    private readonly fit = element<HTMLButtonElement>('clip-fit');
    private view = fitTimeline(0);
    private scope?: string;
    private programRows: Row[] = [];
    private selection: string[] = [];
    private signature = '';
    private state?: TimelineState;
    private seconds = 0;
    private locked = true;
    private drafting = false;
    private readonly placement: TimelineClipPlacement;
    visible = false;

    constructor(public enabled: boolean, private readonly actions: ClipActions) {
        this.placement = new TimelineClipPlacement((...args) => actions.place(...args));
        this.toggle.onclick = () => { this.enabled = !this.enabled; this.actions.changed(); };
        this.seek.oninput = () => this.actions.seek(this.seek.valueAsNumber);
        this.time.oninput = () => { this.drafting = true; };
        this.time.onblur = () => { this.drafting = false; this.showTime(); };
        element<HTMLFormElement>('clip-clock').onsubmit = event => {
            event.preventDefault();
            const value = this.time.valueAsNumber;
            this.drafting = false;
            if (Number.isFinite(value)) this.actions.seek(clamp(value, 0, this.view.duration));
        };
        this.play.onclick = () => {
            const position = this.state?.position;
            const program = position?.playing ? position.program : this.reference;
            if (program !== undefined) this.actions.transport(position?.playing ? 'pause' : 'play_all', program);
        };
        this.stop.onclick = () => {
            if (this.state?.position) this.actions.transport('stop', this.state.position.program);
        };
        this.keys.onclick = () => {
            const program = this.selected[0];
            if (program) { this.enabled = false; this.actions.keys(program); }
        };
        this.zoomIn.onclick = () => this.zoom(1.25);
        this.zoomOut.onclick = () => this.zoom(.8);
        this.fit.onclick = () => this.fitClips();
        this.pan.oninput = () => { this.view = panTimeline(this.view, this.pan.valueAsNumber); this.layout(); };
        this.pane.onkeydown = event => {
            if (event.target instanceof HTMLInputElement || event.altKey || event.metaKey || event.ctrlKey) return;
            if (event.key === '+' || event.key === '=') this.zoom(1.25);
            else if (event.key === '-') this.zoom(.8);
            else if (event.key === '0') this.fitClips();
            else return;
            event.preventDefault();
        };
    }

    get reference(): number | undefined {
        const position = this.state?.position;
        if (position?.all && this.state?.programs.some(program => program.id === position.program)) return position.program;
        return this.state?.programs.find(canPreviewProgram)?.id;
    }

    private get frame(): FrameTarget | undefined { return !this.locked && this.visible ? this.state?.frame : undefined; }

    private get selected(): Clip[] {
        const programs = new Map(this.programRows.map(row => [row.key, row.program]));
        return this.selection.flatMap(key => programs.get(key) ?? []);
    }

    follow(state: TimelineState, pending: boolean, editing: boolean, seconds: number): void {
        this.state = state;
        const playing = Boolean(state.position?.playing);
        const locked = pending || editing || playing;
        this.locked = locked;
        this.seconds = seconds;
        const clips = state.programs.filter((program): program is Clip => Boolean(program.clip));
        if (state.clipScope && this.scope !== state.clipScope) {
            this.scope = state.clipScope;
            this.selection = [];
        }
        const available = Boolean(clips.length && !state.transition && (state.clipScope || (!state.frame && this.scope)));
        this.toggle.hidden = !available;
        this.visible = this.enabled && available;
        this.toggle.disabled = !state.frame || locked;
        this.toggle.setAttribute('aria-pressed', String(this.visible));
        labelButton(this.toggle, this.visible ? 'Return to animation keyframes' : 'Compare animation clips');
        this.pane.hidden = this.tools.hidden = !this.visible;
        element('timeline-toolbar').toggleAttribute('data-clips', this.visible);
        if (!this.visible) { this.placement.follow(undefined, []); return; }
        const signature = JSON.stringify([this.scope, clips.map(({id, name, source_key, clip}) => [id, name, source_key, clip])]);
        if (signature !== this.signature) {
            this.signature = signature;
            this.renderRows(clips);
        }
        const elapsed = state.position?.all ?? false;
        this.play.disabled = !state.frame || this.reference === undefined || (!playing && locked);
        this.stop.disabled = !state.frame || !state.position || (!playing && locked);
        labelButton(this.play, playing ? 'Pause clips' : elapsed && !state.position?.completed ? 'Resume clips' : 'Play all clips',
            playing ? 'debug-pause' : 'run-all');
        this.seek.disabled = this.time.disabled = !state.frame || editing || playing || this.reference === undefined;
        this.showTime();
        this.showSelection();
    }

    private renderRows(programs: readonly Clip[]): void {
        const focused = this.programRows.find(row => row.bar === document.activeElement || row.label === document.activeElement);
        const wasBar = focused?.bar === document.activeElement;
        const counts = new Map<string, number>();
        for (const { source_key } of programs) {
            if (source_key) counts.set(source_key, (counts.get(source_key) ?? 0) + 1);
        }
        this.programRows = programs.map(program => this.row(program,
            program.source_key && counts.get(program.source_key) === 1 ? program.source_key : String(program.id)));
        const visible = new Set(this.programRows.map(row => row.key));
        this.selection = this.selection.filter(key => visible.has(key));
        const duration = programs.reduce((end, program) => Math.max(end, program.clip.end ?? (program.clip.start + (program.preview?.duration ?? program.timeline?.duration ?? 1))), 0.05);
        if (this.view.duration !== duration) this.view = fitTimeline(duration);
        this.rows.replaceChildren(...this.programRows.map(row => row.label.parentElement!));
        this.layout();
        if (focused) {
            const restored = this.programRows.find(row => row.key === focused.key);
            (restored ? wasBar && !restored.bar.hidden ? restored.bar : restored.label : this.toggle).focus({ preventScroll: true });
        }
    }

    private row(program: Clip, key: string): Row {
        const item = create('li');
        item.className = 'clip-row';
        const label = create('button');
        label.type = 'button';
        label.className = 'clip-label';
        const order = create('span');
        order.className = 'clip-order';
        order.setAttribute('aria-hidden', 'true');
        label.append(create('span', program.name), order);
        label.onclick = () => {
            if (this.locked) return;
            this.selection = this.selection.includes(key) ? this.selection.filter(selected => selected !== key) : [...this.selection, key];
            this.showSelection();
        };
        const lane = create('div');
        lane.className = 'clip-lane';
        const bar = create('button');
        bar.type = 'button';
        bar.className = 'clip-bar';
        const timing = program.clip;
        const end = timing.end === null ? '∞' : `${number.format(timing.end)} s`;
        labelButton(bar, `${program.name} · ${number.format(timing.start)}–${end}${timing.editable ? '' : ' · Read-only timing'}`);
        bar.toggleAttribute('data-infinite', timing.end === null);
        bar.toggleAttribute('data-readonly', !timing.editable);
        bar.onclick = () => { if (this.frame) this.actions.seek(timing.start); };
        lane.append(bar);
        item.append(label, lane);
        const row = { key, program, label, order, bar };
        if (timing.editable) bindClipDrag(bar, lane, timing.start, () => this.view, () => this.frame,
            time => this.layoutBar(row, time), (time, frame) => this.actions.place(frame, [program.id], time, 0));
        return row;
    }

    private showSelection(): void {
        const selected = this.selected;
        for (const row of this.programRows) {
            const index = this.selection.indexOf(row.key);
            row.label.setAttribute('aria-pressed', String(index >= 0));
            row.label.setAttribute('aria-label', index >= 0 ? `${row.program.name}, sequence position ${index + 1}` : row.program.name);
            row.order.textContent = index >= 0 ? String(index + 1) : '';
            row.bar.toggleAttribute('data-selected', index >= 0);
            row.bar.toggleAttribute('data-editable', row.program.clip.editable && Boolean(this.frame));
        }
        this.placement.follow(this.frame, selected);
        this.keys.disabled = !this.frame || selected.length !== 1 || !selected[0].timeline;
    }

    private showTime(): void {
        if (!this.drafting || this.time.disabled) this.time.value = String(Number(this.seconds.toFixed(6)));
        this.seek.value = String(this.seconds);
        this.seek.setAttribute('aria-valuetext', `${number.format(this.seconds)} seconds elapsed`);
        const head = visibleTime(this.view, this.seconds) ? `${timelinePercent(this.view, this.seconds)}%` : '-2px';
        this.pane.style.setProperty('--clip-head', head);
    }

    private fitClips(): void { this.view = fitTimeline(this.view.duration); this.layout(); }

    private zoom(factor: number): void { this.view = zoomTimeline(this.view, factor, this.seconds); this.layout(); }

    private layout(): void {
        const { start, span, duration } = this.view;
        this.zoomIn.disabled = span <= minimumTimelineSpan;
        this.zoomOut.disabled = this.fit.disabled = span >= duration;
        this.seek.min = String(start);
        this.seek.max = String(start + span);
        this.pan.max = String(duration - span);
        this.pan.value = String(start);
        element('clip-pan-row').hidden = span >= duration;
        element('clip-duration').textContent = `/ ${number.format(duration)} s`;
        element('clip-marks').replaceChildren(...[0, .25, .5, .75, 1].map(fraction => {
            const mark = create('span', number.format(start + fraction * span));
            mark.style.left = `${fraction * 100}%`;
            return mark;
        }));
        for (const row of this.programRows) this.layoutBar(row, row.program.clip.start);
        this.showTime();
    }

    private layoutBar(row: Row, start: number): void {
        const clip = row.program.clip;
        const end = clip.end === null ? this.view.duration : start + (clip.end - clip.start);
        const left = timelinePercent(this.view, start);
        const right = timelinePercent(this.view, end);
        row.bar.hidden = right < 0 || left > 100;
        row.bar.style.setProperty('--clip-left', `${clamp(left, 0, 100)}%`);
        row.bar.style.setProperty('--clip-width', `${clamp(right, 0, 100) - clamp(left, 0, 100)}%`);
    }
}
