import type { Source } from './protocol';
import type { HostedAnimationCatalog, HostedAnimationControl, HostedAnimationFrame, HostedAnimationState } from './hostedAnimationProtocol';

export interface HostedSource {
    document: Source;
    revision: number;
    assembly: string;
    projectPath: string;
    fixture?: string | null;
}

export interface HostedFrame {
    revision: number;
    sequence: number;
    image: { png: string; width: number; height: number };
    fixture?: string | null;
    fixtures?: string[] | null;
    animations?: HostedAnimationCatalog;
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
    | { type: 'fixture'; revision: number; name: string | null };
export type ToHosted = { type: 'frame'; frame: HostedFrame }
    | { type: 'animations'; state: HostedAnimationState }
    | { type: 'animationReply'; request: number }
    | { type: 'status'; message: string; interactive: boolean; fixtures: HostedFixtureCatalog };
