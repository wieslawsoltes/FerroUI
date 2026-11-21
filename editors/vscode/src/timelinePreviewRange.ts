import type { TimelinePreviewRange } from './timelineProtocol';

export function validPreviewRange(range: TimelinePreviewRange | undefined, duration: number): range is TimelinePreviewRange {
    return Boolean(range && Number.isFinite(range.start) && Number.isFinite(range.end) && range.start >= 0
        && range.start < range.end && range.end <= duration && range.end * 10_000_000 < 2 ** 63
        && typeof range.loop === 'boolean');
}

/** Advance on the work area's clock. A one-shot holds the exact Out time. */
export function stepPreviewRange(range: TimelinePreviewRange, current: number, seconds: number): { time: number; completed: boolean } {
    const remaining = range.end - current;
    if (seconds < remaining) return { time: Math.min(range.end, current + seconds), completed: false };
    if (range.loop) {
        const offset = (seconds - remaining) % (range.end - range.start);
        return { time: Math.min(range.end, range.start + offset), completed: false };
    }
    return { time: range.end, completed: true };
}
