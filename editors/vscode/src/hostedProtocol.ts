import type { Source } from './protocol';
import type { HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationPresentation, HostedAnimationRestore,
    HostedAnimationState, HostedCurveQuery, HostedKeyframeDraft, HostedKeyframeSelection } from './hostedAnimationProtocol';
import type { CurvePoint, KeyframeDraftPreview } from './timelineProtocol';

export interface HostedSource {
    document: Source;
    revision: number;
    assembly: string;
    projectPath: string;
    fixture?: string | null;
    restoreAnimation?: HostedAnimationRestore;
}

export interface HostedFrame {
    revision: number;
    sequence: number;
    image: { png: string; width: number; height: number };
    fixture?: string | null;
    fixtures?: string[] | null;
    animations?: HostedAnimationCatalog;
    animationPresentation?: HostedAnimationPresentation;
}

export interface HostedAnimationSample {
    frame: HostedFrame;
    elapsed: number;
    completed: boolean;
}

export interface HostedFixtureCatalog {
    generation: number;
    revision: number;
    available: string[];
    selected: string | null;
    pending: boolean;
}

export type HostedInput = { revision: number; sequence: number; x: number; y: number } & (
    { type: 'click' } | { type: 'scroll'; delta: number } | { type: 'text'; text: string });

export type FromHosted = HostedInput | { type: 'ready' | 'refresh' }
    | { type: 'animation'; request: number; frame: HostedAnimationFrame; control: HostedAnimationControl }
    | { type: 'animationTargets' | 'animationSource'; frame: HostedAnimationFrame; program: number }
    | { type: 'chooseKeyframe'; frame: HostedAnimationFrame; program: number }
    | { type: 'curve'; frame: HostedAnimationFrame; request: number; query: HostedCurveQuery }
    | { type: 'keyframeDraft'; frame: HostedAnimationFrame; request: number; draft: HostedKeyframeDraft; apply: boolean }
    | { type: 'fixture'; revision: number; name: string | null };
export type ToHosted = { type: 'frame'; frame: HostedFrame }
    | { type: 'animations'; state: HostedAnimationState; version: number }
    | { type: 'animationReply'; request: number }
    | { type: 'openKeyframe'; key: HostedKeyframeSelection }
    | { type: 'curveReply'; request: number; frame: HostedAnimationFrame; points?: CurvePoint[]; error?: string }
    | { type: 'keyframeReply'; frame: HostedAnimationFrame; request: number; preview?: KeyframeDraftPreview; applied?: boolean; error?: string }
    | { type: 'status'; message: string; interactive: boolean; fixtures: HostedFixtureCatalog };
