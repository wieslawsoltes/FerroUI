import type { TimelineRuler, TimelineViewportReply, TimelineViewportRequest } from '../src/timelineViewportProtocol';
import { fitKeyTimes, fitTimeline, minimumTimelineSpan, type TimelineViewport } from './timelineViewport';

export interface TimelineRulerView {
    ruler: TimelineRuler;
    program?: number;
    range: TimelineViewport;
    selectedKeys?: readonly number[];
    apply(range: TimelineViewport): void;
}

export function answerTimelineViewport({ request, change }: TimelineViewportRequest<unknown>, view?: TimelineRulerView): TimelineViewportReply {
    const reply = { type: 'timelineViewportReply', request } as const;
    if (!view || view.range.duration <= 0) {
        return change ? { ...reply, error: 'Show a Timeline ruler with an animation or clips first.' } : reply;
    }
    let range = view.range;
    const { duration } = range;
    if (change) {
        if (view.ruler !== change.ruler || view.program !== change.program) {
            return { ...reply, error: 'The visible ruler or animation changed. Read the Timeline view again.' };
        }
        if (change.action === 'fit') {
            range = fitTimeline(duration);
        } else if (change.action === 'fit_selection') {
            if (!view.selectedKeys?.length) return { ...reply, error: 'Select keys on the visible curve first.' };
            range = fitKeyTimes(duration, view.selectedKeys);
        } else {
            const { start, end } = change;
            if (start === undefined || end === undefined || !Number.isFinite(start) || !Number.isFinite(end)
                || start < 0 || end > duration || end <= start
                || end - start + Number.EPSILON * duration < Math.min(minimumTimelineSpan, duration)) {
                return { ...reply, error: 'Choose a window within the ruler of at least 0.05 seconds, or fit the entire ruler.' };
            }
            range = { duration, start, span: end - start };
        }
        view.apply(range);
    }
    return { ...reply, viewport: { ruler: view.ruler, program: view.program, start: range.start,
        end: Math.min(duration, range.start + range.span), duration, canFitSelection: Boolean(view.selectedKeys?.length) } };
}
