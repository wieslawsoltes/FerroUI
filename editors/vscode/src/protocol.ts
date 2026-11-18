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
export interface DesignerViewState {
    version: 1;
    uri: string;
    caret: Position;
    viewport: Viewport;
}
export function isViewport(value: unknown): value is Viewport {
    if (!value || typeof value !== 'object') return false;
    const { width, height, dark } = value as Partial<Viewport>;
    const validSize = (size: unknown): boolean => typeof size === 'number' && Number.isInteger(size)
        && size >= MIN_VIEWPORT_SIZE && size <= MAX_VIEWPORT_SIZE;
    return validSize(width) && validSize(height) && typeof dark === 'boolean';
}
export interface Bounds { x: number; y: number; width: number; height: number }
export type RuntimeTreeMode = 'visual' | 'logical' | 'combined';
export type OutlineMode = 'source' | RuntimeTreeMode;
export interface RuntimeRow {
    inspector_id: number;
    source_id: number | null;
    parent: number | null;
    type: string;
    name: string | null;
    bounds: Bounds | null;
    visual: boolean;
    logical: boolean;
    logical_parent: number | null;
}
export interface Property { name: string; value: string; editable: boolean }
export interface PropertyField {
    name: string;
    group: string;
    kind: 'boolean' | 'number' | 'enum' | 'color' | 'thickness' | 'object' | 'text';
    options: string[];
    authored: string | null;
    effective: string | null;
    origin: string | null;
    range: Range | null;
    editable: boolean;
    canReset: boolean;
    mixed?: boolean;
}
export interface EventField {
    name: string;
    argumentType: string;
    handler: string | null;
    range: Range | null;
    editable: boolean;
}
export interface ResourceRow {
    id: number;
    name: string;
    typeName: string;
    scope: string;
    range: Range;
    usages: number;
    visible: boolean;
    properties: string[];
}
export type DetailsView = 'events' | 'resources';
export type DesignerView = 'canvas' | 'outline' | 'properties' | 'toolbox' | DetailsView | 'assets';
export interface DetailsState {
    frame?: FrameTarget;
    target?: NodeTarget;
    language?: Language;
    label?: string;
    events?: EventField[];
    resources?: ResourceRow[];
    message?: string;
}
export type FromDetails = { type: 'ready' }
    | { type: 'reveal'; frame: FrameTarget; range: Range }
    | { type: 'edit'; request: number; target: NodeTarget; operation: NodeOperation };
export type ToDetails = { type: 'state'; state: DetailsState }
    | { type: 'edit'; request: number; error?: string };
export type InspectionTarget = { type: 'source'; node: number } | { type: 'selection'; nodes: number[] } | { type: 'runtime'; id: number };
export interface Inspection { properties: PropertyField[]; events: EventField[] }
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
    | { type: 'resetProperty'; node: number; name: string }
    | { type: 'setEvent'; node: number; name: string; value: string | null }
    | { type: 'applyResource'; frame: number; node: number; resource: number; property: string; dynamic: boolean }
    | { type: 'removeNode'; node: number };
export type Operation = NodeOperation | { type: 'applyCounterpart'; source: string }
    | { type: 'apply'; label: string; edits: object[] }
    | { type: 'place'; frame: number; placement: Placement }
    | ({ type: 'resize'; frame: number } & ResizeGesture);
export interface ResizeGesture { node: number; handle: number; dx: number; dy: number }

// Node IDs and tokens are local to a designer session.
export interface FrameTarget { session: number; token: number }
export interface NodeTarget extends FrameTarget { node: number; selection?: number[] }
export interface PropertiesState {
    session?: number;
    target?: NodeTarget;
    node?: OutlineNode;
    message?: string;
    runtime?: RuntimeRow;
    frame?: FrameTarget;
    fields?: PropertyField[];
    selectionCount?: number;
}
export type FromProperties = { type: 'ready' }
    | { type: 'toolbox' }
    | { type: 'reveal'; frame: FrameTarget; range: Range }
    | { type: 'edit'; request: number; target: NodeTarget; operation: NodeOperation };
export type ToProperties = { type: 'state'; state: PropertiesState }
    | { type: 'edit'; request: number; error?: string };

export interface Asset { id: string; title: string; category: string; description: string; icon: string; rust: boolean }
export interface ProjectAsset {
    id: string;
    label: string;
    path: string;
    kind: string;
    relative: string;
    thumbnail: boolean;
    source?: string;
}
export interface AssetsState {
    frame?: FrameTarget;
    folder?: string;
    assets: ProjectAsset[];
    canInsert: boolean;
    message?: string;
}
export type FromAssets = { type: 'ready' } | DragMessage
    | { type: 'preview' | 'open' | 'copy'; id: string }
    | { type: 'insert'; frame: FrameTarget; id: string };
