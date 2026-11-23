import type { FrameTarget } from '../src/protocol';
import type { CurveChannel, CurveMetric, CurvePoint, TimelineCurveRequest, TimelineCurveResponse, TimelineKeyframe, TimelineTrack } from '../src/timelineProtocol';
import { create } from './dom';
import { CurveScale } from './timelineCurveScale';
import { curveKeyPosition, snapCurveInteger, type CurveHandle, type RenderedKeyframe } from './timelineKeyDrag';
import { timelinePercent, visibleTime, type TimelineViewport } from './timelineViewport';

interface CurveSnapshot {
    frame: FrameTarget;
    catalog: string;
    program: number;
}

const axisNumber = new Intl.NumberFormat(undefined, { maximumSignificantDigits: 3, notation: 'compact' });
const colorChannels = new Set<CurveChannel>(['red', 'green', 'blue', 'alpha']);
const angleChannels = new Set<CurveChannel>(['direction', 'angle', 'angle_x', 'angle_y']);
const pixelChannels = new Set<CurveChannel>(['blur_radius', 'offset_x', 'offset_y', 'spread', 'shadow_depth', 'm31', 'm32']);
const channelLabels: Partial<Record<CurveChannel, string>> = {
    percent_x: 'X (%)', percent_y: 'Y (%)', absolute_x: 'X (px)', absolute_y: 'Y (px)',
    percent: 'Percent', absolute: 'Pixels',
    blur_radius: 'Blur radius (px)', offset_x: 'Offset X (px)', offset_y: 'Offset Y (px)', spread: 'Spread (px)',
    direction: 'Direction (°)', shadow_depth: 'Shadow depth (px)', shadow_opacity: 'Shadow opacity',
    angle: 'Angle (°)', angle_x: 'X (°)', angle_y: 'Y (°)',
    m31: 'M31 (px)', m32: 'M32 (px)',
    stop_offset: 'Offset',
};

export function channelComponent(channel: CurveChannel): CurveChannel {
    return channel.split('/').at(-1) as CurveChannel;
}

export function channelLabel(channel: CurveChannel): string {
    const component = channelComponent(channel);
    const label = channelLabels[component] ?? component[0].toUpperCase() + component.slice(1).replaceAll('_', ' ');
    if (component === channel) return label;
    if (channel.startsWith('box_shadow/')) return `Shadow ${channel.split('/')[1]} · ${label}`;
    if (channel.startsWith('gradient_stop/')) return `Stop ${channel.split('/')[1]} · ${label}`;
    if (channel.startsWith('gradient/')) {
        const [, field] = channel.split('/');
        if (field === 'opacity') return 'Brush opacity';
        if (field === 'angle') return label;
        const name = field[0].toUpperCase() + field.slice(1).replaceAll('_', ' ');
        return `${name} · ${label}`;
    }
    if (channel.startsWith('transform_matrix/')) return `Combined · ${label}`;
    if (channel.startsWith('transform_operation/')) {
        const [, index, kind] = channel.split('/');
        return `Operation ${index} · ${kind[0].toUpperCase() + kind.slice(1)} ${label}`;
    }
    const parameter = channel.split('/')[2].replace(/[_-]+/g, ' ')
        .replace(/(^|\s)(\p{L})/gu, (_, space: string, letter: string) => space + letter.toUpperCase());
    return component === 'value' ? parameter : `${parameter} ${label}`;
}

export function normalizedChannel(channel: CurveChannel): boolean {
    return isColorChannel(channel) || channel === 'shadow_opacity'
        || channel === 'gradient/opacity/value';
}

