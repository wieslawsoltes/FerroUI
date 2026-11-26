import type { Position, Range, TextEdit } from './protocol';

export interface HandlerRequest {
    version: number;
    position: Position;
    event?: string | null;
}

export interface HandlerTarget {
    rustUri: string;
    existingName: string | null;
    suggestedName: string;
    event: string;
    declaringType: string;
    parameterTypes: string[];
}

export interface HandlerLocation { uri: string; range: Range }

export interface HandlerPlan {
    handler: string;
    declaration: HandlerLocation;
    edit: { document_changes: { uri: string; version: number; edits: TextEdit[] }[] };
}

export interface HandlerDraft { uri: string; version: number; source: string }

export type HandlerMethod = 'eventHandlerTarget' | 'planEventHandler' | 'eventHandlerDefinition';
