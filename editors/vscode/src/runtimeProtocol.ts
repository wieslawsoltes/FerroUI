import type { Bounds } from './protocol';
import type { KeyboardFocus, KeyboardInput } from './previewKeyboardProtocol';
import type { AppRuntime } from './runtimeTargets';

export interface RuntimeNode {
    node_id: number;
    type: string;
    name?: string;
    text?: string;
    role?: string;
    visible?: boolean;
    bounds?: Bounds;
    bounds_in_root?: Bounds;
    children?: RuntimeNode[];
}

export interface RuntimeProperty {
    name: string;
    type: string;
    value: unknown;
    read_only: boolean;
    priority: string;
    binding: boolean;
}

export interface RuntimeSource {
    uri?: string;
    workspacePath?: string;
    manifestPath?: string;
    line: number;
    column: number;
    language?: 'rust';
    columnEncoding?: 'unicodeScalar';
    invocation?: { line: number; column: number; index: number };
    revision?: string;
}

export const runtimeHierarchies = ['visual', 'logical', 'automation'] as const;
export const runtimeInspections = ['properties', 'styles', 'resources', 'diagnostics'] as const;
export const runtimeModes = ['inspect', 'interact'] as const;
export const runtimeFilters = ['treeFilter', 'propertyFilter'] as const;
export const runtimeFilterLength = 256;
export type RuntimeHierarchy = typeof runtimeHierarchies[number];
export type RuntimeInspection = typeof runtimeInspections[number];
export type RuntimeMode = typeof runtimeModes[number];
export type RuntimeFilter = typeof runtimeFilters[number];

export interface RuntimeTreeDisclosure {
    collapsed: readonly number[];
    search?: { query: string; collapsed: readonly number[] };
}

export interface RuntimePresentation {
    hierarchy: RuntimeHierarchy;
    inspection: RuntimeInspection;
    mode: RuntimeMode;
    treeFilter: string;
    propertyFilter: string;
    disclosure: RuntimeTreeDisclosure;
}

export interface RuntimeSelection {
    node: number;
    view: RuntimeInspection;
}

export interface RuntimeInspectionResult extends RuntimeSelection {
    data: unknown;
    canReveal: boolean;
    canApply: boolean;
    canTrace: boolean;
    canInspectTemplates: boolean;
}

export type RuntimeViewChange =
    | { action: 'window'; window: number }
    | { action: 'hierarchy'; hierarchy: RuntimeHierarchy }
    | { action: 'select'; node: number; view: RuntimeInspection; frame: number }
    | { action: 'disclosure'; node: number; expanded: boolean; frame: number }
    | { action: 'mode'; mode: RuntimeMode }
    | { action: 'filter'; field: RuntimeFilter; query: string }
    | { action: 'refresh' };

export interface RuntimeViewSnapshot extends RuntimePresentation {
    runtime: AppRuntime;
    generation: number;
    revision: number;
    frame: number | null;
    debuggerPaused: boolean;
    keyboardInput: boolean;
    window: number | null;
    windows: RuntimeNode[];
    selection: RuntimeSelection | null;
}

export interface RuntimeFrame {
    token: number;
    root: RuntimeNode;
    tree: RuntimeNode[];
    png: string;
    keyboardScope?: number;
}

export interface RuntimeKeyboardRequest {
    type: 'keyboard';
    scope: number;
    request: number;
    input: KeyboardInput;
}

export type RuntimeKeyboardResult = { type: 'keyboardResult'; scope: number; request: number }
    & ({ focus: KeyboardFocus | null } | { error: string });

export type ToRuntime =
    | { type: 'state'; target?: AppRuntime; targets: { runtime: AppRuntime; label: string }[];
        generation?: number; status: string; windows: RuntimeNode[]; root?: number; debuggerPaused: boolean }
    | { type: 'presentation'; presentation: Partial<RuntimePresentation> }
    | { type: 'invalidateFrame' | 'clearSelection' | 'releaseKeyboard' }
    | { type: 'frame'; frame: RuntimeFrame }
    | { type: 'inspection'; token: number; revision: number; inspection: RuntimeInspectionResult }
    | RuntimeKeyboardResult
    | { type: 'problem'; message: string };

type RuntimeFrameAction =
    | { type: 'pointer'; x: number; y: number; interact: boolean; overlap?: boolean }
    | { type: 'reveal'; node: number }
    | { type: 'templates'; node: number }
    | { type: 'trace'; node: number; property: string }
    | { type: 'text'; node: number; text: string }
    | { type: 'key'; node: number; key: string }
    | { type: 'scroll'; node: number; x: number; y: number }
    | { type: 'assign' | 'assignSource'; node: number; property: string; value: string };

export type FromRuntime =
    | { type: 'ready' | 'start' | 'attach' }
    | { type: 'target'; target: AppRuntime }
    | { type: 'presentation'; generation: number; change: RuntimeViewChange }
    | RuntimeKeyboardRequest
    | (RuntimeFrameAction & { token: number });
