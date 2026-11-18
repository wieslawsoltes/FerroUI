import type { Source } from './protocol';

export interface HostedSource {
    document: Source;
    revision: number;
    assembly: string;
    projectPath: string;
}

export interface HostedFrame {
    revision: number;
    sequence: number;
    image: { png: string; width: number; height: number };
}

export type HostedInput = { revision: number; sequence: number; x: number; y: number } & (
    { type: 'click' } | { type: 'scroll'; delta: number } | { type: 'text'; text: string });

export type FromHosted = HostedInput | { type: 'ready' | 'refresh' };
export type ToHosted = { type: 'frame'; frame: HostedFrame } | { type: 'status'; message: string; interactive: boolean };
