import { localMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';

interface Content {
    type: string;
    text?: string;
    mimeType?: string;
    data?: string;
}

const protocol = '2025-11-25';
const responseLimit = 24 * 1024 * 1024;

export class RuntimeClient {
    private readonly lifetime = new AbortController();
    private session?: string;
    private version = protocol;
    private initialize?: Promise<void>;
    private catalog?: Promise<Set<string>>;
    private next = 0;

    constructor(readonly endpoint: RuntimeMcpEndpoint) {
        if (!localMcpEndpoint(endpoint.url) || !/^[0-9a-f]{64}$/i.test(endpoint.token)) {
            throw new Error('Choose an authenticated local FerroUI runtime.');
        }
    }

    async call<T>(name: string, args: object = {}, signal?: AbortSignal): Promise<T> {
        await this.ensureConnected();
        const result = await this.request<{ content: Content[]; isError?: boolean }>('tools/call', { name, arguments: args }, signal);
        const content = result.content?.[0];
        if (!content || result.isError) throw new Error(content?.text ?? `The runtime could not run ${name}.`);
        if (content.type === 'image' && content.mimeType === 'image/png' && typeof content.data === 'string') return content.data as T;
        if (content.type !== 'text' || typeof content.text !== 'string') throw new Error('Unexpected runtime content.');
        return JSON.parse(content.text) as T;
    }

    async supports(name: string): Promise<boolean> {
        await this.ensureConnected();
        this.catalog ??= this.request<{ tools: Array<{ name: string }> }>('tools/list', {})
            .then(result => new Set(result.tools.map(tool => tool.name)))
            .catch(error => { this.catalog = undefined; throw error; });
        return (await this.catalog).has(name);
    }

    private ensureConnected(): Promise<void> {
        return this.initialize ??= this.connect().catch(error => { this.initialize = undefined; throw error; });
    }

    private async connect(): Promise<void> {
        const result = await this.request<{ protocolVersion: string }>('initialize', {
            protocolVersion: protocol, capabilities: {}, clientInfo: { name: 'FerroUI Live App', version: '1' },
        });
        if (!this.session || typeof result.protocolVersion !== 'string') throw new Error('The runtime did not establish an MCP session.');
        this.version = result.protocolVersion;
        await this.request('notifications/initialized', {}, undefined, true);
    }

    private headers(): Record<string, string> {
        return { Authorization: `Bearer ${this.endpoint.token}`, Accept: 'application/json, text/event-stream',
            ...(this.session ? { 'Mcp-Session-Id': this.session, 'MCP-Protocol-Version': this.version } : {}) };
    }

    private async request<T>(method: string, params: object, signal?: AbortSignal, notify = false): Promise<T> {
        const id = ++this.next;
        const response = await fetch(this.endpoint.url, {
            method: 'POST', redirect: 'error', headers: { ...this.headers(), 'Content-Type': 'application/json' },
            signal: AbortSignal.any([this.lifetime.signal, AbortSignal.timeout(5000), ...(signal ? [signal] : [])]),
            body: JSON.stringify({ jsonrpc: '2.0', method, params, ...(notify ? {} : { id }) }),
        });
        if (!response.ok) {
            void response.body?.cancel();
            throw new Error(`The runtime returned HTTP ${response.status}.`);
        }
        this.session = response.headers.get('Mcp-Session-Id') ?? this.session;
        if (notify) return undefined as T;
        const packet = JSON.parse(await readResponse(response)) as { id?: number; result?: T; error?: { message: string } };
        if (packet.id !== id) throw new Error('The runtime reply did not match its request.');
        if (packet.error) throw new Error(packet.error.message);
        if (packet.result === undefined) throw new Error('The runtime returned no result.');
        return packet.result;
    }

    dispose(): void {
        if (this.lifetime.signal.aborted) return;
        this.lifetime.abort();
        if (this.session) void fetch(this.endpoint.url, { method: 'DELETE', redirect: 'error', headers: this.headers(),
            signal: AbortSignal.timeout(1500) }).then(response => response.body?.cancel()).catch(() => {});
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
