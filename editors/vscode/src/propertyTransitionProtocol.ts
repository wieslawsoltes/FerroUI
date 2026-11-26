import type { Range } from './protocol';
import type { StateEasing } from './stateGroupProtocol';

export interface TransitionEntry {
    id: number;
    typeName: string;
    property: string | null;
    durationSeconds: number | null;
    delaySeconds: number | null;
    easing: string | null;
    editableTiming: string[];
    canRetarget?: boolean;
    range: Range;
}

export interface TransitionProperty {
    property: string;
    label: string;
    typeName: string;
}

export interface TransitionList {
    owner: number;
    canEdit: boolean;
    canEditCollection?: boolean;
    canAdd?: boolean;
    requiresTarget?: boolean;
    properties?: TransitionProperty[];
    range: Range | null;
    entries: TransitionEntry[];
}

export interface TransitionTiming {
    duration?: number;
    delay?: number;
    easing?: StateEasing;
}

export type TransitionCommand =
    | { action: 'timing'; transition: number; timing: TransitionTiming }
    | { action: 'add'; property: string; type_name?: string; timing?: TransitionTiming }
    | { action: 'retarget'; transition: number; property: string; type_name?: string }
    | { action: 'move'; transition: number; index: number }
    | { action: 'remove'; transition: number };

export interface TransitionOwner {
    owner: number;
    target_type?: string;
}

export type TransitionEdit = TransitionOwner & { command: TransitionCommand };
