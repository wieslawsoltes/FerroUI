import type { TimelineTrackCommand, TimelineTrackView } from './timelineTrackProtocol';
import type { TimelineViewportChange, TimelineViewportState } from './timelineViewportProtocol';

export interface TimelinePresentation {
    viewport?: TimelineViewportState;
    track_view?: TimelineTrackView;
}

export type TimelinePresentationChange =
    | { kind: 'viewport'; viewport: TimelineViewportChange }
    | { kind: 'tracks'; track: TimelineTrackCommand };

export interface TimelinePresentationRequest<Frame> {
    type: 'timelinePresentationRequest';
    request: number;
    frame: Frame;
    change?: TimelinePresentationChange;
}

export type TimelinePresentationReply = { type: 'timelinePresentationReply'; request: number } &
    ({ presentation: TimelinePresentation } | { error: string });
