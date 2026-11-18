import type { FrameTarget } from './protocol';

export interface TimelineKeyframe { time: number; value_text: string; easing_text: string }
export interface TimelineTrack { target: number; property: string; keyframes: TimelineKeyframe[] }
export interface KeyframeLocation { target: number; property: string; time: number }
export type KeyframeGroupEdit = { action: 'move_keyframes' | 'duplicate_keyframes'; offset: number }
    | { action: 'remove_keyframes' };
export interface AuthoredTimeline {
    duration: number;
    delay: number;
    delay_between_iterations: number;
    speed_ratio: number;
    repeat_text: string;
    playback_direction: string;
    playback_behavior: string;
    fill: string;
    easing_text: string;
    total_duration: number | null;
    tracks: TimelineTrack[];
}
export interface TimelineProgram {
    id: number;
    source_key?: string | null;
    name: string;
    owner: number;
    style?: number | null;
    targets: number[];
    selector: string;
    timeline: AuthoredTimeline | null;
}

export interface TimelinePosition {
    session: number;
    program: number;
    time: number;
    elapsed?: number;
    playing: boolean;
    busy: boolean;
}

export interface TimelinePreviewPose {
    program: number;
    time: number;
    sourceKey?: string;
    anchor?: object;
}

export interface TimelineState {
    frame?: FrameTarget;
    programs: TimelineProgram[];
    names: Record<number, string>;
    writable: boolean;
    canRecord?: boolean;
    recording?: boolean;
    position?: TimelinePosition;
    message?: string;
}

export type TimelineTransportControl =
    | { action: 'seek'; program: number; time: number }
    | { action: 'play'; program: number; time?: number }
    | { action: 'pause' | 'stop'; program: number };

export type TimelineControl = TimelineTransportControl
    | { action: 'record'; program: number; enabled: boolean; time?: number };

export type FromTimeline =
    | { type: 'ready' }
    | { type: 'control'; frame: FrameTarget; control: TimelineControl }
    | { type: 'programActions'; frame: FrameTarget; program: number | undefined }
    | { type: 'recordKeyframe'; frame: FrameTarget; program: number; time: number }
    | { type: 'keyframeGroup'; frame: FrameTarget; program: number; keys: KeyframeLocation[]; edit?: KeyframeGroupEdit }
    | { type: 'keyframe'; frame: FrameTarget; program: number; track: number; time: number };

export type ToTimeline = { type: 'position'; position?: TimelinePosition; frame?: FrameTarget }
    | { type: 'keyframesEdited'; keys?: KeyframeLocation[]; before: FrameTarget }
    | { type: 'error'; message: string };