export type ToAssets = DragUpdate | { type: 'state'; state: AssetsState }
    | { type: 'thumbnail'; id: string; uri?: string }
    | { type: 'assetStatus'; message?: string; error?: boolean };
export interface Point { x: number; y: number }
export type DragOrigin = { type: 'asset'; control: string } | { type: 'node'; node: number; grab?: Point }
    | { type: 'nodes'; nodes: number[]; anchor: number; grab?: Point }
    | { type: 'image'; source: string; asset?: string };
export type OutlinePosition = 'before' | 'inside' | 'after';
export type DropTarget = { type: 'canvas' } & Point | { type: 'outline'; node: number; position: OutlinePosition };
export interface Placement { origin: DragOrigin; target: DropTarget }
export interface DropHint { bounds: Bounds | null; description: string }
export interface DragGesture { id: string; frame: FrameTarget; origin: DragOrigin }
export type DragMessage = { type: 'beginDrag'; gesture: DragGesture } | { type: 'endDrag'; id: string }
    | { type: 'drop'; id: string; frame: FrameTarget; target: DropTarget };
export type DragUpdate = { type: 'drag'; gesture?: DragGesture };
export type PlacementResult = { type: 'placement'; id: string; sequence: number; hint?: DropHint; error?: string };

export type OutlineRow = Omit<Pick<OutlineNode, 'id' | 'parent' | 'label' | 'name' | 'canMove' | 'canInsert'>, 'id' | 'parent'> & {
    id: string;
    parent: string | null;
    source?: number;
    runtime?: RuntimeRow;
};
export interface OutlineState {
    frame?: FrameTarget;
    session?: number;
    mode?: OutlineMode;
    nodes: OutlineRow[];
    selected?: string;
    selectedRows?: string[];
    message?: string;
}
export interface ToolboxState { frame?: FrameTarget; assets: Asset[]; canInsert: boolean; message?: string }
export type FromOutline = { type: 'ready' } | DragMessage
    | { type: 'mode'; mode: OutlineMode }
    | { type: 'layout'; frame: FrameTarget }
    | { type: 'select'; frame: FrameTarget; id: string; openSource: boolean; extend?: boolean }
    | { type: 'reorder'; frame: FrameTarget; origin: Extract<DragOrigin, { type: 'node' | 'nodes' }>; anchor: number; position: 'before' | 'after' };
export type ToOutline = { type: 'state'; state: OutlineState } | DragUpdate | { type: 'edit'; error?: string };
export type FromToolbox = { type: 'ready' } | DragMessage | { type: 'insert'; frame: FrameTarget; control: string };
export type ToToolbox = DragUpdate | { type: 'state'; state: ToolboxState } | { type: 'edit'; error?: string };

export type CanvasNode = Pick<OutlineNode, 'id' | 'label' | 'name' | 'bounds' | 'canMove'> & { selectable?: boolean };
export interface StudioPresentation {
    image: NonNullable<Snapshot['image']>;
    nodes: Array<{ id: number; bounds: Bounds | null; canInsert: boolean; canMove: boolean; selectable: boolean }>;
    notices: string[];
    selection: number[];
}
export type CanvasSnapshot = Omit<Snapshot, 'nodes'> & { nodes: CanvasNode[] };
export interface CanvasAction {
    tab?: 'preview' | 'counterpart';
    zoom?: 'fit' | 'fit_selection' | 'actual';
    scale?: number;
    pan_x?: number;
    pan_y?: number;
    pan_tool?: boolean;
}
export interface CanvasState {
    tab: 'preview' | 'counterpart';
    scale: number;
    fit: boolean;
    scrollX: number;
    scrollY: number;
    panTool: boolean;
}
export type FromDesigner = DragMessage
    | { type: 'ready' | 'refresh' }
    | { type: 'dock'; view: Exclude<DesignerView, 'canvas'> }
    | { type: 'previewDrop'; id: string; frame: FrameTarget; target: DropTarget; sequence: number }
    | { type: 'viewport'; viewport: Viewport }
    | { type: 'canvasResult'; request: number; state?: CanvasState; error?: string }
    | { type: 'block'; token: number; index: number }
    | { type: 'select'; token: number; id: number; openSource: boolean; extend?: boolean }
    | ({ type: 'resize'; token: number } & ResizeGesture)
    | { type: 'apply'; token: number; source: string };

export type ToDesigner = DragUpdate | PlacementResult
    | { type: 'viewState'; state: DesignerViewState }
    | { type: 'canvasAction'; request: number; action: CanvasAction }
    | { type: 'pending' }
    | { type: 'snapshot'; frame: FrameTarget; sourceKey: string; snapshot: CanvasSnapshot; viewport: Viewport; selected: number | undefined; selection?: number[]; runtime?: RuntimeRow }
    | { type: 'error'; message: string }
    | { type: 'selection'; token: number; selected: number | undefined; selection?: number[]; runtime?: RuntimeRow }
    | { type: 'edit'; error?: string };

export function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
}
