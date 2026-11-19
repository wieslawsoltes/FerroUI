export interface TimelineViewport {
    duration: number;
    start: number;
    span: number;
}

export const minimumTimelineSpan = 0.05;

export function fitTimeline(duration: number): TimelineViewport {
    return { duration, start: 0, span: duration };
}

export function panTimeline(view: TimelineViewport, start: number): TimelineViewport {
    return { ...view, start: clamp(start, 0, view.duration - view.span) };
}

export function zoomTimeline(view: TimelineViewport, factor: number, time: number): TimelineViewport {
    if (view.span <= 0) return view;
    const span = clamp(view.span / factor, Math.min(minimumTimelineSpan, view.duration), view.duration);
    const fraction = clamp((time - view.start) / view.span, 0, 1);
    const pivot = view.start + view.span * fraction;
    return panTimeline({ ...view, span }, pivot - span * fraction);
}

export function fitKeyTimes(duration: number, keys: readonly number[]): TimelineViewport {
    if (!keys.length || duration <= 0) return fitTimeline(duration);
    let first = duration;
    let last = 0;
    for (const time of keys) {
        first = Math.min(first, time);
        last = Math.max(last, time);
    }
    const span = Math.min(duration, Math.max(minimumTimelineSpan, duration * .06, (last - first) * 1.2));
    return panTimeline({ duration, start: 0, span }, first + (last - first - span) / 2);
}

export function timelinePercent(view: TimelineViewport, time: number): number {
    return view.span > 0 ? 100 * (time - view.start) / view.span : 0;
}

export function visibleTime(view: TimelineViewport, time: number): boolean {
    // Include endpoint keys when the visible span and its offset have differing precision.
    const percent = timelinePercent(view, time);
    return percent >= -1e-8 && percent <= 100 + 1e-8;
}

export function stepTimelineFrame(time: number, frames: number, rate: number, duration: number): number {
    const frame = time * rate;
    const exact = Math.abs(frame - Math.round(frame)) < 1e-8 ? Math.round(frame) : frame;
    const origin = frames < 0 ? Math.ceil(exact) : Math.floor(exact);
    return clamp((origin + frames) / rate, 0, duration);
}

export function clamp(value: number, minimum: number, maximum: number): number {
    return Math.min(maximum, Math.max(minimum, value));
}
