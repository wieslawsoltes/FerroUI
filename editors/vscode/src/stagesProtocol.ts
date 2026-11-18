import type { PreviewFinding, Range, Snapshot, Source } from './protocol';

export type StageImage = NonNullable<Snapshot['image']>;
export interface StageConfiguration {
    name: string;
    source_uri: string;
    target: { kind: string };
    environment: { width: number; height: number; theme: 'light' | 'dark' | 'inherit'; flow_direction: 'inherit' | 'left_to_right' | 'right_to_left' };
    layout: 'fixed' | 'size_that_fits' | 'fixed_width' | 'fixed_height';
    design_time_data: boolean;
    states: StageState[];
    pinned: boolean;
    auto_update: boolean;
    mode: 'select' | 'live';
}
export interface StageInfo {
    id: number;
    epoch: number;
    configuration: StageConfiguration;
    status: { state: string; message?: string };
    layout_size: { width: number; height: number };
    rendered_revision: number | null;
}
export interface StageManifest {
    version: number;
    stages: StageConfiguration[];
    active: number | null;
    prototype?: PrototypeGraph;
}
export type OverlayOptions = { placement: { kind: string; x?: number; y?: number }; dim_background: boolean; dismiss_on_outside: boolean };
export type PrototypeAction = { kind: 'navigate'; stage: number } | { kind: 'back' | 'close_overlay' }
    | { kind: 'open_overlay' | 'swap_overlay'; stage: number; options: OverlayOptions };
export interface PrototypeGraph {
    flows: Array<{ name: string; start: number }>;
    links: Array<{ name: string; source: number; trigger: unknown; action: PrototypeAction }>;
}
export interface PrototypeHotspot { stage: number; label: string; anchor: unknown }
export interface PrototypeScene {
    flow: string;
    screen: number;
    current: number;
    overlays: Array<{ stage: number; options: OverlayOptions }>;
    can_go_back: boolean;
}
export type PrototypeCommand = { action: 'start'; flow: string } | { action: 'activate'; link: number }
    | { action: 'back' | 'closeOverlay' | 'restart' | 'stop' };
export interface StageModel { epoch: number; manifest: StageManifest; stages: StageInfo[]; prototype: PrototypeScene | null }
export interface StageTarget { id: number; epoch: number }
export interface StageChecks { uri: string; findings: (PreviewFinding & { range: Range | null })[] }
export interface SourceAnchor { path: { namespace: string; type_name: string; name: string | null; key: string | null }[] }
export interface StageState { owner: SourceAnchor; group: string; state: string }
export interface StageStateOptions { owner: SourceAnchor; owner_name: string; group: string; states: string[] }
export interface StagePointer extends StageTarget { x: number; y: number }
export interface StagePick { range?: Range; uri?: string; interacted?: boolean; prototype?: PrototypeScene }
interface AddStage { action: 'add'; name: string; width: number; height: number; dark: boolean; epoch: number }
type ExistingStageChange = (
    { action: 'remove' | 'refresh'; id: number }
    | { action: 'update'; id: number; configuration: StageConfiguration }
    | { action: 'setPrototype'; graph: PrototypeGraph }) & { epoch: number };
export type StageChange = ExistingStageChange | (AddStage & { sourcePath?: string });
export type NativeStageChange = ExistingStageChange | (AddStage & { source?: Source });
export type FromStages = { type: 'ready' | 'add' | 'saveManifest' | 'loadManifest' }
    | ({ type: 'pointer' } & StagePointer) | ({ type: 'checks' } & StageTarget)
    | { type: 'edit'; epoch: number; id: number; field: 'name' | 'environment' | 'mode' | 'layout' | 'pause' | 'refresh' | 'remove' | 'export' | 'designData' | 'state' };
export type StagesRequest = FromStages | { type: 'prototype'; command?: PrototypeCommand } | { type: 'editPrototype' };
export type ToStages = { type: 'model'; model: StageModel; uri: string }
    | ({ type: 'image'; image: StageImage } & StageTarget) | { type: 'status'; message: string }
    | { type: 'prototype'; scene: PrototypeScene | null };
