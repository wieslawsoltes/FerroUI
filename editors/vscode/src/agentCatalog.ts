type Schema = {
    type?: string;
    enum?: readonly string[];
    minimum?: number;
    maximum?: number;
    maxLength?: number;
    maxItems?: number;
    items?: Schema;
    properties?: Record<string, Schema>;
    required?: readonly string[];
    additionalProperties?: boolean;
};

const text = { type: 'string', maxLength: 1024 * 1024 };
const object = { type: 'object' };
const truth = { type: 'boolean' };
const integer = (minimum: number, maximum = Number.MAX_SAFE_INTEGER) => ({ type: 'integer', minimum, maximum });
const node = integer(1);
const session = { session: integer(1) };
const frame = { ...session, expected_revision: integer(0) };
const buffer = { ...session, expected_version: integer(1) };
const choice = (...values: string[]) => ({ type: 'string', enum: values });

function tool(description: string, properties: Record<string, Schema> = {}, required: string[] = [], write = false) {
    return {
        description,
        inputSchema: { type: 'object', properties, required, additionalProperties: false },
        annotations: { readOnlyHint: !write, destructiveHint: write, openWorldHint: false },
    };
}

export const agentCatalog = {
    ferroui_designs: tool('List open FerroUI designer sessions and the active session in this VS Code window.'),
    designer_open: tool('Open a workspace XAML or Rust file and its design tab. For Rust, optionally specify a zero-based invocation position.',
        { path: text, line: integer(0), character: integer(0) }, ['path'], true),
    designer_document: tool('Read the live VS Code buffer version, preview revision, selection, diagnostics, and optionally unsaved source and its XAML/RSX projection.',
        { ...session, include_source: truth, include_counterpart: truth }, ['session']),
    designer_outline: tool('Read mapped source controls, parent IDs, ranges, and native canvas bounds in document order. Optionally include the drop and property state of one node.',
        { ...session, node, offset: integer(0, 2000), limit: integer(1, 200) }, ['session']),
    designer_toolbox: tool('List Designer Studio control assets available to this XAML or Rust design.', session, ['session']),
    designer_preview_image: tool('Capture the current native artboard as PNG with viewport and source revision metadata.', session, ['session']),
    designer_view: tool('Show a designer view, refresh its preview, or set its logical width, height and appearance.',
        { ...session, view: choice('canvas', 'outline', 'properties', 'toolbox'), width: integer(64, 1536), height: integer(64, 1536), dark: truth, refresh: truth }, ['session'], true),
    designer_select: tool('Select one source node in the canvas, Outline and Properties. Open its source editor when requested.',
        { ...frame, node, reveal_source: truth }, ['session', 'expected_revision', 'node'], true),
    designer_place: tool('Insert a Toolbox asset or move a control using Designer Studio placement. Use an outline anchor and inside/before/after, or canvas coordinates at the native viewport scale.',
        { ...frame, asset: text, node, anchor: node, position: choice('inside', 'before', 'after'), x: { type: 'number' }, y: { type: 'number' } },
        ['session', 'expected_revision'], true),
    designer_apply: tool('Apply one Studio-compatible source transaction to the unsaved editor buffer. Edits use Rust enum JSON, such as {"SetAttribute":{"node":12,"name":"Width","value":"180"}}. Also supports RemoveAttribute, InsertChild, RemoveNode, and MoveNode. Read the current revision first.',
        { ...frame, label: { type: 'string', maxLength: 256 }, edits: { type: 'array', maxItems: 4096, items: object } }, ['session', 'expected_revision', 'label', 'edits'], true),
    designer_apply_counterpart: tool('Apply an edited XAML or RSX projection to the current buffer using its current designer revision.',
        { ...frame, source: text }, ['session', 'expected_revision', 'source'], true),
    designer_source: tool('Replace the complete authored buffer, including a draft with syntax errors, using its exact VS Code buffer version.',
        { ...buffer, source: text }, ['session', 'expected_version', 'source'], true),
    designer_history: tool('Undo or redo one transaction in the source editor at the given buffer version.',
        { ...buffer, action: choice('undo', 'redo') }, ['session', 'expected_version', 'action'], true),
    designer_save: tool('Save the given version of the source buffer. A running hot reload session will observe the save.',
        buffer, ['session', 'expected_version'], true),
    designer_studio_tools: tool('List the original read-only Designer Studio tool schemas that can run against the native preview of an editor session.', session, ['session']),
    designer_studio_query: tool('Run an original read-only Studio tool against the current native session. Use designer_studio_tools for names and argument schemas. Rust sessions expose their projected XAML in these queries.',
        { ...session, tool: text, arguments: object }, ['session', 'tool', 'arguments']),
    ferroui_reload_status: tool('Read the active managed runtime and last reported reload state.'),
    ferroui_reload_control: tool('Open the FerroUI Cargo target chooser, rebuild the managed application, or stop it.',
        { action: choice('start', 'rebuild', 'stop') }, ['action'], true),
};

export type AgentTool = keyof typeof agentCatalog;

export function validate(schema: Schema, value: unknown): void {
    if (schema.enum && !schema.enum.includes(value as string)) throw new Error('Invalid choice');
    if (schema.type === 'number' || schema.type === 'integer') {
        const n = value as number;
        if (!Number.isFinite(value) || (schema.type === 'integer' && !Number.isSafeInteger(n))
            || n < (schema.minimum ?? -Infinity) || n > (schema.maximum ?? Infinity)) throw new Error('Number is outside the allowed range');
    } else if (schema.type === 'string') {
        if (typeof value !== 'string' || value.length > (schema.maxLength ?? Infinity)) throw new Error('Expected a valid string');
    } else if (schema.type === 'boolean') {
        if (typeof value !== 'boolean') throw new Error('Expected a boolean');
    } else if (schema.type === 'array') {
        if (!Array.isArray(value) || value.length > (schema.maxItems ?? Infinity)) throw new Error('Expected a bounded array');
        if (schema.items) for (const item of value) validate(schema.items, item);
    } else if (schema.type === 'object') {
        if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Expected an arguments object');
        const arguments_ = value as Record<string, unknown>;
        for (const key of schema.required ?? []) if (!Object.hasOwn(arguments_, key)) throw new Error(`Missing ${key}`);
        for (const key of Object.keys(arguments_)) {
            const child = schema.properties && Object.hasOwn(schema.properties, key) ? schema.properties[key] : undefined;
            if (child) validate(child, arguments_[key]);
            else if (schema.additionalProperties === false) throw new Error(`Unknown argument ${key}`);
        }
    }
}
