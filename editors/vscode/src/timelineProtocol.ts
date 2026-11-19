import type { FrameTarget, Range } from './protocol';
import type { StateTransitionControl, StateTransitionPosition } from './stateTransitionProtocol';

export interface TimelineKeyframe { time: number; value_text: string; easing_text: string; value_kind?: string }
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
    sourceRange?: Range;
    name: string;
    owner: number;
    style?: number | null;
    targets: number[];
    selector: string;
    timeline: AuthoredTimeline | null;
    preview?: TimelineProgramPreview;
    diagnostics?: string[];
}

export interface TimelineClock {
    duration: number;
    total_duration: number | null;
    native?: boolean;
}

export interface TimelineProgramPreview extends TimelineClock {
    native: boolean;
    available: boolean;
    explicit_targets: boolean;
}

/** Source anchors owned by the native preview engine. */
export type TimelineTargetOverrides = readonly object[];

export interface TimelinePosition {
    session: number;
    program: number;
    time: number;
    elapsed?: number;
    all?: boolean;
    completed?: boolean;
    playing: boolean;
    busy: boolean;
}

export interface TimelinePreviewPose {
    program: number;
    time: number;
    sourceKey?: string;
    anchor?: object;
    playback?: { elapsed: number; all: boolean; completed?: boolean };
}

export interface TimelineState {
    frame?: FrameTarget;
    documentVersion?: number;
    programs: TimelineProgram[];
    names: Record<number, string>;
    writable: boolean;
    canRecord?: boolean;
    canRecordGeometry?: boolean;
    canEditKeyframes?: boolean;
    canPlayAll?: boolean;
    recording?: boolean;
    position?: TimelinePosition;
    transition?: StateTransitionPosition;
    message?: string;
}

export type TimelineTransportControl =
    | { action: 'seek'; program: number; time: number }
    | { action: 'play' | 'play_all'; program: number; time?: number }
    | { action: 'pause' | 'stop'; program: number };

export type TimelineControl = TimelineTransportControl
    | { action: 'record'; program: number; enabled: boolean; time?: number };

export type FromTimeline =
    | { type: 'ready' }
    | { type: 'control'; frame: FrameTarget; control: TimelineControl }
    | { type: 'transitionControl'; frame: FrameTarget; control: StateTransitionControl }
    | { type: 'programActions'; frame: FrameTarget; program: number | undefined }
    | { type: 'programTargets'; frame: FrameTarget; program: number }
    | { type: 'revealProgram'; frame: FrameTarget; program: number }
    | { type: 'recordKeyframe'; frame: FrameTarget; program: number; time: number; track?: number }
    | TimelineKeyframeRequest
    | { type: 'keyframeGroup'; frame: FrameTarget; program: number; keys: KeyframeLocation[]; edit?: KeyframeGroupEdit }
    | { type: 'keyframe'; frame: FrameTarget; program: number; track: number; time: number };

export type ToTimeline = { type: 'position'; position?: TimelinePosition; frame?: FrameTarget }
    | { type: 'keyframesEdited'; keys?: KeyframeLocation[]; before: FrameTarget }
    | TimelineKeyframeResponse
    | { type: 'error'; message: string };

export interface KeyframeDraft {
    program: number;
    target: number;
    property: string;
    previous: number;
    time: number;
    value: string;
    easing: string;
}

export interface TimelineKeyframeRequest {
    type: 'keyframeDraft';
    frame: FrameTarget;
    request: number;
    draft: KeyframeDraft;
    apply: boolean;
}

export interface KeyframeDraftPreview { samples: number[]; valueKind: string }

export interface AppliedKeyframe {
    frame: FrameTarget;
    version: number;
    program: number;
    target: number;
    keyframe: TimelineKeyframe;
}

export interface TimelineKeyframeResponse {
    type: 'keyframeDraftResult';
    frame: FrameTarget;
    request: number;
    preview?: KeyframeDraftPreview;
    applied?: AppliedKeyframe | null;
    error?: string;
}
