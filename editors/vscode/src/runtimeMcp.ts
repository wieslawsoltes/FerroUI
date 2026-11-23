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

/** Decode either the managed notification or the ordinary two-line app announcement. */
export function runtimeEndpointReader(): (line: string) => RuntimeMcpEndpoint | undefined {
    let url: string | undefined;
    return raw => {
        const line = raw.trim();
        if (line.startsWith(mcpEndpointPrefix)) {
            url = undefined;
            return parseMcpEndpoint(line.slice(mcpEndpointPrefix.length));
        }
        const token = /^Authorization: Bearer ([a-f0-9]{64})$/i.exec(line)?.[1];
        const endpoint = url && token ? { url, token } : undefined;
        const announced = /^FerroUI MCP: (\S+)$/.exec(line)?.[1];
        url = announced && localMcpEndpoint(announced) ? announced : undefined;
        return endpoint;
    };
}
