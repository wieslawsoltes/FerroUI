import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';

interface RuntimeCall {
    runtime: AppRuntime;
    expected_generation: number;
    tool: string;
    arguments?: object;
}

/** Route an app's tools through the workspace host that owns its authenticated connection. */
export class RuntimeMcpActions {
    constructor(private readonly connections: RuntimeConnections) {}

    tools(runtime: AppRuntime, signal: AbortSignal) {
        return this.use(runtime, undefined, signal, async (client, generation, active) =>
            ({ runtime, generation, tools: await client.tools(active) }));
    }

    call(request: RuntimeCall, readOnly: boolean, signal: AbortSignal) {
        const { runtime, expected_generation, tool } = request;
        return this.use(runtime, expected_generation, signal, async (client, generation, active) => {
            const definition = (await client.tools(active)).find(entry => entry.name === tool);
            if (!definition) throw new Error('Choose a tool from ferroui_runtime_tools for this app.');
            if (readOnly && definition.annotations?.readOnlyHint !== true) {
                throw new Error('Use ferroui_runtime_call with editor and application write permissions for this tool.');
            }
            active.throwIfAborted();
            const response = await client.invoke(tool, request.arguments ?? {}, active);
            const image = response.content.find(item => item.type === 'image' && item.mimeType === 'image/png');
            const text = response.content.flatMap(item => item.type === 'text' && typeof item.text === 'string' ? [readText(item.text)] : []);
            const result = response.structuredContent ?? (text.length === 1 ? text[0] : text.length ? text : undefined);
            return { runtime, generation, result, ...(image ? { mimeType: image.mimeType, data: image.data } : {}) };
        });
    }

    private async use<T>(runtime: AppRuntime, expected: number | undefined, signal: AbortSignal,
        action: (client: RuntimeClient, generation: number, signal: AbortSignal) => Promise<T>): Promise<T> {
        signal.throwIfAborted();
        const session = this.connections.session(runtime);
        if (session.runtime !== runtime) throw new Error('Use an exact runtime ID from ferroui_app_sessions.');
        const retired = new Error('The application connection changed. Read its current MCP tools.');
        if (expected !== undefined && expected !== session.generation) throw retired;
        const cancellation = new AbortController();
        const active = AbortSignal.any([signal, cancellation.signal]);
        const subscription = this.connections.onDidChange(() => {
            if (!session.current()) cancellation.abort(retired);
        });
        const client = new RuntimeClient(session.endpoint);
        try {
            const result = await action(client, session.generation, active);
            active.throwIfAborted();
            if (!session.current()) throw retired;
            return result;
        } finally {
            subscription.dispose();
            client.dispose();
        }
    }
}

function readText(text: string): unknown {
    try { return JSON.parse(text); }
    catch { return text; }
}
