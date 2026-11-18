import type { FrameTarget } from './protocol';

export interface TimelineKeyframe { time: number; value_text: string; easing_text: string }
export interface TimelineTrack { target: number; property: string; keyframes: TimelineKeyframe[] }
export interface TimelineProgram {
    id: number;
    name: string;
    owner: number;
    targets: number[];
    selector: string;
    timeline: { duration: number; tracks: TimelineTrack[] } | null;
}

export interface TimelinePosition {
    session: number;
    program: number;
    time: number;
    playing: boolean;
    busy: boolean;
}

export interface TimelineState {
    frame?: FrameTarget;
    programs: TimelineProgram[];
    names: Record<number, string>;
    writable: boolean;
    position?: TimelinePosition;
    message?: string;
}

export type TimelineControl =
    | { action: 'seek'; program: number; time: number }
    | { action: 'play'; program: number; time?: number }
    | { action: 'pause'; program: number };

export type FromTimeline =
    | { type: 'ready' }
    | { type: 'control'; frame: FrameTarget; control: TimelineControl }
    | { type: 'recordKeyframe'; frame: FrameTarget; program: number; time: number }
    | ({ frame: FrameTarget; program: number; track: number; time: number }
        & ({ type: 'keyframe' } | { type: 'moveKeyframe'; to: number }));

export type ToTimeline = { type: 'position'; position?: TimelinePosition; frame?: FrameTarget }
    | { type: 'error'; message: string };
