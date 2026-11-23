export const timelineRulers = ['tracks', 'clips', 'curve'] as const;
export type TimelineRuler = typeof timelineRulers[number];

export interface TimelineViewportState {
    ruler: TimelineRuler;
    program?: number;
    start: number;
    end: number;
    duration: number;
    canFitSelection: boolean;
}

export interface TimelineViewportChange {
    ruler: TimelineRuler;
    program?: number;
    action: 'window' | 'fit' | 'fit_selection';
    start?: number;
    end?: number;
}

export interface TimelineViewportRequest<Frame> {
    type: 'timelineViewportRequest';
    request: number;
    frame: Frame;
    change?: TimelineViewportChange;
}

export type TimelineViewportReply = { type: 'timelineViewportReply'; request: number } &
    ({ viewport?: TimelineViewportState } | { error: string });
