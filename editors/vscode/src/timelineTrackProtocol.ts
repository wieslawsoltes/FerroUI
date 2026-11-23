import type { CurveChannel } from './timelineProtocol';

export interface TimelineTrackIdentity {
    target: number;
    property: string;
}

export interface TimelineTrackInfo extends TimelineTrackIdentity {
    name: string;
    visible: boolean;
    channel?: CurveChannel;
    channels?: readonly CurveChannel[];
    canChooseChannel?: boolean;
}

export interface TimelineTrackGroup {
    target: number;
    name: string;
    visible: boolean;
    expanded: boolean;
    locked: boolean;
    canCollapse: boolean;
    canLock: boolean;
}

export interface TimelineTrackFilterState {
    query: string;
    selectedOnly: boolean;
}

export interface TimelineTrackView {
    program: number;
    filter?: TimelineTrackFilterState;
    groups?: TimelineTrackGroup[];
    tracks: TimelineTrackInfo[];
    selected?: TimelineTrackIdentity;
}

export type TimelineTrackCommand = { program: number } & (
    | ({ action: 'filter' } & TimelineTrackFilterState)
    | { action: 'collapse'; target: number; collapsed: boolean }
    | { action: 'lock'; target: number; locked: boolean }
    | ({ action: 'select' } & TimelineTrackIdentity)
    | ({ action: 'channel'; channel: CurveChannel } & TimelineTrackIdentity)
);
