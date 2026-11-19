import type { FrameTarget, Range } from './protocol';
import type { StateTransitionControl, StateTransitionPosition } from './stateTransitionProtocol';

export interface TimelineKeyframe { time: number; value_text: string; easing_text: string; value_kind?: string }
export interface TimelineTrack { target: number; property: string; keyframes: TimelineKeyframe[]; curve_channels?: CurveChannel[] }
export interface AnimationSourceIssue { range: Range; reason: string }
export interface KeyframeLocation { target: number; property: string; time: number }
export type KeyframeGroupEdit = { action: 'move_keyframes' | 'duplicate_keyframes'; offset: number }
    | { action: 'scale_keyframes'; pivot: number; factor: number }
    | { action: 'remove_keyframes' };
export type TimelineKeyAction = KeyframeGroupEdit | { action: 'copy_keyframes' } | { action: 'mirror_keyframes' }
    | { action: 'paste_keyframes'; reverse: boolean; track?: number };
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
    clip?: { start: number; end: number | null; editable: boolean };
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
    previewRange?: TimelinePreviewRange;
    playing: boolean;
    busy: boolean;
}

export interface TimelinePreviewRange {
    start: number;
    end: number;
    loop: boolean;
}

export interface TimelineMarker { name: string; time: number }
export type TimelineMarkerEdit = { action: 'set'; name: string; time: number } | { action: 'remove'; name: string };

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
    preferredProgram?: string;
    programs: TimelineProgram[];
    sourceIssues?: AnimationSourceIssue[];
    names: Record<number, string>;
    selectedTargets?: readonly number[];
    markers?: Record<number, TimelineMarker[]>;
    writable: boolean;
    canRecord?: boolean;
    canRecordGeometry?: boolean;
    canEditKeyframes?: boolean;
    canTransferKeys?: boolean;
    canAuthorRustKeys?: boolean;
    canCreateRustProgram?: boolean;
    canShowCurves?: boolean;
    canShowVelocity?: boolean;
    curveCatalog?: string;
    canPlayAll?: boolean;
    clipScope?: string;
    recording?: boolean;
    position?: TimelinePosition;
    transition?: StateTransitionPosition;
    message?: string;
}

export type TimelineTransportControl =
    | { action: 'seek' | 'scrub_all'; program: number; time: number }
    | { action: 'play_range'; program: number; range: TimelinePreviewRange }
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
    | { type: 'sourceIssues'; frame: FrameTarget }
    | { type: 'clipPlacement'; frame: FrameTarget; programs: number[]; start?: number; gap?: number }
    | { type: 'markerEdit'; frame: FrameTarget; program: number; edit: TimelineMarkerEdit }
    | { type: 'recordKeyframe'; frame: FrameTarget; program: number; time: number; track?: number; candidates?: number[] }
    | TimelineCurveRequest
    | TimelineKeyframeRequest
    | { type: 'keyframeGroup'; frame: FrameTarget; program: number; keys: KeyframeLocation[]; edit?: TimelineKeyAction; destinations?: number[] }
    | { type: 'keyframe'; frame: FrameTarget; program: number; track: number; time: number };

export type ToTimeline = { type: 'position'; position?: TimelinePosition; frame?: FrameTarget }
    | { type: 'clipsPlaced'; before: FrameTarget }
    | { type: 'keyframesEdited'; keys?: KeyframeLocation[]; before: FrameTarget }
    | TimelineKeyframeResponse
    | TimelineCurveResponse
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

export interface CurvePoint { time: number; value: number }
export type CurveMetric = 'value' | 'velocity';
export type CurveChannel = 'value' | 'x' | 'y' | 'width' | 'height' | 'left' | 'top' | 'right' | 'bottom'
    | 'top_left' | 'top_right' | 'bottom_left' | 'bottom_right';

export interface TimelineCurveRequest {
    type: 'curve';
    frame: FrameTarget;
    request: number;
    program: number;
    target: number;
    property: string;
    window: { start: number; end: number };
    metric?: CurveMetric;
    channel?: CurveChannel;
}

export interface TimelineCurveResponse {
    type: 'curveResult';
    frame: FrameTarget;
    request: number;
    points?: CurvePoint[];
    error?: string;
    stale?: boolean;
}
