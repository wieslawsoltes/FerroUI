import { isIPv4 } from 'node:net';

export interface RuntimeMcpEndpoint { url: string; token: string }

export const mcpEndpointPrefix = 'FERROUI_MCP_V1=';

export function localMcpEndpoint(value: string): boolean {
    try {
        const url = new URL(value);
        const host = url.hostname;
        return url.protocol === 'http:' && (host === '[::1]' || isIPv4(host) && host.startsWith('127.'))
            && url.pathname === '/mcp' && !url.search && !url.hash && !url.username && !url.password;
    } catch { return false; }
}

export function parseMcpEndpoint(text: string): RuntimeMcpEndpoint | undefined {
    try {
        const { address, token } = JSON.parse(text);
        if (typeof address !== 'string' || !/^(?:127(?:\.\d{1,3}){3}|\[::1\]):[1-9]\d{0,4}$/.test(address)
            || typeof token !== 'string' || !/^[0-9a-f]{64}$/i.test(token)) return undefined;
        const url = `http://${address}/mcp`;
        if (localMcpEndpoint(url)) return { url, token };
    } catch { /* Invalid runtime notification */ }
    return undefined;
}
