import type { HostedAnimationFrame, HostedAnimationProgram, HostedAnimationTarget, HostedKeyframeSelection } from '../src/hostedAnimationProtocol';
import type { FromHosted, ToHosted } from '../src/hostedProtocol';
import type { TimelineTrack } from '../src/timelineProtocol';
import { create, element } from './dom';
import { editorFrame, type HostedKeyframeEditor } from './hostedKeyframes';
import { TimelineCurvePlot } from './timelineCurves';
import { dragKeyframe, renderKeyframe } from './timelineKeyDrag';
import { clamp, fitKeyTimes, fitTimeline, panTimeline, timelinePercent, visibleTime, zoomTimeline } from './timelineViewport';

interface TrackChoice {
    track: TimelineTrack;
    target: HostedAnimationTarget;
    editable: boolean;
}

export class HostedCurves {
    private readonly toggle = element<HTMLButtonElement>('hosted-animation-curves');
    private readonly panel = element('hosted-curves');
    private readonly select = element<HTMLSelectElement>('hosted-curve-track');
    private readonly lane = element('hosted-curve-lane');
    private readonly label = element('hosted-curve-scale');
    private readonly ruler = element('hosted-curve-axis');
    private readonly fitKey = element<HTMLButtonElement>('hosted-curve-fit-key');
    private readonly number = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
    private view = fitTimeline(0);
    private active = false;
    private program?: HostedAnimationProgram;
    private frame?: HostedAnimationFrame;
    private revision?: number;
    private version = 0;
    private catalog = '';
    private choices: TrackChoice[] = [];
    private plot?: TimelineCurvePlot;
    private selected?: number;
    private pending?: { request: number; key: string };
    private request = 0;

    constructor(private readonly send: (message: FromHosted) => void, private readonly editor: HostedKeyframeEditor) {
        this.toggle.onclick = () => { this.active = !this.active; this.paint(); };
        this.select.onchange = () => { this.selected = undefined; this.editor.close(); this.buildPlot(); };
        element('hosted-curve-out').onclick = () => this.zoom(0.8);
        element('hosted-curve-in').onclick = () => this.zoom(1.25);
        element('hosted-curve-fit').onclick = () => this.setView(fitTimeline(this.view.duration));
        this.fitKey.onclick = () => {
            if (this.selected !== undefined) this.setView(fitKeyTimes(this.view.duration, [this.selected]));
        };
        this.lane.addEventListener('wheel', event => {
            if (event.ctrlKey || event.metaKey) {
                const { left, width } = this.lane.getBoundingClientRect();
                const time = this.view.start + this.view.span * clamp((event.clientX - left) / width, 0, 1);
                this.zoom(event.deltaY > 0 ? 0.8 : 1.25, time);
            } else if (event.shiftKey || event.deltaX) {
                const fraction = (event.deltaX || event.deltaY) / Math.max(1, this.lane.clientWidth);
                this.setView(panTimeline(this.view, this.view.start + fraction * this.view.span));
            } else return;
            event.preventDefault();
        }, { passive: false });
    }

    follow(program: HostedAnimationProgram | undefined, targets: HostedAnimationTarget[], frame: HostedAnimationFrame | undefined,
        version: number, usable: boolean) {
        const sameProgram = program?.id === this.program?.id && program?.duration === this.program?.duration;
        const sameRevision = frame?.revision === this.revision;
        const last = this.choices[this.select.selectedIndex];
        this.program = program;
        this.frame = usable ? frame : undefined;
        this.revision = frame?.revision ?? this.revision;
        this.version = version;
        const catalog = JSON.stringify([program?.id, program?.duration, program?.curveTracks, program?.authoring?.codec, targets]);
        if (catalog !== this.catalog) {
            this.catalog = catalog;
            this.choices = program ? choicesFor(program, targets) : [];
            this.select.replaceChildren(...this.choices.map(choice => new Option(
                targets.length === 1 ? choice.track.property : `${choice.track.property} · ${choice.target.label}`,
            )));
            const selected = retainedCurveTrack(this.choices, last, sameRevision);
            this.select.selectedIndex = selected < 0 ? 0 : selected;
            if (!sameProgram) this.view = fitTimeline(program?.duration ?? 0);
            if (!sameProgram || selected < 0) this.selected = undefined;
            if (this.active) { this.buildPlot(); return; }
            this.plot = undefined;
        }
        this.paint();
    }

    receive(reply: Extract<ToHosted, { type: 'curveReply' }>) {
        if (this.pending?.request !== reply.request) return;
        const { key } = this.pending;
        this.pending = undefined;
        if (key === this.key() && this.frame?.sequence === reply.frame.sequence) {
            this.plot?.receive(key, reply);
        }
        this.paint();
    }

    private zoom(factor: number, time = this.view.start + this.view.span / 2) {
        this.setView(zoomTimeline(this.view, factor, time));
    }

    private setView(view: typeof this.view) {
        if (view.start === this.view.start && view.span === this.view.span) return;
        this.view = view;
        this.ruler.replaceChildren();
        this.paint();
    }

