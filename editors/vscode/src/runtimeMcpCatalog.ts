import { integer, object, tool } from './agentSchema';
import { runtimeArgument as runtime } from './runtimeTargets';

const request = { runtime, expected_generation: integer(0), tool: { type: 'string', maxLength: 128 }, arguments: object };
const identity = ['runtime', 'expected_generation', 'tool'];

export const runtimeMcpCatalog = {
    ferroui_runtime_tools: tool('List the authenticated app or Studio MCP tools, including their input schemas and permissions. Use an exact runtime ID from ferroui_app_sessions.', { runtime }, ['runtime']),
    ferroui_runtime_query: tool('Run a read-only tool from ferroui_runtime_tools on that same runtime generation. Returns its data or native PNG.', request, identity),
    ferroui_runtime_call: tool('Run a tool from ferroui_runtime_tools on that same runtime generation. Mutations still require the application’s own permissions.', request, identity, true),
};
