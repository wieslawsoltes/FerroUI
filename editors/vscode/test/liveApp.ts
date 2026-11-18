import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { RuntimeClient } from '../src/runtimeClient';
import { mcpEndpointPrefix, parseMcpEndpoint, type RuntimeMcpEndpoint } from '../src/runtimeMcp';
import type { RuntimeNode } from '../src/runtimeProtocol';

export async function verifyLiveApp(): Promise<void> {
    const app = process.env.FERROUI_TEST_HOSTED_APP;
    if (!app) return;
    const child = spawn(app, ['--demo', '--mcp', '--mcp-input'], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true,
        env: { ...process.env, FERROUI_EDITOR_RELOAD: '1' } });
    let client: RuntimeClient | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const lines = createInterface({ input: child.stderr });
    try {
        const endpoint = await new Promise<RuntimeMcpEndpoint>((resolve, reject) => {
            timeout = setTimeout(() => reject(new Error('The native Todo app did not publish its inspector endpoint')), 10_000);
            lines.on('line', text => {
                if (!text.startsWith(mcpEndpointPrefix)) return;
                const endpoint = parseMcpEndpoint(text.slice(mcpEndpointPrefix.length));
                if (endpoint) resolve(endpoint);
            });
            child.once('error', reject);
            child.once('exit', code => reject(new Error(`The native Todo app exited during startup (${code})`)));
        });
        client = new RuntimeClient(endpoint);
        assert.ok(await client.supports('get_element_at'));
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'NewTask' })).matched);
        const [window] = await client.call<RuntimeNode[]>('list_windows');
        const [entry] = await client.call<RuntimeNode[]>('find_elements', { name: 'NewTask' });
        const [add] = await client.call<RuntimeNode[]>('find_elements', { name: 'AddTask' });
        const png = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.ok(entry && add && png.length);
        await client.call('type_text', { node_id: entry.node_id, text: 'QA task from VS Code' });
        await client.call('click', { node_id: add.node_id });
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { text: 'QA task from VS Code' })).matched);
        const updated = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.notEqual(updated, png);
        console.log('Live App used the real runtime inspector to find controls, enter text, click a bound action and capture the updated native window');
    } finally {
        clearTimeout(timeout);
        client?.dispose();
        lines.close();
        child.kill();
    }
}
