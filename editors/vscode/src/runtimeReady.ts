import { setTimeout as delay } from 'node:timers/promises';
import { RuntimeClient, type RuntimeEndpoint } from './runtimeClient';
import type { RuntimeNode } from './runtimeProtocol';

/** Authenticate and accept a native frame before publishing a device session. */
export async function verifyRuntime(endpoint: RuntimeEndpoint, signal: AbortSignal): Promise<void> {
    const deadline = AbortSignal.any([signal, AbortSignal.timeout(10_000)]);
    const client = new RuntimeClient(endpoint);
    const abort = () => client.dispose();
    deadline.addEventListener('abort', abort, { once: true });
    try {
        deadline.throwIfAborted();
        const tools = await client.tools(deadline);
        if (['list_windows', 'get_tree', 'screenshot'].some(name => !tools.some(tool => tool.name === name))) {
            throw new Error('The app inspector needs native window frames and visual inspection for Live App.');
        }
        for (;;) {
            const windows = await client.call<RuntimeNode[]>('list_windows', {}, deadline);
            const ready = windows.find(window => window.bounds && window.bounds.width > 0 && window.bounds.height > 0);
            if (ready) {
                const frame = await client.call<string>('screenshot', { node_id: ready.node_id }, deadline);
                if (typeof frame !== 'string' || !frame.startsWith('iVBORw0KGgo')) throw new Error('The app inspector did not return a native PNG.');
                return;
            }
            await delay(150, undefined, { signal: deadline });
        }
    } catch (error) {
        signal.throwIfAborted();
        if (deadline.aborted) throw new Error('The app did not present an inspectable window before the connection deadline.');
        throw error;
    } finally {
        deadline.removeEventListener('abort', abort);
        client.dispose();
    }
}
