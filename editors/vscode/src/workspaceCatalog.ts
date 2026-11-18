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

export const workspaceCatalog = {
    ferroui_workspace_files: tool('List local workspace roots and matching project files. Use the returned folder name or URI to resolve paths in a multi-root window.',
        { folder: path, glob: path, limit: integer(1, 200) }),
    ferroui_workspace_read: tool('Read the live VS Code text buffer, diagnostics, language, and opaque revision for any project file. A range uses zero-based UTF-16 positions.',
        { ...location, range, include_source: truth }, ['path']),
    ferroui_workspace_create: tool('Create a new project text file with initial contents. Uses VS Code file history and fails if the destination already exists.',
        { ...location, source: text, open: truth }, ['path', 'source'], true, false),
    ferroui_workspace_edit: tool('Apply one undoable, atomic text edit across project buffers. Use each current revision and either replacement source or non-overlapping UTF-16 edits per file.',
        { files: { type: 'array', maxItems: 32, items: record({ ...checked, source: text,
            edits: { type: 'array', maxItems: 2048, items: edit } }, ['path', 'expected_revision']) } }, ['files'], true),
    ferroui_workspace_file: tool('Open, save, format, undo, redo, rename, or delete a project file. Changes require its current revision. Rename uses new_path in the same workspace folder.',
        { ...checked, action: choice('open', 'save', 'format', 'undo', 'redo', 'rename', 'delete'), new_path: path, position: at },
        ['path', 'action'], true),
};

export type WorkspaceTool = keyof typeof workspaceCatalog;
export function isWorkspaceTool(name: string): name is WorkspaceTool {
    return Object.hasOwn(workspaceCatalog, name);
}
