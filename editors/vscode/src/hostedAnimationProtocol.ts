import type { AnimationSourceIssue, KeyframeDraft, KeyframeDraftPreview, TimelineClipTiming, TimelineCurveRequest, TimelineKeyframe, TimelinePreviewRange, TimelineTrack } from './timelineProtocol';
import type { Range } from './protocol';
import type { StateTransitionRoute } from './stateTransitionProtocol';

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

export interface HostedAnimationCatalog {
    programs: HostedAnimationProgram[];
    targets: HostedAnimationTarget[];
    sourceIssues?: AnimationSourceIssue[];
    clipScope?: string;
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
}

export type HostedKeyframeDraft = Omit<KeyframeDraft, 'previous'> & { previous?: number };
export type HostedCurveQuery = Pick<TimelineCurveRequest, 'program' | 'target' | 'property' | 'window' | 'metric' | 'channel'>;
export interface HostedStateDraft { program: number; duration: number; easing: string }
export interface HostedClipDraft { programs: number[]; start?: number; gap?: number }
export interface HostedSourcePatch { start: number; end: number; expected: string; replacement: string }
export interface HostedSourcePlan { patches: HostedSourcePatch[] }

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
