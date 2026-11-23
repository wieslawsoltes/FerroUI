import type { PreviewSourceBuffer, Range, Source } from './protocol';
import type { HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationPresentation, HostedAnimationRestore,
    HostedAnimationState, HostedClipDraft, HostedCurveQuery, HostedKeyframeDraft, HostedKeyframeGroupAction,
    HostedKeyframeSample, HostedKeyframeSelection } from './hostedAnimationProtocol';
import type { CurvePoint, KeyframeDraftPreview, KeyframeLocation } from './timelineProtocol';
import type { FromHostedPalette, ToHostedPalette } from './hostedPaletteProtocol';
import type { FromHostedStyles, ToHostedStyles } from './hostedStylesProtocol';
import type { RustModuleProject } from './cargoTarget';

export interface HostedSource {
    document: Source;
    revision: number;
    assembly: string;
    projectPath: string;
    fixture?: string | null;
    restoreAnimation?: HostedAnimationRestore;
    selectClipKey?: string;
}

export interface HostedFrame {
    revision: number;
    sequence: number;
    image: { png: string; width: number; height: number };
    fixture?: string | null;
    fixtures?: string[] | null;
    paletteAvailable?: boolean;
    stylesAvailable?: boolean;
    templatesAvailable?: boolean;
    rustScope?: { active: Range; blocks: Range[] };
    animations?: HostedAnimationCatalog;
    animationPresentation?: HostedAnimationPresentation;
}

export interface HostedWorkspaceSources {
    root: string;
    localSources: PreviewSourceBuffer[];
    rustProject?: RustModuleProject;
}

export interface HostedWorkspaceFrame {
    frame: HostedFrame | null;
    error: string | null;
    dependencies: string[] | null;
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

type HostedPoint = { x: number; y: number };
export type HostedGesture = (HostedPoint & ({ type: 'click' } | { type: 'scroll'; delta: number }))
    | { type: 'text'; text: string };

export type HostedInput = HostedAnimationFrame & HostedPoint & HostedGesture;

export type FromHosted = HostedInput | FromHostedPalette | FromHostedStyles | { type: 'ready' | 'refresh' }
    | { type: 'templates'; frame: HostedAnimationFrame }
    | { type: 'animation'; request: number; frame: HostedAnimationFrame; control: HostedAnimationControl }
    | { type: 'clipPlacement'; request: number; frame: HostedAnimationFrame; draft: HostedClipDraft }
    | { type: 'clipActions'; request: number; frame: HostedAnimationFrame; program?: number }
    | { type: 'stateActions'; request: number; frame: HostedAnimationFrame; program?: number }
    | { type: 'propertyTransitionActions'; request: number; frame: HostedAnimationFrame }
    | { type: 'animationTargets' | 'animationSource'; frame: HostedAnimationFrame; program: number }
    | { type: 'animationIssues'; frame: HostedAnimationFrame }
    | { type: 'chooseKeyframe' | 'editStateRoute'; frame: HostedAnimationFrame; program: number }
    | { type: 'curve'; frame: HostedAnimationFrame; request: number; query: HostedCurveQuery }
    | { type: 'keyframeDraft'; frame: HostedAnimationFrame; request: number; draft: HostedKeyframeDraft; apply: boolean }
    | { type: 'recordKeyframe'; frame: HostedAnimationFrame; request: number; draft: HostedKeyframeSample }
    | { type: 'keyframeGroup'; frame: HostedAnimationFrame; request: number; action: HostedKeyframeGroupAction }
    | { type: 'fixture'; revision: number; name: string | null };
export type ToHosted = ToHostedPalette | ToHostedStyles | { type: 'frame'; frame: HostedFrame }
    | { type: 'animations'; state: HostedAnimationState; version: number }
    | { type: 'animationReply'; request: number }
    | { type: 'openKeyframe'; key: HostedKeyframeSelection }
    | { type: 'curveReply'; request: number; frame: HostedAnimationFrame; points?: CurvePoint[]; error?: string }
    | { type: 'keyframeReply'; frame: HostedAnimationFrame; request: number; preview?: KeyframeDraftPreview; applied?: boolean; error?: string }
    | { type: 'keyframeLaneReply'; frame: HostedAnimationFrame; request: number; keys?: KeyframeLocation[]; version?: number; error?: string }
    | { type: 'status'; message: string; interactive: boolean; fixtures: HostedFixtureCatalog };
