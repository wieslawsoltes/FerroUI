export interface Position { line: number; character: number }
export interface Range { start: Position; end: Position }
export interface TextEdit { range: Range; newText: string }
export interface EditResult { edits: TextEdit[]; selection: Range | null }
export type Language = 'xaml' | 'rust';

export interface Source {
    language: Language;
    uri: string;
    source: string;
    caret: Position;
}
export interface Viewport { width: number; height: number; dark: boolean }
export const DEFAULT_VIEWPORT: Viewport = { width: 800, height: 520, dark: false };
export const MIN_VIEWPORT_SIZE = 64;
export const MAX_VIEWPORT_SIZE = 1536;
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
    canMove: boolean;
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
export type NodeOperation =
    | { type: 'setProperty'; node: number; name: string; value: string }
    | { type: 'removeNode'; node: number };
export type Operation = NodeOperation | { type: 'applyCounterpart'; source: string }
    | { type: 'apply'; label: string; edits: object[] }
    | { type: 'place'; frame: number; placement: Placement };

// Node IDs and tokens are local to a designer session.
export interface FrameTarget { session: number; token: number }
export interface NodeTarget extends FrameTarget { node: number }
export interface PropertiesState {
    session?: number;
    target?: NodeTarget;
    node?: OutlineNode;
    message?: string;
}
export type FromProperties = { type: 'ready' }
    | { type: 'toolbox' }
    | { type: 'edit'; request: number; target: NodeTarget; operation: NodeOperation };
export type ToProperties = { type: 'state'; state: PropertiesState }
    | { type: 'edit'; request: number; error?: string };

export interface Asset { id: string; title: string; category: string; description: string; icon: string; rust: boolean }
export interface Point { x: number; y: number }
export type DragOrigin = { type: 'asset'; control: string } | { type: 'node'; node: number; grab?: Point };
export type OutlinePosition = 'before' | 'inside' | 'after';
export type DropTarget = { type: 'canvas' } & Point | { type: 'outline'; node: number; position: OutlinePosition };
export interface Placement { origin: DragOrigin; target: DropTarget }
export interface DropHint { bounds: Bounds | null; description: string }
export interface DragGesture { id: string; frame: FrameTarget; origin: DragOrigin }
export type DragMessage = { type: 'beginDrag'; gesture: DragGesture } | { type: 'endDrag'; id: string }
    | { type: 'drop'; id: string; frame: FrameTarget; target: DropTarget };
export type DragUpdate = { type: 'drag'; gesture?: DragGesture };
export type PlacementResult = { type: 'placement'; id: string; sequence: number; hint?: DropHint; error?: string };

export type OutlineRow = Pick<OutlineNode, 'id' | 'parent' | 'label' | 'name' | 'canMove' | 'canInsert'>;
export interface OutlineState { frame?: FrameTarget; session?: number; nodes: OutlineRow[]; selected?: number; message?: string }
export interface ToolboxState { frame?: FrameTarget; assets: Asset[]; canInsert: boolean; message?: string }
export type FromOutline = { type: 'ready' } | DragMessage
    | { type: 'select'; target: NodeTarget; openSource: boolean }
    | { type: 'reorder'; frame: FrameTarget; node: number; anchor: number; position: 'before' | 'after' };
export type ToOutline = { type: 'state'; state: OutlineState } | DragUpdate | { type: 'edit'; error?: string };
export type FromToolbox = { type: 'ready' } | DragMessage | { type: 'insert'; frame: FrameTarget; control: string };
export type ToToolbox = DragUpdate | { type: 'state'; state: ToolboxState } | { type: 'edit'; error?: string };

export type CanvasNode = Pick<OutlineNode, 'id' | 'label' | 'name' | 'bounds' | 'canMove'>;
export type CanvasSnapshot = Omit<Snapshot, 'nodes'> & { nodes: CanvasNode[] };
export type FromDesigner = DragMessage
    | { type: 'ready' | 'refresh' }
    | { type: 'dock'; view: 'outline' | 'properties' | 'toolbox' }
    | { type: 'previewDrop'; id: string; frame: FrameTarget; target: DropTarget; sequence: number }
    | { type: 'viewport'; viewport: Viewport }
    | { type: 'block'; token: number; index: number }
    | { type: 'select'; token: number; id: number; openSource: boolean }
    | { type: 'apply'; token: number; source: string };

export type ToDesigner = DragUpdate | PlacementResult
    | { type: 'pending' }
    | { type: 'snapshot'; frame: FrameTarget; sourceKey: string; snapshot: CanvasSnapshot; viewport: Viewport; selected: number | undefined }
    | { type: 'error'; message: string }
    | { type: 'selection'; token: number; selected: number | undefined }
    | { type: 'edit'; error?: string };

export function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}
