import { choice, integer, text, tool, truth, type Schema } from './agentSchema';

const path = { type: 'string', maxLength: 4096 };
const location = { path, folder: path };
const revision = { type: 'string', maxLength: 128 };
const checked = { ...location, expected_revision: revision };
const record = (properties: Record<string, Schema>, required: string[]): Schema =>
    ({ type: 'object', properties, required, additionalProperties: false });
const at = record({ line: integer(0), character: integer(0) }, ['line', 'character']);
const range = record({ start: at, end: at }, ['start', 'end']);
const edit = record({ range, newText: text }, ['range', 'newText']);
const formatting = record({
    tabSize: integer(1, 16), insertSpaces: truth, trimTrailingWhitespace: truth,
    insertFinalNewline: truth, trimFinalNewlines: truth, preserveBlankLines: truth,
    attributeLayout: choice('preserve', 'inline', 'onePerLine', 'wrap'),
    wrapColumn: integer(20, 240), closingBracketOnNewLine: truth,
    selfClosingSpace: choice('preserve', 'space', 'none'),
}, []);

export const workspaceCatalog = {
    ferroui_workspace_files: tool('List local workspace roots and matching project files. Use the returned folder name or URI to resolve paths in a multi-root window.',
        { folder: path, glob: path, limit: integer(1, 200) }),
    ferroui_workspace_read: tool('Read the live VS Code text buffer, diagnostics, language, and opaque revision for any project file. A range uses zero-based UTF-16 positions.',
        { ...location, range, include_source: truth }, ['path']),
    ferroui_workspace_language: tool('Query the active VS Code language provider for Rust, XAML, or another workspace language. Returns document symbols, completion, hover, definitions, references, or a rename range. Except for symbols, pass a zero-based UTF-16 position.',
        { ...location, action: choice('symbols', 'completion', 'hover', 'definition', 'references', 'rename_range'),
            position: at, limit: integer(1, 200) }, ['path', 'action']),
    ferroui_workspace_create: tool('Create a new project text file with initial contents. Uses VS Code file history and fails if the destination already exists.',
        { ...location, source: text, open: truth }, ['path', 'source'], true, false),
    ferroui_workspace_edit: tool('Apply one undoable, atomic text edit across project buffers. Use each current revision and either replacement source or non-overlapping UTF-16 edits per file.',
        { files: { type: 'array', maxItems: 32, items: record({ ...checked, source: text,
            edits: { type: 'array', maxItems: 2048, items: edit } }, ['path', 'expected_revision']) } }, ['files'], true),
    ferroui_workspace_file: tool('Open, save, format, undo, redo, rename, or delete a project file. Changes require its current revision. Format accepts a UTF-16 range and optional FerroUI XAML formatting overrides. Rename uses new_path in the same workspace folder.',
        { ...checked, action: choice('open', 'save', 'format', 'undo', 'redo', 'rename', 'delete'),
            new_path: path, position: at, range, options: formatting },
        ['path', 'action'], true),
};

export const fileMoveCatalog = {
    ferroui_workspace_plan_xaml_move: tool('Review an XAML file move within its project. Returns the destination and exact edits for XAML, Rust, Cargo builds, saved stage sets, and native scenarios.',
        { ...location, destination: path }, ['path', 'destination']),
    ferroui_workspace_move_xaml: tool('Apply a reviewed XAML file move and its source changes in one VS Code Undo step. Copy review from ferroui_workspace_plan_xaml_move.',
        { ...location, destination: path, review: revision }, ['path', 'destination', 'review'], true),
};

const rename = { ...location, position: at, new_name: { type: 'string', maxLength: 256 } };
export const renameCatalog = {
    ferroui_workspace_plan_resource_rename: tool('Review a XAML resource key and its resolved project references. Pass a UTF-16 location on the declaration, element name, or resource operand. A matched theme family includes its authored layers. Returns exact patches and a review identifier.',
        rename, ['path', 'position', 'new_name']),
    ferroui_workspace_rename_resource: tool('Apply a checked XAML resource rename across the project in one Undo step. Copy review from ferroui_workspace_plan_resource_rename.',
        { ...rename, review: revision }, ['path', 'position', 'new_name', 'review'], true),
    ferroui_workspace_plan_property_rename: tool('Review the public name change of a Cargo-registered XAML property or typed binding member at a UTF-16 position. Returns the authored Rust registration, XAML patches and possible unmapped references.',
        rename, ['path', 'position', 'new_name']),
    ferroui_workspace_rename_property: tool('Apply the exact reviewed property rename as one edit across Rust and XAML. Copy review from ferroui_workspace_plan_property_rename after inspecting the patches and unmapped locations.',
        { ...rename, review: revision }, ['path', 'position', 'new_name', 'review'], true),
    ferroui_workspace_plan_type_rename: tool('Review a Cargo-registered XAML type at a UTF-16 position. Returns edits to the registration and mapped XAML type uses. For names derived from Rust, rust-analyzer supplies the authored Rust reference edits.',
        rename, ['path', 'position', 'new_name']),
    ferroui_workspace_rename_type: tool('Apply a reviewed XAML type rename across Rust and XAML in one Undo step. Use the review identifier from ferroui_workspace_plan_type_rename.',
        { ...rename, review: revision }, ['path', 'position', 'new_name', 'review'], true),
};

export type WorkspaceTool = keyof typeof workspaceCatalog;
export function isWorkspaceTool(name: string): name is WorkspaceTool {
    return Object.hasOwn(workspaceCatalog, name);
}
