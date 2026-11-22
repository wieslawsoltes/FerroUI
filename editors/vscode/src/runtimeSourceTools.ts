import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections, RuntimeTarget } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';
import type { RuntimeNode } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';
import { readRuntimeTemplates } from './runtimeTemplates';

export class RuntimeSourceTools {
    constructor(private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor) {}

    async read(node: number, signal: AbortSignal, target: RuntimeTarget = 'managed'): Promise<unknown> {
        return this.useRuntime(target, signal, async (client, generation, current, runtime) => {
            const source = await this.source.read(client, node, signal);
            if (!current()) throw new Error('The application connection changed. Read its current runtime tree.');
            return { runtime, generation, source: source ? this.source.describe(source, runtime) : null };
        });
    }

    async templates(node: number, signal: AbortSignal, target: RuntimeTarget = 'managed') {
        return this.useRuntime(target, signal, async (client, generation, current, runtime) => {
            const templates = await readRuntimeTemplates(client, node, signal);
            if (!current()) throw new Error('The application connection changed. Read its current runtime tree.');
            return { runtime, generation, templates };
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
        return this.useRuntime(target, signal, async (client, currentGeneration, current, runtime) => {
            if (generation !== currentGeneration) throw new Error('Read the current application generation before editing.');
            const [element] = await client.call<RuntimeNode[]>('get_tree', { node_id: node, depth: 0 }, signal);
            if (element?.node_id !== node) throw new Error('Read the current runtime tree before selecting.');
            await this.source.apply(client, element, property, value, signal, runtime, current, version);
            return { applied: true, runtime, generation };
        });
    }

    private async useRuntime<T>(target: RuntimeTarget, signal: AbortSignal,
        action: (client: RuntimeClient, generation: number, current: () => boolean, runtime: AppRuntime) => Promise<T>): Promise<T> {
        signal.throwIfAborted();
        const { endpoint, generation, current, runtime } = this.runtimes.session(target);
        const client = new RuntimeClient(endpoint);
        try { return await action(client, generation, current, runtime); }
        finally { client.dispose(); }
    }
}
