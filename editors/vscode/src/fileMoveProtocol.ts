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
    publishedAssets?: { from: string; to: string }[];
}

export interface RustSymbolRename {
    uri: string;
    position: Position;
    oldName: string;
    newName: string;
}

export type GeneratedRustRename = RustSymbolRename;

export interface RustRenameResolution {
    rename: RustSymbolRename;
    documents: FileMoveDocument['edit'][];
}

export type FileMoveRustResolution = RustRenameResolution;
