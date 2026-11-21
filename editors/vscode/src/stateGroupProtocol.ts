import type { Range } from './protocol';

export interface StateSetter { property: string; value: string; target: string }
export interface VisualState { name: string; setters: StateSetter[] }
export type StateEasing = { kind: 'Linear' } | { kind: 'Named'; value: string }
    | { kind: 'CubicBezier'; value: { x1: number; y1: number; x2: number; y2: number } }
    | { kind: 'Spring'; value: { mass: number; stiffness: number; damping: number; initial_velocity: number } };
export interface StateTransition { from: string | null; to: string; duration: number; easing: StateEasing }
export interface VisualStateGroup { owner: number; name: string; states: VisualState[]; transitions: StateTransition[] }
export interface AuthoredStateGroup { model: VisualStateGroup; sourceRange: Range }
export type StateGroupEdit = { action: 'upsert'; model: VisualStateGroup }
    | { action: 'remove'; owner: number; group: string };
