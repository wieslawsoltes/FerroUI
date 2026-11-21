import { choice, integer, tool, truth, type Schema } from './agentSchema';

const session = { session: integer(1) };
const generation = { ...session, expected_generation: integer(1) };
const frame = { ...generation, expected_revision: integer(1), expected_sequence: integer(1) };
const address = { type: 'string', maxLength: 4096 };
const target = record({ address, node: integer(0) });
const position = record({ line: integer(0), character: integer(0) });
const member = { type: 'string', maxLength: 512 };
const rustEdit = record({ action: choice('create', 'create_nested', 'selector', 'setter', 'add_setter', 'remove',
    'set_resource', 'add_color_resource', 'color_resource'), owner: integer(0), style: integer(0), setter: integer(0),
    resource: integer(0), key: member, selector: address, getter: member, value: {},
    layer: choice('shared', 'default', 'light', 'dark', 'high_contrast') }, ['action']);
const edit = record({ action: choice('create', 'create_theme', 'selector', 'base', 'setter', 'resource', 'remove', 'rust'),
    selector: address, target_type: member, key: {}, property: member, value: {}, dynamic: truth, edit: rustEdit }, ['action']);
const editArgs = { ...frame, arguments: record({ target, edit }) };

export const hostedStylesCatalog = {
    designer_hosted_styles: tool('Read the accepted app preview sources, one file’s style catalog, or an authored style definition. Use files, catalog with address, or read with target in arguments.',
        { ...session, arguments: record({ action: choice('files', 'catalog', 'read'), address, target }, ['action']) }, ['session']),
    designer_hosted_style_edit_draft: tool('Return the checked source patch for a style, setter, resource, or control-theme edit. Use the target and frame from designer_hosted_styles.',
        editArgs, Object.keys(editArgs)),
    designer_hosted_style_edit: tool('Apply a checked app preview style or theme edit to its owning XAML or mapped Rust buffer in one Undo step.',
        editArgs, Object.keys(editArgs), true, false),
    designer_hosted_style_resources: tool('Read resources compatible with a setter in the accepted style and theme scopes.',
        { ...frame, arguments: record({ target, property: member }) }, [...Object.keys(frame), 'arguments']),
    designer_hosted_style_source: tool('Reveal the reported style or setter range in the accepted App Preview source.',
        { ...frame, arguments: record({ address, range: record({ start: position, end: position }) }) }, [...Object.keys(frame), 'arguments'], true, false),
    designer_hosted_style_show: tool('Open the App Preview Styles & Themes editor. Supply a target in arguments to choose a definition and compare to show its Rust theme colors.',
        { ...generation, arguments: record({ target, compare: truth }, []) }, Object.keys(generation), true, false),
};

function record(properties: Record<string, Schema>, required = Object.keys(properties)): Schema {
    return { type: 'object', properties, required, additionalProperties: false };
}
