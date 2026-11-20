import type { Position, TextEdit } from './protocol';

export interface FileMoveDraft {
    uri: string;
    version: number;
    source: string;
}

export interface FileMoveDocument {
    edit: { uri: string; version: number; edits: TextEdit[] };
    before: string;
    after: string;
}

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