export function isColorChannel(channel: CurveChannel): boolean { return colorChannels.has(channelComponent(channel)); }

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
    readonly keys: { rendered: RenderedKeyframe; keyframe: TimelineKeyframe }[] = [];
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
        if (choices.length > 1 || choices[0]?.startsWith('effect_parameter/')) {
            const select = this.channelPicker = create('select');
            const name = `${track.property} curve channel`;
            select.setAttribute('aria-label', name);
            select.title = name;
            select.className = 'timeline-curve-channel';
            select.hidden = true;
            select.append(...choices.map(channel => new Option(channelLabel(channel), channel)));
            if (preferred && choices.includes(preferred)) select.value = preferred;
            select.onchange = () => changed(this.channel);
            label.append(select);
        }
    }

    get channel(): CurveChannel { return (this.channelPicker?.value as CurveChannel | undefined) ?? curveChannels(this.track)[0] ?? 'value'; }
    has(key: string): boolean { return this.accepted?.key === key; }

    handle(keyframe: TimelineKeyframe, current: string | undefined): CurveHandle | undefined {
        const value = this.value(keyframe);
        if (value === undefined || !this.scale || this.painted?.metric !== 'value' || !current || !this.has(current)) return;
        return { value, scale: this.scale, channel: this.channel, normalized: normalizedChannel(this.channel),
            step: this.component(keyframe)?.step,
            keyRow: this.channel === 'value' ? 0 : 20 };
    }

    private component(keyframe: TimelineKeyframe) {
        return keyframe.curve_values?.find(item => item.channel === this.channel);
    }

    private value(keyframe: TimelineKeyframe): number | undefined {
        const value = this.component(keyframe)?.value
            ?? (this.channel === 'value' && keyframe.value_kind === 'Number' ? Number(keyframe.value_text) : undefined);
        return value !== undefined && Number.isFinite(value) ? value : undefined;
    }


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
            for (const { rendered } of this.keys) {
                rendered.button.style.top = '';
                rendered.button.removeAttribute('data-curve-key');
                rendered.button.removeAttribute('aria-description');
            }
            return;
        }
        const scalar = channel === 'value' && metric === 'value';
        this.lane.dataset.curve = metric === 'value' && !scalar ? 'component' : metric;
        const component = channelComponent(channel);
        this.line.dataset.channel = component;
        const visible = points?.filter(point => visibleTime(view, point.time)) ?? [];
        // Include neighbors so panning can move immediately while the current range samples arrive.
        const first = visible[0];
        const last = visible.at(-1);
        const before = points?.findLast(point => point.time < (first?.time ?? view.start));
        const after = points?.find(point => point.time > (last?.time ?? view.start + view.span));
        const isVelocity = metric === 'velocity';
        const handles = this.keys.map(({ keyframe }) => isVelocity ? undefined : this.value(keyframe));
        const units = this.keys[0] && this.component(this.keys[0].keyframe)?.step;
        const keyValues = isVelocity ? [0] : normalizedChannel(channel) || units === 'boolean' ? [0, 1]
            : handles.flatMap((value, index) => {
                if (value === undefined || !visibleTime(view, this.keys[index].rendered.key.time)) return [];
                return units === 'integer' ? [snapCurveInteger(value - 1), snapCurveInteger(value + 1)] : [value];
            });
        const allValues = handles.filter(value => value !== undefined);
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
        this.scaleLabels.toggleAttribute('data-waiting', !scalar && !points?.length);
        this.ticks[0].textContent = axisNumber.format(low);
        this.ticks[1].textContent = axisNumber.format(high);
        this.unit.textContent = component.startsWith('percent') ? (isVelocity ? 'pp/s' : '%')
            : angleChannels.has(component) ? (isVelocity ? '°/s' : '°')
            : component.startsWith('absolute') || pixelChannels.has(component)
                ? (isVelocity ? 'px/s' : 'px') : isVelocity ? '/s' : '';
        this.zeroLine.toggleAttribute('data-visible', isVelocity && Boolean(points?.length) && low < 0 && high > 0);
        const baseline = scale.percent(0).toFixed(3);
        this.zeroLine.setAttribute('y1', baseline);
        this.zeroLine.setAttribute('y2', baseline);
        this.keys.forEach(({ rendered }, index) => {
            const value = handles[index];
            rendered.button.toggleAttribute('data-curve-key', value !== undefined);
            if (value === undefined) rendered.button.removeAttribute('aria-description');
            else rendered.button.setAttribute('aria-description', `${channelLabel(channel)}, ${axisNumber.format(value)}`);
            rendered.button.style.top = value === undefined ? ''
                : curveKeyPosition(scale.percent(value), scalar ? 0 : 20);
        });
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

    add(lane: HTMLElement, label: HTMLElement, track: TimelineTrack, identity: string): TimelineCurvePlot {
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

    handle(plot: TimelineCurvePlot, keyframe: TimelineKeyframe): CurveHandle | undefined {
        return plot.handle(keyframe, this.key(plot));
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
