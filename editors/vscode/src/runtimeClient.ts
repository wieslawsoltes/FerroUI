import { localMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';
import { sendRuntimeRequest } from './runtimeHttp';

interface Content {
    type: string;
    text?: string;
    mimeType?: string;
    data?: string;
}

export interface RuntimeToolDefinition {
    name: string;
    description?: string;
    inputSchema: object;
    annotations?: { readOnlyHint?: boolean };
}

export interface RuntimeToolResult {
    content: Content[];
    structuredContent?: unknown;
    isError?: boolean;
}

export interface RuntimeInspectionChannel {
    tools(signal: AbortSignal): Promise<RuntimeToolDefinition[]>;
    invoke(name: string, args: object, signal: AbortSignal): Promise<RuntimeToolResult>;
}

export type RuntimeEndpoint = RuntimeMcpEndpoint | { channel: RuntimeInspectionChannel };

const protocol = '2025-11-25';
const responseLimit = 24 * 1024 * 1024;

export class RuntimeClient {
    private readonly lifetime = new AbortController();
    private session?: string;
    private version = protocol;
    private initialize?: Promise<void>;
    private catalog?: Promise<RuntimeToolDefinition[]>;
    private next = 0;

    constructor(readonly endpoint: RuntimeEndpoint) {
        if ('channel' in endpoint) return;
        if (!localMcpEndpoint(endpoint.url) || !/^[0-9a-f]{64}$/i.test(endpoint.token)
            || endpoint.devicePort !== undefined && (!Number.isInteger(endpoint.devicePort) || endpoint.devicePort < 1 || endpoint.devicePort > 65_535)) {
            throw new Error('Choose an authenticated local FerroUI runtime.');
        }
    }

    async call<T>(name: string, args: object = {}, signal?: AbortSignal): Promise<T> {
        const { content: [content] } = await this.invoke(name, args, signal);
        if (content.type === 'image' && content.mimeType === 'image/png' && typeof content.data === 'string') return content.data as T;
        if (content.type !== 'text' || typeof content.text !== 'string') throw new Error('Unexpected runtime content.');
        return JSON.parse(content.text) as T;
    }

    async invoke(name: string, args: object, signal?: AbortSignal): Promise<RuntimeToolResult> {
        const { endpoint } = this;
        let result: RuntimeToolResult;
        if ('channel' in endpoint) result = await endpoint.channel.invoke(name, args, this.active(signal));
        else {
            await this.ensureConnected(endpoint);
            result = await this.request<RuntimeToolResult>(endpoint, 'tools/call', { name, arguments: args }, signal);
        }
        const content = result.content?.[0];
        if (!content || result.isError) throw new Error(content?.text ?? `The runtime could not run ${name}.`);
        return result;
    }

    async supports(name: string): Promise<boolean> {
        return (await this.tools()).some(tool => tool.name === name);
    }

    async tools(signal?: AbortSignal): Promise<RuntimeToolDefinition[]> {
        const { endpoint } = this;
        const load = () => 'channel' in endpoint ? endpoint.channel.tools(this.active(signal)) : this.ensureConnected(endpoint)
            .then(() => this.request<{ tools: RuntimeToolDefinition[] }>(endpoint, 'tools/list', {}, signal)).then(result => result.tools);
        return this.catalog ??= load().catch(error => { this.catalog = undefined; throw error; });
    }

    private active(signal?: AbortSignal): AbortSignal {
        return signal ? AbortSignal.any([this.lifetime.signal, signal]) : this.lifetime.signal;
    }

    private ensureConnected(endpoint: RuntimeMcpEndpoint): Promise<void> {
        return this.initialize ??= this.connect(endpoint).catch(error => { this.initialize = undefined; throw error; });
    }

    private async connect(endpoint: RuntimeMcpEndpoint): Promise<void> {
        const result = await this.request<{ protocolVersion: string }>(endpoint, 'initialize', {
            protocolVersion: protocol, capabilities: {}, clientInfo: { name: 'FerroUI Live App', version: '1' },
        });
        if (!this.session || typeof result.protocolVersion !== 'string') throw new Error('The runtime did not establish an MCP session.');
        this.version = result.protocolVersion;
        await this.request(endpoint, 'notifications/initialized', {}, undefined, true);
    }

    private headers(endpoint: RuntimeMcpEndpoint): Record<string, string> {
        return { Authorization: `Bearer ${endpoint.token}`, Accept: 'application/json, text/event-stream',
            ...(this.session ? { 'Mcp-Session-Id': this.session, 'MCP-Protocol-Version': this.version } : {}) };
    }

    private async request<T>(endpoint: RuntimeMcpEndpoint, method: string, params: object, signal?: AbortSignal, notify = false): Promise<T> {
        const id = ++this.next;
        const active = AbortSignal.any([this.active(signal), AbortSignal.timeout(5000)]);
        const body = JSON.stringify({ jsonrpc: '2.0', method, params, ...(notify ? {} : { id }) });
        const response = await sendRuntimeRequest(endpoint, 'POST', { ...this.headers(endpoint), 'Content-Type': 'application/json' }, active, body);
        if (!response.ok) {
            void response.body?.cancel();
            throw new Error(`The runtime returned HTTP ${response.status}.`);
        }
        this.session = response.headers.get('Mcp-Session-Id') ?? this.session;
        if (notify) {
            void response.body?.cancel();
            return undefined as T;
        }
        const packet = JSON.parse(await readResponse(response)) as { id?: number; result?: T; error?: { message: string } };
        if (packet.id !== id) throw new Error('The runtime reply did not match its request.');
        if (packet.error) throw new Error(packet.error.message);
        if (packet.result === undefined) throw new Error('The runtime returned no result.');
        return packet.result;
    }

    dispose(): void {
        if (this.lifetime.signal.aborted) return;
        this.lifetime.abort();
        const { endpoint } = this;
        if (this.session && 'url' in endpoint) void sendRuntimeRequest(endpoint, 'DELETE', this.headers(endpoint), AbortSignal.timeout(1500))
            .then(response => response.body?.cancel()).catch(() => {});
    }
}

async function readResponse(response: Response): Promise<string> {
    const reader = response.body?.getReader();
    if (!reader) throw new Error('The runtime returned an empty response.');
    const decoder = new TextDecoder();
    let count = 0;
    let text = '';
    try {
        for (;;) {
            const next = await reader.read();
            if (next.done) return text + decoder.decode();
            count += next.value.byteLength;
            if (count > responseLimit) throw new Error('The runtime response exceeds 24 MiB.');
            text += decoder.decode(next.value, { stream: true });
        }
    } finally {
        void reader.cancel().catch(() => {});
    }
}
