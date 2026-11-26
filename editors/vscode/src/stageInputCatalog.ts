import { choice, integer, type Schema } from './agentSchema';
import { keyboardInputSchema } from './previewKeyboardProtocol';

export const stageContactInput: Schema = { type: 'object', additionalProperties: false,
    required: ['contact', 'phase', 'device', 'x', 'y'], properties: {
        contact: integer(0, 4294967295), phase: choice('down', 'move', 'up', 'cancel'), device: choice('mouse', 'touch', 'pen'),
        x: { type: 'number', minimum: -1024, maximum: 1024 }, y: { type: 'number', minimum: -1024, maximum: 1024 },
        pressure: { type: 'number', minimum: 0, maximum: 1 },
        tiltX: { type: 'number', minimum: -90, maximum: 90 }, tiltY: { type: 'number', minimum: -90, maximum: 90 },
        twist: { type: 'number', minimum: 0, exclusiveMaximum: 360 },
    } };

export const prototypeInput: Schema = { oneOf: [
    { type: 'object', additionalProperties: false, required: ['kind', 'input'],
        properties: { kind: choice('keyboard'), input: keyboardInputSchema } },
    { type: 'object', additionalProperties: false, required: ['kind', 'input'],
        properties: { kind: choice('pointer'), input: stageContactInput } },
] };
