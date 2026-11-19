import type { FrameTarget, Point } from './protocol';

export interface SpacingHandle {
    id: string;
    owner: number;
    ownerLabel: string;
    property: string;
    label: string;
    position: Point;
    axis: Point;
    value: number;
    minimum: number;
    maximum: number | null;
    disabledReason: string | null;
}
export interface SpacingCatalog {
    model: number;
    revision: number;
    handles: SpacingHandle[];
    bands: { points: Point[]; filled: boolean; padding: boolean }[];
}
export interface SpacingChange {
    model: number;
    handle: string;
    adjustment: { delta: Point } | { value: number };
}
export interface SpacingDraft {
    handle: SpacingHandle;
    description: string;
    changed: boolean;
}
export interface SpacingRequest {
    type: 'spacing';
    frame: FrameTarget;
    selection: number[];
    request: number;
    operation: { kind: 'measure' } | ({ kind: 'draft' | 'commit' | 'exact' } & SpacingChange);
}
export interface SpacingResponse {
    type: 'spacingResult';
    request: number;
    catalog?: SpacingCatalog;
    draft?: SpacingDraft;
    error?: string;
}
