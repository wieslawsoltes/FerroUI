import { choice, object, truth, type Schema } from './agentSchema';
import { stageDirectionLabels, stageThemeLabels } from './stagesProtocol';

const dimension = { type: 'number', minimum: 1, maximum: 16384 };
const shortName = { type: 'string', maxLength: 64 };
const stateName = { type: 'string', maxLength: 256 };
const size = { type: 'object', additionalProperties: false, required: ['name', 'width', 'height'],
    properties: { name: shortName, width: dimension, height: dimension } };
const stateGroup = { type: 'object', additionalProperties: false, required: ['owner', 'group', 'states'],
    properties: { owner: object, group: stateName, states: { type: 'array', maxItems: 16, items: stateName } } };
const matrix = { type: 'object', additionalProperties: false, properties: {
    sizes: { type: 'array', maxItems: 16, items: size },
    themes: { type: 'array', maxItems: 4, items: choice(...Object.keys(stageThemeLabels)) },
    directions: { type: 'array', maxItems: 3, items: choice(...Object.keys(stageDirectionLabels)) },
    design_time_data: { type: 'array', maxItems: 2, items: truth },
    state_groups: { type: 'array', maxItems: 32, items: stateGroup },
} };

export const stageComparisonSchema: Schema = { oneOf: [
    { type: 'object', additionalProperties: false, required: ['kind', 'preset'],
        properties: { kind: choice('preset'), preset: choice('sizes', 'themes', 'orientations', 'directions') } },
    { type: 'object', additionalProperties: false, required: ['kind', 'matrix'],
        properties: { kind: choice('matrix'), matrix } },
] };
