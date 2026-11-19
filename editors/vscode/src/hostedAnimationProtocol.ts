import type { AnimationSourceIssue, KeyframeDraft, KeyframeDraftPreview, TimelineClipTiming, TimelineCurveRequest, TimelineKeyframe, TimelineMarker, TimelineMarkerAction, TimelinePreviewRange, TimelineTrack } from './timelineProtocol';
import type { Range } from './protocol';
import type { StateTransitionRoute } from './stateTransitionProtocol';
import type { KeyframeGroupEdit, KeyframeLocation, KeyframePaste, TimelineKeyAction } from './timelineProtocol';

export interface HostedStateRoute extends StateTransitionRoute {
    easing: string;
    sourceRange: Range;
}

export interface HostedAnimationProgram {
    id: number;
    name: string;
    selector: string;
    duration: number;
    totalDuration: number | null;
    defaultTargets: number[];
    compatibleTargets: number[];
    sourceOffset: number | null;
    sourceKey?: string;
    markerKey?: string;
    customAnimator: boolean;
    authoring?: { codec: string; tracks: TimelineTrack[] } | null;
    curveTracks?: TimelineTrack[] | null;
    clip?: TimelineClipTiming | null;
    stateRoute?: HostedStateRoute;
}

export interface HostedAnimationTarget {
    id: number;
    label: string;
    typeName: string;
}

export interface HostedClipOwner extends HostedAnimationTarget { selector: string; sourceLine: number }
export interface HostedClipAuthoring {
    owners: HostedClipOwner[];
    programs: number[];
}

export interface HostedAnimationCatalog {
    programs: HostedAnimationProgram[];
    targets: HostedAnimationTarget[];
    sourceIssues?: AnimationSourceIssue[];
    clipScope?: string;
    markerScope?: string;
    keyframeSelection?: boolean;
    clipAuthoring?: HostedClipAuthoring;
}

export interface HostedAnimationFrame {
    revision: number;
    sequence: number;
}

export interface HostedAnimationRequest extends HostedAnimationFrame {
    program?: number;
    targets?: number[];
    clips?: { program: number; targets?: number[] }[];
    elapsed: number;
    restart: boolean;
}

export type HostedAnimationControl =
    | { action: 'select'; program: number }
    | { action: 'seek'; program: number; elapsed: number }
    | { action: 'play'; program: number; elapsed?: number }
    | { action: 'scrub_all'; elapsed: number }
    | { action: 'play_all'; elapsed?: number }
    | { action: 'play_range'; program?: number; range: TimelinePreviewRange }
    | { action: 'marker'; program?: number; marker: TimelineMarkerAction }
    | { action: 'pause' }
    | { action: 'stop' }
    | { action: 'targets'; program: number; targets?: number[] };

export type HostedAnimationAction = HostedAnimationControl
    | { action: 'reveal'; program: number }
    | { action: 'reveal_issue'; index: number };

export interface HostedAnimationPosition {
    program: number;
    elapsed: number;
    playing: boolean;
    completed: boolean;
    clips?: number[];
    previewRange?: TimelinePreviewRange;
}

export interface HostedAnimationPresentation {
    selection?: number;
    position?: HostedAnimationPosition;
    overrides: Record<number, number[]>;
}

export interface HostedAnimationRestore {
    frame: HostedAnimationFrame;
    presentation: HostedAnimationPresentation;
}

export interface HostedAnimationState extends HostedAnimationCatalog, HostedAnimationPresentation {
    frame?: HostedAnimationFrame;
    busy: boolean;
    canShowVelocity?: boolean;
    canPreviewRanges?: boolean;
    markers?: { programs: Record<number, TimelineMarker[]>; shared: TimelineMarker[] };
}

export type HostedKeyframeDraft = Omit<KeyframeDraft, 'previous'> & { previous?: number };
export type HostedCurveQuery = Pick<TimelineCurveRequest, 'program' | 'target' | 'property' | 'window' | 'metric' | 'channel'>;
export interface HostedStateDraft { program: number; duration: number; easing: string }
export interface HostedClipDraft { programs: number[]; start?: number; gap?: number }
export type HostedClipEdit =
    | { action: 'create'; owner: number; name: string; selector: string; duration: number }
    | { action: 'rename' | 'duplicate'; program: number; name: string }
    | { action: 'remove'; program: number };
export interface HostedClipPlan extends HostedSourcePlan { selectedKey: string | null }
export interface HostedSourcePatch { start: number; end: number; expected: string; replacement: string }
export interface HostedSourcePlan { patches: HostedSourcePatch[] }

export type HostedKeyframeCommand = ({ keyframes: KeyframeLocation[] } & (KeyframeGroupEdit
    | { action: 'copy_keyframes' } | { action: 'mirror_keyframes'; pivot: number }))
    | { action: 'paste_keyframes'; paste: KeyframePaste };
export interface HostedKeyframeGroupDraft { program: number; command: HostedKeyframeCommand }
export interface HostedKeyframeGroupPlan extends HostedSourcePlan {
    keys: KeyframeLocation[] | null;
    clipboard: string | null;
}
export interface HostedKeyframeGroupAction {
    program: number;
    keys: KeyframeLocation[];
    pivot: number;
    edit?: TimelineKeyAction;
}

export interface HostedKeyframePlan extends KeyframeDraftPreview {
    patch: HostedSourcePatch | null;
}

export interface HostedKeyframeSelection {
    frame: HostedAnimationFrame;
    version: number;
    program: number;
    target: number;
    targetName: string;
    property: string;
    duration: number;
    keyframe: TimelineKeyframe;
}
