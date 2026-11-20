import type { Position } from './protocol';
import type { ReviewedDocument } from './reviewedSources';

export interface FileMoveDraft {
    uri: string;
    version: number;
    source: string;
}

export type FileMoveDocument = ReviewedDocument;

export interface FileMovePlan {
    from: string;
    to: string;
    review: string;
    documents: FileMoveDocument[];
    generatedRust?: GeneratedRustRename[];
}

export interface GeneratedRustRename {
    uri: string;
    position: Position;
    oldName: string;
    newName: string;
}

export interface FileMoveRustResolution {
    rename: GeneratedRustRename;
    documents: FileMoveDocument['edit'][];
}
