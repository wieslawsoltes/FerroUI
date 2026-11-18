import type { Source } from './protocol';

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
    | { type: 'fixture'; revision: number; name: string | null };
export type ToHosted = { type: 'frame'; frame: HostedFrame }
    | { type: 'status'; message: string; interactive: boolean; fixtures: HostedFixtureCatalog };
