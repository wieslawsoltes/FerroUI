import type { TimelineTrackCommand, TimelineTrackView } from './timelineTrackProtocol';
import type { TimelineSnapChange, TimelineSnapState, TimelineWorkAreaChange, TimelineWorkAreaState } from './timelineTimingProtocol';
import type { TimelineViewportChange, TimelineViewportState } from './timelineViewportProtocol';

export interface TimelinePresentation {
    viewport?: TimelineViewportState;
    track_view?: TimelineTrackView;
    work_area?: TimelineWorkAreaState;
    frame_snap?: TimelineSnapState;
}

export type TimelinePresentationChange =
    | { kind: 'viewport'; viewport: TimelineViewportChange }
    | { kind: 'tracks'; track: TimelineTrackCommand }
    | { kind: 'work_area'; work_area: TimelineWorkAreaChange }
    | { kind: 'snap'; snap: TimelineSnapChange };

export interface TimelinePresentationRequest<Frame> {
    type: 'timelinePresentationRequest';
    request: number;
    frame: Frame;
    change?: TimelinePresentationChange;
}

export type TimelinePresentationReply = { type: 'timelinePresentationReply'; request: number } &
    ({ presentation: TimelinePresentation } | { error: string });
