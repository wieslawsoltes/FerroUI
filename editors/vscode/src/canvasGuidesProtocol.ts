import type { FrameTarget } from './protocol';

export type GuideAxis = 'Horizontal' | 'Vertical';
export interface GuideDraft { id?: number; axis: GuideAxis; position: number }
export interface Guide extends GuideDraft { id: number }
export interface GuidePreferences { rulers_visible: boolean; guides_visible: boolean; guides: Guide[] }
export interface GuideSettings extends GuidePreferences {
    grid_visible: boolean;
    grid_spacing: number;
    snap_to_grid: boolean;
    snap_to_guides: boolean;
    snap_to_objects: boolean;
    snap_tolerance: number;
    small_nudge: number;
    large_nudge: number;
}
export interface RulerSpec {
    thickness: number;
    majorPixels: number;
    factors: number[];
    subdivisions: number;
    maxTicks: number;
}
export interface GuideCatalog {
    defaults: GuideSettings;
    ruler: RulerSpec;
    maxGuides: number;
    maxPosition: number;
}
export type GuideEdit =
    | { action: 'visibility'; rulers?: boolean; guides?: boolean }
    | { action: 'add'; axis: GuideAxis; position: number }
    | { action: 'move' | 'duplicate'; id: number; position: number }
    | { action: 'remove'; id: number }
    | { action: 'clear' }
    | { action: 'apply'; rulers: boolean; guides: boolean; lines: GuideDraft[] };
export interface GuideUpdate { settings: GuideSettings; selected: number | null }
export type GuideScope = 'document' | 'definition' | 'rsx';
export interface CanvasGuidesState {
    frame: FrameTarget;
    scope: GuideScope;
    epoch: number;
    settings: GuideSettings;
    ruler: RulerSpec;
    maxGuides: number;
    maxPosition: number;
}
