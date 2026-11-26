import { choice, type Schema } from './agentSchema';

export type KeyboardInput = { kind: 'focus' } | { kind: 'key'; key: string } | { kind: 'text'; text: string };
export interface KeyboardFocus { name: string; role: string }

export interface PreviewKeyboardRequest {
    type: 'keyboard';
    scope: number;
    request: number;
    input: KeyboardInput;
}

export type PreviewKeyboardResult = { type: 'keyboardResult'; scope: number; request: number }
    & ({ focus: KeyboardFocus | null } | { error: string });

export const keyboardInputSchema: Schema = { oneOf: [
    { type: 'object', additionalProperties: false, required: ['kind'], properties: { kind: choice('focus') } },
    { type: 'object', additionalProperties: false, required: ['kind', 'key'],
        properties: { kind: choice('key'), key: { type: 'string', maxLength: 64 } } },
    { type: 'object', additionalProperties: false, required: ['kind', 'text'],
        properties: { kind: choice('text'), text: { type: 'string', maxLength: 8192 } } },
] };
