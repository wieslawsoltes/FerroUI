import { choice, integer, truth, type Schema } from './agentSchema';

const time = { type: 'number', minimum: 0 };
export const previewRangeSchema: Schema = { type: 'object', additionalProperties: false, required: ['start', 'end', 'loop'],
    properties: { start: time, end: time, loop: truth } };

const subject = { scope: choice('program', 'clips'), program: integer(1) };
export const timelineWorkAreaSchema: Schema = { oneOf: [
    { type: 'object', additionalProperties: false, required: ['scope', 'action', 'range'],
        properties: { ...subject, action: choice('set'), range: previewRangeSchema } },
    { type: 'object', additionalProperties: false, required: ['scope', 'action'],
        properties: { ...subject, action: choice('enable', 'disable', 'set_in', 'set_out') } },
] };

export const timelineSnapSchema: Schema = { type: 'object', additionalProperties: false, required: ['program', 'frame_rate'],
    properties: { program: integer(1), frame_rate: integer(0) } };
