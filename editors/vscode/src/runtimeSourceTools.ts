import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections } from './runtimeConnections';
import type { RuntimeNode } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';

export class RuntimeSourceTools {
    constructor(private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor) {}

    async read(node: number, signal: AbortSignal): Promise<unknown> {
        return this.useManaged(signal, async (client, generation, current) => {
            const source = await this.source.read(client, node, signal);
            if (!current()) throw new Error('The managed application changed. Read its current runtime tree.');
            return { generation, source: source ? this.source.describe(source) : null };
        });
    }

    async apply(node: number, property: string, value: string, generation: number, version: number, signal: AbortSignal): Promise<unknown> {
        return this.useManaged(signal, async (client, currentGeneration, current) => {
            if (generation !== currentGeneration) throw new Error('Read the current managed application generation before editing.');
            const [target] = await client.call<RuntimeNode[]>('get_tree', { node_id: node, depth: 0 }, signal);
            if (target?.node_id !== node) throw new Error('Read the current managed application tree before selecting.');
            await this.source.apply(client, target, property, value, signal, current, version);
            return { applied: true, generation };
        });
    }

    private async useManaged<T>(signal: AbortSignal,
        action: (client: RuntimeClient, generation: number, current: () => boolean) => Promise<T>): Promise<T> {
        signal.throwIfAborted();
        const endpoint = this.runtimes.get('managed');
        if (!endpoint) throw new Error('Start a managed FerroUI application with MCP inspection enabled.');
        const generation = this.runtimes.managedRevision;
        const current = () => this.runtimes.managedRevision === generation && this.runtimes.get('managed') === endpoint;
        const client = new RuntimeClient(endpoint);
        try { return await action(client, generation, current); }
        finally { client.dispose(); }
    }
}
