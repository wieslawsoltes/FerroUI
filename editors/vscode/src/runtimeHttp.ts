import { request } from 'node:http';
import { Readable } from 'node:stream';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

export function sendRuntimeRequest(endpoint: RuntimeMcpEndpoint, method: 'POST' | 'DELETE', headers: Record<string, string>,
    signal: AbortSignal, body?: string): Promise<Response> {
    if (endpoint.devicePort === undefined) return fetch(endpoint.url, { method, headers, signal, body, redirect: 'error' });

    // ADB changes the socket address. Authenticate to the device's original HTTP authority.
    return new Promise((resolve, reject) => {
        const connection = request(endpoint.url, {
            method, signal, headers: { ...headers, Host: `127.0.0.1:${endpoint.devicePort}`,
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
