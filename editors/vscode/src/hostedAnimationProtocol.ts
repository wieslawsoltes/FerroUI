import type { KeyframeDraft, KeyframeDraftPreview, TimelineKeyframe, TimelineTrack } from './timelineProtocol';

export interface HostedAnimationProgram {
    id: number;
    name: string;
    selector: string;
    duration: number;
    totalDuration: number | null;
    defaultTargets: number[];
    compatibleTargets: number[];
    sourceOffset: number;
    customAnimator: boolean;
    authoring?: { codec: string; tracks: TimelineTrack[] } | null;
}

export interface HostedAnimationTarget {
    id: number;
    label: string;
    typeName: string;
}

export interface HostedAnimationCatalog {
    programs: HostedAnimationProgram[];
    targets: HostedAnimationTarget[];
}

export interface HostedAnimationFrame {
    revision: number;
    sequence: number;
}

export interface HostedAnimationRequest extends HostedAnimationFrame {
    program?: number;
    targets?: number[];
    elapsed: number;
    restart: boolean;
}

export type HostedAnimationControl =
    | { action: 'select'; program: number }
    | { action: 'seek'; program: number; elapsed: number }
    | { action: 'play'; program: number; elapsed?: number }
    | { action: 'pause' }
    | { action: 'stop' }
    | { action: 'targets'; program: number; targets?: number[] };

export interface HostedAnimationPosition {
    program: number;
    elapsed: number;
    playing: boolean;
    completed: boolean;
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
}

export type HostedKeyframeDraft = Omit<KeyframeDraft, 'previous'> & { previous?: number };

export interface HostedKeyframePlan extends KeyframeDraftPreview {
    patch: { start: number; end: number; expected: string; replacement: string } | null;
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
