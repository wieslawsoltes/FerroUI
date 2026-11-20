import type { TextEdit } from './protocol';

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
}
