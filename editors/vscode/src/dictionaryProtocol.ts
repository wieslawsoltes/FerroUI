import type { Position, Range, TextEdit } from './protocol';

export type DictionaryMethod = 'dictionaryExtractionTarget' | 'planDictionaryExtraction';

export interface DictionaryRequest { version: number; position: Position }

export interface DictionaryTarget { range: Range; entries: number; suggestedFile: string }

export interface DictionaryPlan {
    review: string;
    target: DictionaryTarget;
    edit: { uri: string; version: number; edits: TextEdit[] };
    file: { uri: string; fileName: string; source: string };
}
