import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { RuntimeClient } from '../src/runtimeClient';
import { localMcpEndpoint, type RuntimeMcpEndpoint } from '../src/runtimeMcp';

export async function withNativeApp(args: string[], verify: (client: RuntimeClient) => Promise<void>): Promise<void> {
    const executable = process.env.FERROUI_TEST_HOSTED_APP;
    if (!executable) return;
    const child = spawn(executable, [...args, '--mcp', '--mcp-input'], {
        stdio: ['ignore', 'ignore', 'pipe'], windowsHide: true,
        env: { ...process.env, FERROUI_EDITOR_RELOAD: '0', FERROUI_EDITOR_SOURCE: '1' },
    });
    let client: RuntimeClient | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const lines = createInterface({ input: child.stderr });
    try {
        const endpoint = await new Promise<RuntimeMcpEndpoint>((resolve, reject) => {
            let url: string | undefined;
            let startupError: string | undefined;
            timeout = setTimeout(() => reject(new Error(`The native fixture ${args[0]} did not publish its inspector endpoint`)), 10_000);
            lines.on('line', line => {
                const endpoint = line.match(/^FerroUI MCP: (\S+)$/)?.[1];
                if (endpoint && localMcpEndpoint(endpoint)) url = endpoint;
                const token = line.match(/^Authorization: Bearer ([a-fA-F0-9]{64})$/)?.[1];
                if (url && token) resolve({ url, token });
                if (line.startsWith('Could not open ')) startupError = line;
            });
            child.once('error', reject);
            child.once('close', code => reject(new Error(startupError ?? `The native fixture exited during startup (${code})`)));
        });
        client = new RuntimeClient(endpoint);
        await verify(client);
    } finally {
        clearTimeout(timeout);
        client?.dispose();
        lines.close();
        child.kill();
    }
}
