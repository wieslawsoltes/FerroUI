import { choice, integer, object, tool, type Schema } from './agentSchema';
import { runtimeFilterLength, runtimeFilters, runtimeHierarchies, runtimeInspections, runtimeModes } from './runtimeProtocol';
import { runtimeArgument as runtime } from './runtimeTargets';

const request = { runtime, expected_generation: integer(0), tool: { type: 'string', maxLength: 128 }, arguments: object };
const identity = ['runtime', 'expected_generation', 'tool'];

function viewAction(action: string, properties: Record<string, Schema> = {}): Schema {
    return { type: 'object', properties: { action: choice(action), ...properties },
        required: ['action', ...Object.keys(properties)], additionalProperties: false };
}

export const runtimeViewChangeSchema: Schema = { oneOf: [
    viewAction('window', { window: integer(0) }),
    viewAction('hierarchy', { hierarchy: choice(...runtimeHierarchies) }),
    viewAction('select', { node: integer(0), view: choice(...runtimeInspections), frame: integer(1) }),
    viewAction('mode', { mode: choice(...runtimeModes) }),
    viewAction('filter', { field: choice(...runtimeFilters), query: { type: 'string', maxLength: runtimeFilterLength } }),
    viewAction('refresh'),
] };

export const runtimeMcpCatalog = {
    ferroui_runtime_tools: tool('List the authenticated app or Studio MCP tools, including their input schemas and permissions. Use an exact runtime ID from ferroui_app_sessions.', { runtime }, ['runtime']),
    ferroui_runtime_query: tool('Run a read-only tool from ferroui_runtime_tools on that same runtime generation. Returns its data or native PNG.', request, identity),
    ferroui_runtime_call: tool('Run a tool from ferroui_runtime_tools on that same runtime generation. Mutations still require the application’s own permissions.', request, identity, true),
    ferroui_live_app_view: tool('Read a pinned Live App editor’s windows, hierarchy, filters, selection, mode, generation, revision and accepted frame. Open it with ferroui_runtime_open.', { runtime }, ['runtime']),
    ferroui_live_app_control: tool('Change the pinned Live App editor’s window, hierarchy, selection, mode or filters, or reconnect and refresh it. Use the generation and revision from ferroui_live_app_view.', {
        runtime, expected_generation: integer(1), expected_revision: integer(1), change: runtimeViewChangeSchema,
    }, ['runtime', 'expected_generation', 'expected_revision', 'change'], true, false),
};
