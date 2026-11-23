import { strict as assert } from 'node:assert';
import { createServer } from 'node:http';
import { once } from 'node:events';
import type { AddressInfo } from 'node:net';
import { RuntimeClient } from '../src/runtimeClient';
import { mcpEndpointPrefix, runtimeEndpointReader } from '../src/runtimeMcp';

export async function verifyRuntimeClient(): Promise<void> {
    verifyRuntimeAnnouncements();
    const token = 'a'.repeat(64);
    const session = 'live-test';
    const methods: string[] = [];
    let ended!: () => void;
    const deleted = new Promise<void>(resolve => { ended = resolve; });
    const server = createServer(async (request, response) => {
        if (request.headers.authorization !== `Bearer ${token}`) {
            response.writeHead(401).end();
            return;
        }
        if (request.method === 'DELETE') {
            response.writeHead(request.headers['mcp-session-id'] === session ? 204 : 400).end();
            ended();
            return;
        }
        const chunks: Buffer[] = [];
        for await (const chunk of request) chunks.push(chunk);
        const body = JSON.parse(Buffer.concat(chunks).toString());
        methods.push(body.method);
        if (body.method === 'initialize') {
            response.setHeader('Mcp-Session-Id', session);
            response.end(JSON.stringify({ jsonrpc: '2.0', id: body.id, result: { protocolVersion: '2025-11-25' } }));
            return;
        }
        if (request.headers['mcp-session-id'] !== session || request.headers['mcp-protocol-version'] !== '2025-11-25') {
            response.writeHead(400).end();
            return;
        }
        if (body.method === 'notifications/initialized') {
            response.writeHead(202).end();
            return;
        }
        const reply = body.params.name === 'screenshot'
            ? { type: 'image', mimeType: 'image/png', data: 'cG5n' }
            : { type: 'text', text: '[{"node_id":1,"type":"Window"}]' };
        response.end(JSON.stringify({ jsonrpc: '2.0', id: body.id, result: { content: [reply] } }));
    });
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const client = new RuntimeClient({ url: `http://127.0.0.1:${(server.address() as AddressInfo).port}/mcp`, token });
    try {
        const [windows, screenshot] = await Promise.all([client.call('list_windows'), client.call('screenshot', { node_id: 1 })]);
        assert.deepEqual(windows, [{ node_id: 1, type: 'Window' }]);
        assert.equal(screenshot, 'cG5n');
        assert.deepEqual(methods, ['initialize', 'notifications/initialized', 'tools/call', 'tools/call']);
        client.dispose();
        await Promise.race([deleted, new Promise((_, reject) => {
            const timer = setTimeout(() => reject(new Error('MCP session did not close.')), 2000);
            timer.unref();
        })]);
    } finally {
        client.dispose();
        server.closeAllConnections();
        server.close();
    }
}

function verifyRuntimeAnnouncements(): void {
    const read = runtimeEndpointReader();
    const token = 'a'.repeat(64);
    const bearer = `Authorization: Bearer ${token}`;
    const address = '127.0.0.1:5555';
    const url = `http://${address}/mcp`;
    assert.equal(read(bearer), undefined);
    assert.equal(read('FerroUI MCP: http://example.com:5555/mcp'), undefined);
    assert.equal(read(bearer), undefined);
    assert.equal(read(`FerroUI MCP: ${url}\r`), undefined);
    assert.equal(read('An unrelated application log'), undefined);
    assert.equal(read(bearer), undefined);
    assert.equal(read(`FerroUI MCP: ${url}`), undefined);
    assert.deepEqual(read(bearer), { url, token });
    assert.deepEqual(read(mcpEndpointPrefix + JSON.stringify({ address, token })), { url, token });
}
