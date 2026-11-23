import type { TimelineRuler, TimelineViewportChange, TimelineViewportState } from '../src/timelineViewportProtocol';
import { fitKeyTimes, fitTimeline, minimumTimelineSpan, type TimelineViewport } from './timelineViewport';

export interface TimelineRulerView {
    ruler: TimelineRuler;
    program?: number;
    range: TimelineViewport;
    selectedKeys?: readonly number[];
    apply(range: TimelineViewport): void;
}

export function readTimelineViewport(view?: TimelineRulerView): TimelineViewportState | undefined {
    if (!view || view.range.duration <= 0) return;
    const { duration, start, span } = view.range;
    return { ruler: view.ruler, program: view.program, start,
        end: Math.min(duration, start + span), duration, canFitSelection: Boolean(view.selectedKeys?.length) };
}

export function changeTimelineViewport(change: TimelineViewportChange, view?: TimelineRulerView): void {
    if (!view || view.range.duration <= 0) throw new Error('Show a Timeline ruler with an animation or clips first.');
    if (view.ruler !== change.ruler || view.program !== change.program) {
        throw new Error('The visible ruler or animation changed. Read the Timeline view again.');
    }
    const { duration } = view.range;
    if (change.action === 'fit') {
        view.apply(fitTimeline(duration));
    } else if (change.action === 'fit_selection') {
        if (!view.selectedKeys?.length) throw new Error('Select keys on the visible curve first.');
        view.apply(fitKeyTimes(duration, view.selectedKeys));
    } else {
        const { start, end } = change;
        if (start === undefined || end === undefined || !Number.isFinite(start) || !Number.isFinite(end)
            || start < 0 || end > duration || end <= start
            || end - start + Number.EPSILON * duration < Math.min(minimumTimelineSpan, duration)) {
            throw new Error('Choose a window within the ruler of at least 0.05 seconds, or fit the entire ruler.');
        }
        view.apply({ duration, start, span: end - start });
    }
}
