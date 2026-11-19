import type { FrameTarget } from '../src/protocol';
import type { CurvePoint, TimelineCurveRequest, TimelineCurveResponse, TimelineTrack } from '../src/timelineProtocol';
import { create } from './dom';
import { CurveScale } from './timelineCurveScale';
import type { RenderedKeyframe } from './timelineKeyDrag';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

interface CurveSnapshot {
    frame: FrameTarget;
    catalog: string;
    program: number;
}

const axisNumber = new Intl.NumberFormat(undefined, { maximumSignificantDigits: 3, notation: 'compact' });

/** Uses the Timeline's existing focusable key buttons as the curve's handles. */
export class TimelineCurvePlot {
    private readonly svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    private readonly line = document.createElementNS('http://www.w3.org/2000/svg', 'polyline');
    private readonly scaleLabels = create('span');
    private readonly status = create('span');
    private readonly ticks = [create('span'), create('span')];
    readonly keys: { rendered: RenderedKeyframe; value: number }[] = [];
    private accepted?: { key: string; points: CurvePoint[] };
    private painted?: { enabled: boolean; view: TimelineViewport; points?: CurvePoint[]; error?: string };
    scale?: CurveScale;
    pending?: { request: TimelineCurveRequest; key: string };
    failed?: { key: string; error: string };
    visible = false;

    constructor(readonly lane: HTMLElement, label: HTMLElement, readonly track: TimelineTrack) {
        this.svg.classList.add('timeline-curve-plot');
        this.svg.setAttribute('aria-hidden', 'true');
        this.svg.setAttribute('viewBox', '0 0 100 100');
        this.svg.setAttribute('preserveAspectRatio', 'none');
        this.line.setAttribute('vector-effect', 'non-scaling-stroke');
        this.svg.append(this.line);
        this.scaleLabels.className = 'timeline-curve-scale';
        this.scaleLabels.setAttribute('aria-hidden', 'true');
        this.scaleLabels.append(...this.ticks);
        this.status.className = 'timeline-curve-status';
        lane.prepend(this.svg, this.status);
        label.append(this.scaleLabels);
    }

    has(key: string): boolean { return this.accepted?.key === key; }

    receive(key: string, message: TimelineCurveResponse): void {
        if (message.points?.length && message.points.every(point => Number.isFinite(point.time) && Number.isFinite(point.value))) {
            this.accepted = { key, points: message.points };
            this.failed = undefined;
        } else if (!message.stale) this.failed = { key, error: message.error ?? 'Curve samples are unavailable.' };
    }

    draw(enabled: boolean, view: TimelineViewport, key: string | undefined): void {
        const points = this.accepted?.points;
        const error = this.failed?.key === key ? this.failed?.error : undefined;
        if (this.painted?.enabled === enabled && this.painted.view === view
            && this.painted.points === points && this.painted.error === error) return;
        this.painted = { enabled, view, points, error };
        this.lane.toggleAttribute('data-curve', enabled);
        this.status.textContent = enabled ? error ?? '' : '';
        if (!enabled) {
            this.scale = undefined;
            for (const { rendered } of this.keys) rendered.button.style.top = '';
            return;
        }
        const visible = points?.filter(point => visibleTime(view, point.time)) ?? [];
        // Include neighbors so panning can move immediately while the current range samples arrive.
        const first = visible[0];
        const last = visible.at(-1);
        const before = points?.findLast(point => point.time < (first?.time ?? view.start));
        const after = points?.find(point => point.time > (last?.time ?? view.start + view.span));
        const keyValues = this.keys.flatMap(({ rendered, value }) => visibleTime(view, rendered.key.time) ? [value] : []);
        const scale = this.scale = new CurveScale(visible.length || keyValues.length ? visible : points ?? [],
            keyValues.length || visible.length ? keyValues : this.keys.map(({ value }) => value));
        const trace = [...(before ? [before] : []), ...visible, ...(after ? [after] : [])];
        this.line.setAttribute('points', trace.map(({ time, value }) =>
            `${timelinePercent(view, time).toFixed(3)},${scale.percent(value).toFixed(3)}`).join(' '));
        const [low, high] = scale.bounds;
        this.ticks[0].textContent = axisNumber.format(low);
        this.ticks[1].textContent = axisNumber.format(high);
        for (const { rendered, value } of this.keys) rendered.button.style.top = `${scale.percent(value)}%`;
    }
}

/** Retain accepted traces across playback ticks and sample only on-screen tracks. */
export class TimelineCurves {
    private readonly plots = new Map<Element, TimelineCurvePlot>();
    private frame?: CurveSnapshot;
    private range?: TimelineViewport;
    private enabled = false;
    private timer = 0;
    private request = 0;
    private readonly observer: IntersectionObserver;

    constructor(grid: HTMLElement, private readonly send: (message: TimelineCurveRequest) => void) {
        this.observer = new IntersectionObserver(entries => {
            for (const entry of entries) {
                const plot = this.plots.get(entry.target);
                if (plot) plot.visible = entry.isIntersecting;
            }
            this.schedule();
        }, { root: grid, rootMargin: '30px 0px' });
    }

    add(lane: HTMLElement, label: HTMLElement, track: TimelineTrack): TimelineCurvePlot {
        const plot = new TimelineCurvePlot(lane, label, track);
        this.plots.set(lane, plot);
        this.observer.observe(lane);
        return plot;
    }

    clear(): void {
        this.observer.disconnect();
        this.plots.clear();
    }

    follow(enabled: boolean, range: TimelineViewport, frame?: CurveSnapshot): void {
        this.enabled = enabled;
        this.range = range;
        this.frame = frame;
        const key = this.key();
        for (const plot of this.plots.values()) plot.draw(enabled, range, key);
        this.schedule();
    }

    canEdit(plot: TimelineCurvePlot): boolean {
        const key = this.key();
        return this.enabled && key !== undefined && plot.has(key);
    }

    receive(message: TimelineCurveResponse): void {
        const plot = [...this.plots.values()].find(item => item.pending?.request.request === message.request);
        const pending = plot?.pending;
        if (!plot || !pending || pending.request.frame.token !== message.frame.token
            || pending.request.frame.session !== message.frame.session) return;
        plot.pending = undefined;
        const current = this.frame?.frame;
        if (current?.token === message.frame.token && current.session === message.frame.session && pending.key === this.key()) {
            plot.receive(pending.key, message);
            if (this.range) plot.draw(this.enabled, this.range, pending.key);
        }
        this.schedule();
    }

    private key(): string | undefined {
        if (this.frame && this.range?.span) {
            const { start, span } = this.range;
            return JSON.stringify([this.frame.catalog, this.frame.program, start, span]);
        }
    }

    private schedule(): void {
        window.clearTimeout(this.timer);
        if (this.enabled && this.frame && this.range?.span) this.timer = window.setTimeout(() => this.sample(), 60);
    }

    private sample(): void {
        const snapshot = this.frame;
        const range = this.range;
        const key = this.key();
        if (!snapshot || !range || !key) return;
        for (const plot of this.plots.values()) {
            if (!plot.visible || plot.has(key) || plot.failed?.key === key || plot.pending?.key === key) continue;
            const { target, property } = plot.track;
            const request: TimelineCurveRequest = { type: 'curve', request: ++this.request, frame: snapshot.frame,
                program: snapshot.program, target, property, window: { start: range.start, end: Math.min(range.duration, range.start + range.span) } };
            plot.pending = { request, key };
            this.send(request);
        }
    }
}
