import type { FrameTarget } from './protocol';

export interface DesignFixtureCatalog {
    node: number;
    manifest: { default: string; fixtures: { name: string; data: Record<string, unknown> }[] } | null;
    selected: string | null;
    enabled: boolean;
}

export interface DesignDataState {
    session?: number;
    version?: number;
    frame?: FrameTarget;
    catalog?: DesignFixtureCatalog;
    message?: string;
}

export type FromDesignData = { type: 'ready' }
    | { type: 'preview'; frame: FrameTarget; choice: 'application' | 'default' | 'named'; name?: string }
    | { type: 'apply'; frame: FrameTarget; version: number; source: string };
export type ToDesignData = { type: 'error'; message: string }
    | { type: 'applied'; session: number; version: number; resultVersion: number };
