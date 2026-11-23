import type { TimelinePreviewRange } from './timelineProtocol';

export interface TimelineWorkAreaScope {
    scope: 'program' | 'clips';
    program?: number;
}

export interface TimelineWorkAreaState extends TimelineWorkAreaScope {
    clock: 'authored' | 'elapsed';
    limit: number | null;
    playhead: number;
    range: TimelinePreviewRange;
    enabled: boolean;
    editable: boolean;
}

export type TimelineWorkAreaEdit =
    | { action: 'set'; range: TimelinePreviewRange }
    | { action: 'enable' | 'disable' | 'set_in' | 'set_out' };

export type TimelineWorkAreaChange = TimelineWorkAreaScope & TimelineWorkAreaEdit;

export interface TimelineSnapChange {
    program?: number;
    frame_rate: number;
}

export interface TimelineSnapState extends TimelineSnapChange {
    rates: readonly number[];
    editable: boolean;
}
