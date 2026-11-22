import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections, RuntimeTarget } from './runtimeConnections';
import type { RuntimeNode } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';
import { readRuntimeTemplates } from './runtimeTemplates';

export class RuntimeSourceTools {
    constructor(private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor) {}

    async read(node: number, signal: AbortSignal, target: RuntimeTarget = 'managed'): Promise<unknown> {
        return this.useRuntime(target, signal, async (client, generation, current) => {
            const source = await this.source.read(client, node, signal);
            if (!current()) throw new Error('The application connection changed. Read its current runtime tree.');
            return { generation, source: source ? this.source.describe(source, target) : null };
        });
    }

    async templates(node: number, signal: AbortSignal, target: RuntimeTarget = 'managed') {
        return this.useRuntime(target, signal, async (client, generation, current) => {
            const templates = await readRuntimeTemplates(client, node, signal);
            if (!current()) throw new Error('The application connection changed. Read its current runtime tree.');
            return { generation, templates };
        });
    }

    async templateSource(node: number, id: string, generation: number, signal: AbortSignal, target: RuntimeTarget = 'managed') {
        return this.useRuntime(target, signal, async (client, currentGeneration, current) => {
            if (generation !== currentGeneration) throw new Error('Read the current application generation before opening a template.');
            return this.source.revealTemplate(client, node, id, signal, current);
        });
    }

    async apply(node: number, property: string, value: string, generation: number, version: number, signal: AbortSignal,
        target: RuntimeTarget = 'managed'): Promise<unknown> {
        return this.useRuntime(target, signal, async (client, currentGeneration, current) => {
            if (generation !== currentGeneration) throw new Error('Read the current application generation before editing.');
            const [element] = await client.call<RuntimeNode[]>('get_tree', { node_id: node, depth: 0 }, signal);
            if (element?.node_id !== node) throw new Error('Read the current runtime tree before selecting.');
            await this.source.apply(client, element, property, value, signal, target, current, version);
            return { applied: true, generation };
        });
    }

    private async useRuntime<T>(target: RuntimeTarget, signal: AbortSignal,
        action: (client: RuntimeClient, generation: number, current: () => boolean) => Promise<T>): Promise<T> {
        signal.throwIfAborted();
        const endpoint = this.runtimes.get(target);
        if (!endpoint) throw new Error(target === 'managed'
            ? 'Start a managed FerroUI application with MCP inspection enabled.'
            : 'Attach an authenticated FerroUI application first.');
        const generation = this.runtimes.revision(target);
        const current = () => this.runtimes.revision(target) === generation && this.runtimes.get(target) === endpoint;
        const client = new RuntimeClient(endpoint);
        try { return await action(client, generation, current); }
        finally { client.dispose(); }
    }
}
