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
