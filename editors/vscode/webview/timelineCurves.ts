import type { FrameTarget } from '../src/protocol';
import type { CurveChannel, CurveMetric, CurvePoint, TimelineCurveRequest, TimelineCurveResponse, TimelineTrack } from '../src/timelineProtocol';
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
const colorChannels = new Set<CurveChannel>(['red', 'green', 'blue', 'alpha']);
const channelLabels: Partial<Record<CurveChannel, string>> = {
    percent_x: 'X (%)', percent_y: 'Y (%)', absolute_x: 'X (px)', absolute_y: 'Y (px)',
    percent: 'Percent', absolute: 'Pixels',
    blur_radius: 'Blur radius (px)', offset_x: 'Offset X (px)', offset_y: 'Offset Y (px)',
    direction: 'Direction (°)', shadow_depth: 'Shadow depth (px)', shadow_opacity: 'Shadow opacity',
};

export function curveChannels(track: TimelineTrack): readonly CurveChannel[] {
    return track.curve_channels ?? (track.keyframes[0]?.value_kind === 'Number' ? ['value'] : []);
}

/** Uses the Timeline's existing focusable key buttons as the curve's handles. */
export class TimelineCurvePlot {
    private readonly svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    private readonly line = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    private readonly zeroLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    private readonly scaleLabels = create('span');
    private readonly status = create('span');
    private readonly ticks = [create('span'), create('span')];
    private readonly unit = create('span');
    private readonly channelPicker?: HTMLSelectElement;
    readonly keys: { rendered: RenderedKeyframe; value?: number }[] = [];
    private accepted?: { key: string; metric: CurveMetric; channel: CurveChannel; points: CurvePoint[] };
    private painted?: { metric?: CurveMetric; channel: CurveChannel; view: TimelineViewport; points?: CurvePoint[]; error?: string };
    scale?: CurveScale;
    pending?: { request: TimelineCurveRequest; key: string };
    failed?: { key: string; error: string };
    visible = false;

    constructor(readonly lane: HTMLElement, label: HTMLElement, readonly track: TimelineTrack,
        preferred: CurveChannel | undefined, changed: (channel: CurveChannel) => void) {
        this.svg.classList.add('timeline-curve-plot');
        this.svg.setAttribute('aria-hidden', 'true');
        this.svg.setAttribute('viewBox', '0 0 100 100');
        this.svg.setAttribute('preserveAspectRatio', 'none');
        this.line.setAttribute('vector-effect', 'non-scaling-stroke');
        this.line.setAttribute('stroke-linecap', 'round');
        this.zeroLine.setAttribute('x2', '100');
        this.zeroLine.setAttribute('vector-effect', 'non-scaling-stroke');
        this.zeroLine.classList.add('timeline-curve-zero');
        this.svg.append(this.zeroLine, this.line);
        this.scaleLabels.className = 'timeline-curve-scale';
        this.scaleLabels.setAttribute('aria-hidden', 'true');
        this.scaleLabels.append(...this.ticks, this.unit);
        this.status.className = 'timeline-curve-status';
        lane.prepend(this.svg, this.status);
        label.append(this.scaleLabels);
        const choices = curveChannels(track);
        if (choices.length > 1) {
            const select = this.channelPicker = create('select');
            const name = `${track.property} curve channel`;
            select.setAttribute('aria-label', name);
            select.title = name;
            select.className = 'timeline-curve-channel';
            select.hidden = true;
            select.append(...choices.map(channel => new Option(channelLabels[channel] ??
                channel[0].toUpperCase() + channel.slice(1).replaceAll('_', ' '), channel)));
            if (preferred && choices.includes(preferred)) select.value = preferred;
            select.onchange = () => changed(this.channel);
            label.append(select);
        }
    }

    get channel(): CurveChannel { return (this.channelPicker?.value as CurveChannel | undefined) ?? curveChannels(this.track)[0] ?? 'value'; }
    has(key: string): boolean { return this.accepted?.key === key; }

    receive(key: string, metric: CurveMetric, channel: CurveChannel, message: Pick<TimelineCurveResponse, 'points' | 'error' | 'stale'>): void {
        if (message.points?.length && message.points.every(point => Number.isFinite(point.time) && Number.isFinite(point.value))) {
            this.accepted = { key, metric, channel, points: message.points };
            this.failed = undefined;
        } else if (!message.stale) this.failed = { key, error: message.error ?? 'Curve samples are unavailable.' };
    }