    private buildPlot() {
        this.lane.replaceChildren();
        this.label.replaceChildren();
        this.ruler.replaceChildren();
        const choice = this.choices[this.select.selectedIndex];
        const program = this.program;
        const track = choice?.track;
        const plot = this.plot = program && choice && track ? new TimelineCurvePlot(this.lane, this.label, track) : undefined;
        if (plot && program && choice && track) {
            this.lane.setAttribute('aria-label', `${choice.target.label}, ${track.property} value curve`);
            for (const key of track.keyframes) {
                const value = Number(key.value_text);
                if (!Number.isFinite(value)) continue;
                const location = { time: key.time, target: track.target, property: track.property };
                const rendered = renderKeyframe(location, `${track.property}, ${this.number.format(key.time)} s · ${key.value_text}`);
                const { button } = rendered;
                const open = (found: HostedAnimationFrame): HostedKeyframeSelection => ({
                    frame: found, version: this.version, program: program.id, target: track.target,
                    targetName: choice.target.label, property: track.property, duration: program.duration, keyframe: key,
                });
                if (choice.editable) {
                    button.onclick = () => {
                        if (!this.frame) return;
                        this.selected = key.time;
                        this.editor.open(open(this.frame));
                        this.paint();
                    };
                    dragKeyframe(rendered, this.lane, () => this.view, () => [rendered],
                        () => this.frame && this.plot === plot && this.active ? editorFrame(this.frame) : undefined,
                        time => Math.round(time * 1000) / 1000, () => {}, {
                            value, scale: () => plot.has(this.key() ?? '') ? plot.scale : undefined,
                            commit: (time, value) => {
                                if (!this.frame) return;
                                this.selected = key.time;
                                this.editor.drag(open(this.frame), time, value);
                                this.paint();
                            },
                        });
                }
                plot.keys.push({ rendered, value });
                this.lane.append(button);
            }
        }
        this.paint();
    }

    private key(): string | undefined {
        if (this.frame && this.plot) return JSON.stringify([this.frame.revision, this.catalog, this.select.selectedIndex, this.view.start, this.view.span]);
    }

    private paint() {
        const available = Boolean(this.choices.length);
        this.toggle.hidden = !available;
        this.toggle.setAttribute('aria-pressed', String(available && this.active));
        this.panel.hidden = !available || !this.active;
        if (this.panel.hidden) return;
        if (!this.plot) { this.buildPlot(); return; }

        const choice = this.choices[this.select.selectedIndex];
        if (!this.ruler.childElementCount) {
            for (const mark of [0, 0.25, 0.5, 0.75, 1]) {
                const tick = create('span');
                tick.textContent = `${this.number.format(this.view.start + this.view.span * mark)} s`;
                this.ruler.append(tick);
            }
        }
        const key = this.key();
        this.plot.draw(true, this.view, key);
        this.label.toggleAttribute('data-active', true);
        this.fitKey.disabled = !this.plot.keys.some(({ rendered }) => rendered.key.time === this.selected);
        for (const { rendered } of this.plot.keys) {
            rendered.button.hidden = !visibleTime(this.view, rendered.key.time);
            rendered.button.disabled = !choice.editable || !this.frame;
            rendered.button.setAttribute('aria-pressed', String(rendered.key.time === this.selected));
            if (!rendered.button.classList.contains('timeline-key-dragging')) {
                rendered.button.style.left = `${timelinePercent(this.view, rendered.key.time)}%`;
            }
        }
        const frame = this.frame;
        if (key && frame && this.program && !this.pending && !this.plot.has(key) && this.plot.failed?.key !== key) {
            const request = ++this.request;
            this.pending = { request, key };
            this.send({ type: 'curve', request, frame, query: { program: this.program.id, target: choice.target.id,
                property: choice.track.property, window: { start: this.view.start, end: Math.min(this.view.duration, this.view.start + this.view.span) } } });
        }
    }
}

export function retainedCurveTrack(choices: readonly TrackChoice[], previous: TrackChoice | undefined, sameRevision: boolean): number {
    if (!previous) return -1;
    const matches = choices.filter(choice => choice.track.property === previous.track.property);
    if (matches.length === 1) return choices.indexOf(matches[0]);
    if (sameRevision) {
        const exact = matches.find(choice => choice.target.id === previous.target.id);
        if (exact) return choices.indexOf(exact);
    }
    // Rebuilding a document can renumber its source nodes. Restore the UI choice
    // only when one current control has the former target's label and type.
    const named = matches.filter(({ target }) => target.label === previous.target.label && target.typeName === previous.target.typeName);
    return named.length === 1 ? choices.indexOf(named[0]) : -1;
}

function choicesFor(program: HostedAnimationProgram, targets: HostedAnimationTarget[]): TrackChoice[] {
    const tracks = program.curveTracks ?? [];
    return targets.flatMap(target => {
        const choices = new Map<string, TimelineTrack>();
        for (const track of tracks) {
            if (!choices.has(track.property) || track.target === target.id) choices.set(track.property, track);
        }
        return [...choices.values()].map(track => ({ track, target,
            editable: Boolean(program.authoring?.tracks.some(authored => authored.target === track.target && authored.property === track.property)),
        }));
    });
}
