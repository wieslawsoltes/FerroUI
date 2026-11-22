import { canvasThemeLabels, type PreviewFinding, type PreviewSourceBuffer, type Range, type Snapshot, type Source } from './protocol';

export const stageThemeLabels = { ...canvasThemeLabels, inherit: 'Application theme' } as const;
export type StageTheme = keyof typeof stageThemeLabels;
export const stageDirectionLabels = { inherit: 'Inherited direction', left_to_right: 'Left to right', right_to_left: 'Right to left' } as const;
export interface StageTextSettings { locale?: string; scale_percent?: number }
export const stagePointerLabels = { automatic: 'Current device', touch: 'Touch', pen: 'Pen' } as const;
export type StagePointerProfile = keyof typeof stagePointerLabels;
export type StagePointerDevice = 'mouse' | 'touch' | 'pen';

export type StageImage = NonNullable<Snapshot['image']>;
export interface StageConfiguration {
    name: string;
    source_uri: string;
    target: { kind: string };
    environment: { width: number; height: number; theme: StageTheme; flow_direction: keyof typeof stageDirectionLabels };
    text?: StageTextSettings;
    pointer_input?: StagePointerProfile;
    layout: 'fixed' | 'size_that_fits' | 'fixed_width' | 'fixed_height';
    design_time_data: boolean;
    design_fixture?: string;
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
export interface PrototypeHotspot { stage: number; label: string; anchor: unknown; uri?: string; range?: Range | null }
export type PrototypeEdit = { kind: 'flow'; index?: number }
    | { kind: 'interaction'; index?: number; source?: number; destination?: number };
export interface PrototypeScene {
    flow: string;
    screen: number;
    current: number;
    overlays: Array<{ stage: number; options: OverlayOptions }>;
    can_go_back: boolean;
    frame?: number;
    links?: number[];
    diagnostics?: string[];
}
export type PrototypeCommand = { action: 'start'; flow: string } | { action: 'activate'; link: number }
    | { action: 'back' | 'closeOverlay' | 'restart' | 'stop' };
export interface StageModel { epoch: number; manifest: StageManifest; stages: StageInfo[]; prototype: PrototypeScene | null; dependencies?: string[]; keyboardInput?: boolean; pointerInput?: boolean; availableThemes?: StageTheme[]; textEnvironment?: boolean }
export interface StageTarget { id: number; epoch: number }
export interface StageAccessibleNode {
    runtime_id: number;
    depth: number;
    control: string;
    name: string;
    role: string;
    control_element: boolean;
    enabled: boolean;
    offscreen: boolean;
    range: Range | null;
}
export interface StageChecks {
    uri: string;
    findings: (PreviewFinding & { range: Range | null })[];
    accessibility?: { tree: StageAccessibleNode[]; tab_order: StageAccessibleNode[]; truncated: boolean };
}
export interface SourceAnchor { path: { namespace: string; type_name: string; name: string | null; key: string | null }[] }
export interface StageState { owner: SourceAnchor; group: string; state: string }
export interface StageStateOptions { owner: SourceAnchor; owner_name: string; group: string; states: string[] }
export interface StagePointer extends StageTarget { x: number; y: number }
export interface StageContact {
    contact: number;
    phase: 'down' | 'move' | 'up' | 'cancel';
    device: StagePointerDevice;
    x: number;
    y: number;
    pressure?: number;
    tiltX?: number;
    tiltY?: number;
    twist?: number;
}
export interface StagePointerInput extends StageTarget { input: StageContact }
export type StageInput = { kind: 'focus' } | { kind: 'key'; key: string } | { kind: 'text'; text: string };
export interface StageKeyboard extends StageTarget { input: StageInput }
export interface StageFocus { name: string; role: string }
export type PrototypeInput = { kind: 'keyboard'; input: StageInput } | { kind: 'pointer'; input: StageContact };
export interface PrototypeInputResult { scene: PrototypeScene | null; focus: StageFocus | null }
export interface StagePick { range?: Range; uri?: string; interacted?: boolean; prototype?: PrototypeScene }
interface AddStage { action: 'add'; name: string; width: number; height: number; dark: boolean; epoch: number }
type ExistingStageChange = (
    { action: 'remove' | 'refresh'; id: number }
    | { action: 'update'; id: number; configuration: StageConfiguration }
    | { action: 'setPrototype'; graph: PrototypeGraph }) & { epoch: number };
export type StageChange = ExistingStageChange | (AddStage & { sourcePath?: string });
export type StageDocument = Source & { workspaceRoot?: string };
export type NativeStageChange = ExistingStageChange | (AddStage & { source?: StageDocument; localSources?: PreviewSourceBuffer[] });
export type FromStages = { type: 'ready' | 'add' | 'saveManifest' | 'loadManifest' }
    | ({ type: 'pointer' } & StagePointer) | ({ type: 'checks' } & StageTarget)
    | ({ type: 'keyboard' } & StageKeyboard)
    | ({ type: 'contact' } & StagePointerInput)
    | { type: 'edit'; epoch: number; id: number; field: 'name' | 'environment' | 'mode' | 'layout' | 'pause' | 'refresh' | 'remove' | 'export' | 'designData' | 'state' };
export type StagesRequest = FromStages | { type: 'prototype'; command?: PrototypeCommand; frame?: number }
    | { type: 'prototypeInput'; frame: number; input: PrototypeInput }
    | { type: 'editPrototype'; epoch: number; edit: PrototypeEdit }
    | { type: 'revealTrigger'; epoch: number; index: number };
export type ToStages = { type: 'model'; model: StageModel; uri: string }
    | ({ type: 'image'; image: StageImage } & StageTarget) | { type: 'status'; message: string }
    | ({ type: 'keyboardFocus'; focus: StageFocus | null } & StageTarget)
    | { type: 'inputUnavailable' }
    | { type: 'prototype'; scene: PrototypeScene | null }
    | { type: 'prototypeImage'; frame: number; image: StageImage }
    | { type: 'prototypeFocus'; frame: number; focus: StageFocus | null };
