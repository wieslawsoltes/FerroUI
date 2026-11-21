import type { Bounds } from './protocol';
import type { RuntimeTarget } from './runtimeConnections';

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
    uri: string;
    line: number;
    column: number;
    language?: 'rust';
    columnEncoding?: 'unicodeScalar';
    invocation?: { line: number; column: number; index: number };
    revision?: string;
}

export type RuntimeHierarchy = 'visual' | 'logical' | 'automation';
export type RuntimeInspection = 'properties' | 'styles' | 'resources' | 'diagnostics';

export interface RuntimeFrame {
    token: number;
    root: RuntimeNode;
    tree: RuntimeNode[];
    png: string;
}

export type ToRuntime =
    | { type: 'state'; target?: RuntimeTarget; targets: RuntimeTarget[]; status: string; windows: RuntimeNode[]; root?: number }
    | { type: 'frame'; frame: RuntimeFrame }
    | { type: 'inspection'; token: number; view: RuntimeInspection; node: number; data: unknown; canReveal: boolean; canApply: boolean; canTrace: boolean }
    | { type: 'problem'; message: string };

type RuntimeFrameAction =
    | { type: 'select'; node: number; view: RuntimeInspection }
    | { type: 'pointer'; x: number; y: number; interact: boolean; overlap?: boolean }
    | { type: 'reveal'; node: number }
    | { type: 'trace'; node: number; property: string }
    | { type: 'text'; node: number; text: string }
    | { type: 'key'; node: number; key: string }
    | { type: 'scroll'; node: number; x: number; y: number }
    | { type: 'assign' | 'assignSource'; node: number; property: string; value: string };

export type FromRuntime =
    | { type: 'ready' | 'refresh' | 'start' | 'attach' }
    | { type: 'target'; target: RuntimeTarget }
    | { type: 'window'; id: number }
    | { type: 'hierarchy'; tree: RuntimeHierarchy }
    | (RuntimeFrameAction & { token: number });
