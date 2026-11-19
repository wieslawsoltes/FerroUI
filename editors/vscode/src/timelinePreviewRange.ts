import type { TimelinePreviewRange } from './timelineProtocol';

export function validPreviewRange(range: TimelinePreviewRange | undefined, duration: number): range is TimelinePreviewRange {
    return Boolean(range && Number.isFinite(range.start) && Number.isFinite(range.end) && range.start >= 0
        && range.start < range.end && range.end <= duration && typeof range.loop === 'boolean');
}

/** Advance authored time. Each returned position is rendered using the native seek operation. */
export function stepPreviewRange(range: TimelinePreviewRange, current: number, seconds: number): { time: number; completed: boolean } {
    const span = range.end - range.start;
    if (range.loop) return { time: Math.min(range.end, range.start + ((current - range.start + seconds) % span)), completed: false };
    const time = Math.min(range.end, current + seconds);
    return { time, completed: time >= range.end };
}
