import { choice, integer, text, truth, type Schema } from './agentSchema';

const target = integer(1);
const track = { target, property: text };

function action(name: string, properties: Record<string, Schema>): Schema {
    return { type: 'object', additionalProperties: false, required: ['action', 'program', ...Object.keys(properties)],
        properties: { action: choice(name), program: integer(1), ...properties } };
}

const channel = action('channel', { ...track, channel: text });

export const timelineTrackAction: Schema = { oneOf: [
    action('filter', { query: text, selectedOnly: truth }),
    action('collapse', { target, collapsed: truth }),
    action('lock', { target, locked: truth }),
    channel,
] };

export const hostedTimelineTrackAction: Schema = { oneOf: [action('select', track), channel] };
