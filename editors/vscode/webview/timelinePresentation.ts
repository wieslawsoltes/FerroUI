import type { TimelinePresentationReply, TimelinePresentationRequest } from '../src/timelinePresentationProtocol';
import type { TimelineTrackCommand, TimelineTrackView } from '../src/timelineTrackProtocol';
import { changeTimelineViewport, readTimelineViewport, type TimelineRulerView } from './timelineViewportRequest';

export interface TimelinePresentationView {
    ruler(): TimelineRulerView | undefined;
    tracks(): TimelineTrackView | undefined;
    changeTracks(command: TimelineTrackCommand): void;
}

export function answerTimelinePresentation({ request, change }: TimelinePresentationRequest<unknown>,
    view?: TimelinePresentationView): TimelinePresentationReply {
    const reply = { type: 'timelinePresentationReply', request } as const;
    try {
        if (change && !view) throw new Error('Read the current Timeline revision and program again.');
        if (change?.kind === 'viewport') changeTimelineViewport(change.viewport, view?.ruler());
        if (change?.kind === 'tracks') view?.changeTracks(change.track);
        return { ...reply, presentation: { viewport: readTimelineViewport(view?.ruler()), track_view: view?.tracks() } };
    } catch (error) {
        return { ...reply, error: error instanceof Error ? error.message : String(error) };
    }
}
