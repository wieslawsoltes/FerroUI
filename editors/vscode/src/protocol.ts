export interface Position { line: number; character: number }
export interface Range { start: Position; end: Position }
export interface TextEdit { range: Range; newText: string }
export type Language = 'xaml' | 'rust';

export interface Source {
    language: Language;
    uri: string;
    source: string;
    caret: Position;
}
export interface Viewport { width: number; height: number; dark: boolean }
export interface Bounds { x: number; y: number; width: number; height: number }
export interface Property { name: string; value: string; editable: boolean }
export interface OutlineNode {
    id: number;
    parent: number | null;
    label: string;
    name: string | null;
    range: Range;
    nameRange: Range;
    bounds: Bounds | null;
    properties: Property[];
    addableProperties: string[];
    canInsert: boolean;
    canRemove: boolean;
}
export interface Snapshot {
    nodes: OutlineNode[];
    blocks: Range[];
    activeBlock: Range | null;
    counterpart: { source: string | null; editable: boolean; reason: string | null };
    image: { png: string; width: number; height: number } | null;
    renderError: string | null;
    notices: string[];
}
export type Operation =
    | { type: 'setProperty'; node: number; name: string; value: string }
    | { type: 'insertChild'; node: number; control: string }
    | { type: 'removeNode'; node: number }
    | { type: 'applyCounterpart'; source: string };

export type FromDesigner =
    | { type: 'ready' | 'refresh' }
    | { type: 'viewport'; viewport: Viewport }
    | { type: 'block'; token: number; index: number }
    | { type: 'select'; token: number; id: number }
    | { type: 'edit'; token: number; operation: Operation };

export type ToDesigner =
    | { type: 'pending' }
    | { type: 'snapshot'; token: number; sourceKey: string; snapshot: Snapshot; selected: number | undefined }
    | { type: 'error'; message: string }
    | { type: 'selection'; token: number; selected: number | undefined }
    | { type: 'edit'; error?: string };

export function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}