    draw(metric: CurveMetric | undefined, view: TimelineViewport, key: string | undefined): void {
        const channel = this.channel;
        const points = this.accepted?.metric === metric && this.accepted?.channel === channel ? this.accepted?.points : undefined;
        const error = this.failed?.key === key ? this.failed?.error : undefined;
        if (this.painted && this.painted.metric === metric && this.painted.channel === channel && this.painted.view === view
            && this.painted.points === points && this.painted.error === error) return;
        this.painted = { metric, channel, view, points, error };
        this.status.textContent = metric ? error ?? '' : '';
        if (this.channelPicker) this.channelPicker.hidden = !metric;
        if (!metric) {
            delete this.lane.dataset.curve;
            this.scale = undefined;
            for (const { rendered } of this.keys) rendered.button.style.top = '';
            return;
        }
        const editsValues = channel === 'value' && metric === 'value';
        this.lane.dataset.curve = metric === 'value' && !editsValues ? 'component' : metric;
        this.line.dataset.channel = channel;
        const visible = points?.filter(point => visibleTime(view, point.time)) ?? [];
        // Include neighbors so panning can move immediately while the current range samples arrive.
        const first = visible[0];
        const last = visible.at(-1);
        const before = points?.findLast(point => point.time < (first?.time ?? view.start));
        const after = points?.find(point => point.time > (last?.time ?? view.start + view.span));
        const isVelocity = metric === 'velocity';
        const keyValues = isVelocity ? [0] : colorChannels.has(channel) ? [0, 1]
            : this.keys.flatMap(({ rendered, value }) =>
                value !== undefined && visibleTime(view, rendered.key.time) ? [value] : []);
        const allValues = this.keys.flatMap(({ value }) => value === undefined ? [] : [value]);
        const scale = this.scale = new CurveScale(visible.length || keyValues.length ? visible : points ?? [],
            keyValues.length || visible.length ? keyValues : allValues.length ? allValues : [0]);
        const trace = [...(before ? [before] : []), ...visible, ...(after ? [after] : [])];
        this.line.setAttribute('d', trace.map((point, index) => {
            const starts = index === 0 || point.break_before;
            const ends = index === trace.length - 1 || trace[index + 1].break_before;
            const position = `${timelinePercent(view, point.time).toFixed(3)},${scale.percent(point.value).toFixed(3)}`;
            return `${starts ? 'M' : 'L'}${position}${starts && ends ? `L${position}` : ''}`;
        }).join(' '));
        const [low, high] = scale.bounds;
        this.scaleLabels.toggleAttribute('data-waiting', !editsValues && !points?.length);
        this.ticks[0].textContent = axisNumber.format(low);
        this.ticks[1].textContent = axisNumber.format(high);
        this.unit.textContent = channel.startsWith('percent') ? (isVelocity ? 'pp/s' : '%')
            : channel === 'direction' ? (isVelocity ? '°/s' : '°')
            : channel.startsWith('absolute') || channel === 'blur_radius' || channel.startsWith('offset_') || channel === 'shadow_depth'
                ? (isVelocity ? 'px/s' : 'px') : isVelocity ? '/s' : '';
        this.zeroLine.toggleAttribute('data-visible', isVelocity && Boolean(points?.length) && low < 0 && high > 0);
        const baseline = scale.percent(0).toFixed(3);
        this.zeroLine.setAttribute('y1', baseline);
        this.zeroLine.setAttribute('y2', baseline);
        for (const { rendered, value } of this.keys) rendered.button.style.top = editsValues && value !== undefined ? `${scale.percent(value)}%` : '';
    }
}

/** Retain accepted traces across playback ticks and sample only on-screen tracks. */
export class TimelineCurves {
    private readonly plots = new Map<Element, TimelineCurvePlot>();
    private readonly channels = new Map<string, CurveChannel>();
    private frame?: CurveSnapshot;
    private range?: TimelineViewport;
    private metric?: CurveMetric;
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
        const identity = `${track.target}:${track.property}`;
        const plot = new TimelineCurvePlot(lane, label, track, this.channels.get(identity), channel => {
            this.channels.set(identity, channel);
            if (this.range) plot.draw(this.metric, this.range, this.key(plot));
            this.schedule();
        });
        this.plots.set(lane, plot);
        this.observer.observe(lane);
        return plot;
    }

    clear(): void {
        this.observer.disconnect();
        this.plots.clear();
    }

    follow(metric: CurveMetric | undefined, range: TimelineViewport, frame?: CurveSnapshot): void {
        this.metric = metric;
        this.range = range;
        this.frame = frame;
        for (const plot of this.plots.values()) plot.draw(metric, range, this.key(plot));
        this.schedule();
    }

    canEdit(plot: TimelineCurvePlot): boolean {
        const key = this.key(plot);
        return this.metric === 'value' && plot.channel === 'value' && key !== undefined && plot.has(key);
    }

    receive(message: TimelineCurveResponse): void {
        const plot = [...this.plots.values()].find(item => item.pending?.request.request === message.request);
        const pending = plot?.pending;
        if (!plot || !pending || pending.request.frame.token !== message.frame.token
            || pending.request.frame.session !== message.frame.session) return;
        plot.pending = undefined;
        const current = this.frame?.frame;
        if (current?.token === message.frame.token && current.session === message.frame.session && pending.key === this.key(plot)) {
            plot.receive(pending.key, pending.request.metric ?? 'value', pending.request.channel ?? 'value', message);
            if (this.range) plot.draw(this.metric, this.range, pending.key);
        }
        this.schedule();
    }

    private key(plot: TimelineCurvePlot): string | undefined {
        if (this.metric && this.frame && this.range?.span) {
            const { start, span } = this.range;
            return JSON.stringify([this.frame.catalog, this.frame.program, this.metric, plot.channel, start, span]);
        }
    }

    private schedule(): void {
        window.clearTimeout(this.timer);
        if (this.metric && this.frame && this.range?.span) this.timer = window.setTimeout(() => this.sample(), 60);
    }

    private sample(): void {
        const snapshot = this.frame;
        const range = this.range;
        if (!snapshot || !range) return;
        for (const plot of this.plots.values()) {
            const key = this.key(plot);
            if (!key || !plot.visible || plot.has(key) || plot.failed?.key === key || plot.pending?.key === key) continue;
            const { target, property } = plot.track;
            const request: TimelineCurveRequest = { type: 'curve', request: ++this.request, frame: snapshot.frame,
                program: snapshot.program, target, property, metric: this.metric, channel: plot.channel,
                window: { start: range.start, end: Math.min(range.duration, range.start + range.span) } };
            plot.pending = { request, key };
            this.send(request);
        }
    }
}
