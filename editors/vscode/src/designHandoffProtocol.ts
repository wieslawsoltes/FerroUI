import type { DragOrigin, PreviewSourceBuffer, Source } from './protocol';
import type { ProjectDefinition } from './projectLibrary';

export type HandoffQuery = { action: 'review' } | { action: 'check' | 'copy'; id: string; review: string };

export interface HandoffFile { uri: string; selected: boolean }

export interface HandoffInput {
    document: Source;
    workspaceRoot: string;
    manifest: string;
    selectedFile: boolean;
    revision: string;
    drafts: PreviewSourceBuffer[];
}

export interface HandoffRow {
    id: string;
    name: string;
    kind: string;
    target: string;
    reference: string | null;
    review: string | null;
    issue: string | null;
    insertion: Extract<DragOrigin, { type: 'asset' | 'image' }> | null;
    definition: ProjectDefinition | null;
    assetUri: string | null;
    copy: { direction: 'import' | 'export'; source: string; destination: string; sourceLabel: string; destinationLabel: string; bytes: number } | null;
}

export interface HandoffPlan { manifest: string; document: string; rows: HandoffRow[] }

export interface HandoffState {
    type: 'handoff';
    manifest: string;
    document: string;
    revision: number;
    busy: boolean;
    current: boolean;
    rows?: HandoffRow[];
    status: string;
    error: boolean;
}
