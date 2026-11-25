import { Agent, request } from 'node:http';
import { Readable } from 'node:stream';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

export function sendRuntimeRequest(endpoint: RuntimeMcpEndpoint, method: 'POST' | 'DELETE', headers: Record<string, string>,
    signal: AbortSignal, body?: string): Promise<Response> {
    const { devicePort, openStream } = endpoint;
    if (devicePort === undefined && !openStream) return fetch(endpoint.url, { method, headers, signal, body, redirect: 'error' });

    // A device connection can change how the socket opens while keeping the app's HTTP authority.
    // VS Code's HTTP proxy wrapper delegates to the supplied agent for loopback requests.
    const agent = openStream ? new Agent({ keepAlive: false }) : undefined;
    if (agent && openStream) agent.createConnection = () => openStream(signal);
    return new Promise((resolve, reject) => {
        const connection = request(endpoint.url, {
            method, signal, agent,
            headers: { ...headers,
                ...(devicePort === undefined ? {} : { Host: `127.0.0.1:${devicePort}` }),
                ...(openStream ? { Connection: 'close' } : {}),
                ...(body === undefined ? {} : { 'Content-Length': Buffer.byteLength(body) }) },
        }, incoming => {
            const status = incoming.statusCode ?? 500;
            const responseHeaders = new Headers();
            const session = incoming.headers['mcp-session-id'];
            if (typeof session === 'string') responseHeaders.set('Mcp-Session-Id', session);
            if (status < 200 || status === 204 || status === 205 || status === 304) {
                incoming.resume();
                resolve(new Response(null, { status: status < 200 ? 502 : status, headers: responseHeaders }));
                return;
            }
            const stream = Readable.toWeb(incoming) as ReadableStream<Uint8Array>;
            resolve(new Response(stream, { status, headers: responseHeaders }));
        });
        connection.once('error', reject);
        connection.end(body);
    });
}
