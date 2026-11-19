import type { Range } from './protocol';

export interface StateTransitionRoute {
    owner: number;
    group: string;
    from: string | null;
    to: string;
}

export interface NativeTransitionPosition {
    time: number;
    duration: number;
    sourceRange?: Range;
}

export interface StateTransitionPosition extends NativeTransitionPosition {
    route: StateTransitionRoute;
    playing: boolean;
    busy: boolean;
    revision: number;
}

export type StateTransitionControl =
    | ({ action: 'open'; time?: number; from?: string | null } & Omit<StateTransitionRoute, 'from'>)
    | { action: 'seek'; time: number }
    | { action: 'play' | 'pause' | 'stop' | 'reveal' };
