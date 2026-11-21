import { choice, integer, tool, truth, type Schema } from './agentSchema';
import { contrastProviderLabels, paletteLayerLabels } from './stylesProtocol';

const node = integer(1);
const session = { session: integer(1) };
const generation = { ...session, expected_generation: integer(1) };
const frame = { ...generation, expected_revision: integer(1), expected_sequence: integer(1) };
const address = { type: 'string', maxLength: 4096 };
const target = record({ address, owner: node });
const position = record({ line: integer(0), character: integer(0) });
const layer = choice(...Object.keys(paletteLayerLabels));
const edit: Schema = {
    ...record({ action: choice('set', 'add'), node, owner: node,
        layer, key: { type: 'string', maxLength: 512 },
        kind: choice('color', 'solid_color_brush'), color: { type: 'string', maxLength: 32 },
        provider: choice(...Object.keys(contrastProviderLabels)) }),
    required: ['action', 'color'],
};
const editArgs = { ...frame, arguments: record({ target, edit }) };

export const hostedPaletteCatalog = {
    designer_hosted_palette: tool('Read palette source files and scopes from the accepted App Preview. Supply a target in arguments to read its palette and resolved theme colors.',
        { ...session, arguments: record({ target }) }, ['session']),
    designer_hosted_palette_edit_draft: tool('Return checked source patches for changing a color or adding a declaration. Use the generation, frame, and target from designer_hosted_palette.',
        editArgs, Object.keys(editArgs)),
    designer_hosted_palette_edit: tool('Apply a checked application-preview palette edit to its owning XAML buffer in one Undo step.',
        editArgs, Object.keys(editArgs), true, false),
    designer_hosted_palette_source: tool('Reveal an exact source range from the accepted application-preview palette. Use the source file address from the palette catalog.',
        { ...frame, arguments: record({ address, range: record({ start: position, end: position }) }) },
        [...Object.keys(frame), 'arguments'], true, false),
    designer_hosted_palette_references: tool('Read references to a declared color in the selected palette source.',
        { ...frame, arguments: record({ target, node }) }, [...Object.keys(frame), 'arguments']),
    designer_hosted_palette_show: tool('Open the XAML App Preview palette. Optionally choose a source and scope, a layer, or the theme comparison.',
        { ...generation, arguments: { ...record({ target, layer, compare: truth }), required: [] } }, Object.keys(generation), true, false),
};

function record(properties: Record<string, Schema>): Schema {
    return { type: 'object', properties, required: Object.keys(properties), additionalProperties: false };
}
