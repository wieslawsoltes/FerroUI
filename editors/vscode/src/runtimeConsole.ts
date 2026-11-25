import type { Readable, Writable } from 'node:stream';
import type { RuntimeInspectionChannel, RuntimeToolDefinition, RuntimeToolResult } from './runtimeClient';
import { mcpEndpointPrefix } from './runtimeMcp';

const receiptPrefix = 'FERROUI_MCP_DEVICE_V1=';
const protocol = '2025-11-25';
const requestLimit = 1024 * 1024;
const lineLimit = 24 * requestLimit;

interface Reply { id?: number; jsonrpc?: string; result?: unknown; error?: { message?: string } }

/** The app owns the launch receipt. Tool replies use standard MCP stdio framing. */
export class RuntimeConsole implements RuntimeInspectionChannel {
    private readonly lifetime = new AbortController();
    private readonly pending = new Map<number, (reply: Reply) => void>();
    private readonly unsubscribe: (() => void)[];
    private ready?: (pid: number) => void;
    private connection?: Promise<void>;
    private next = 0;
    private remotePid?: number;

    constructor(private readonly input: Writable, streams: Readable[], private readonly launch: string,
        private readonly report: (line: string) => void) {
        input.on('error', error => this.close(error));
        this.unsubscribe = streams.map(stream => this.listen(stream));
    }

    get pid(): number | undefined { return this.remotePid; }
    get signal(): AbortSignal { return this.lifetime.signal; }

    async waitForReceipt(signal: AbortSignal): Promise<number> {
        const active = AbortSignal.any([this.lifetime.signal, signal]);
        active.throwIfAborted();
        if (this.pid !== undefined) return this.pid;
        if (this.ready) throw new Error('The console is already waiting for its launch receipt.');
        return new Promise((resolve, reject) => {
            const finish = () => {
                this.ready = undefined;
                active.removeEventListener('abort', abort);
            };
            const abort = () => { finish(); reject(active.reason); };
            this.ready = pid => { finish(); resolve(pid); };
            active.addEventListener('abort', abort, { once: true });
        });
    }

    async tools(signal: AbortSignal): Promise<RuntimeToolDefinition[]> {
        await this.connect(signal);
        const { tools } = await this.request<{ tools: RuntimeToolDefinition[] }>('tools/list', {}, signal);
        if (!Array.isArray(tools)) throw new Error('The device inspector did not return a tool catalog.');
        return tools;
    }

    async invoke(name: string, args: object, signal: AbortSignal): Promise<RuntimeToolResult> {
        await this.connect(signal);
        return this.request('tools/call', { name, arguments: args }, signal);
    }

    private connect(signal: AbortSignal): Promise<void> {
        return this.connection ??= (async () => {
            if (this.pid === undefined) throw new Error('The device app has not acknowledged its launch.');
            const result = await this.request<{ protocolVersion: string }>('initialize', {
                protocolVersion: protocol, capabilities: {}, clientInfo: { name: 'FerroUI Live App', version: '1' },
            }, signal);
            if (typeof result?.protocolVersion !== 'string') throw new Error('The device inspector could not initialize MCP.');
            await this.write({ jsonrpc: '2.0', method: 'notifications/initialized' });
        })();
    }

    close(reason = new Error('The app console closed.')): void {
        if (this.lifetime.signal.aborted) return;
        this.lifetime.abort(reason);
        this.unsubscribe.forEach(stop => stop());
        this.input.destroy();
    }

    private request<T>(method: string, params: object, signal: AbortSignal): Promise<T> {
        const active = AbortSignal.any([signal, this.lifetime.signal, AbortSignal.timeout(12_000)]);
        active.throwIfAborted();
        const id = ++this.next;
        return new Promise((resolve, reject) => {
            const finish = () => {
                this.pending.delete(id);
                active.removeEventListener('abort', abort);
            };
            const abort = () => {
                finish();
                if (!this.lifetime.signal.aborted) void this.write({ jsonrpc: '2.0', method: 'notifications/cancelled', params: { requestId: id } }).catch(() => {});
                reject(active.reason);
            };
            this.pending.set(id, reply => {
                finish();
                if (reply.error || reply.result === undefined) reject(new Error(reply.error?.message ?? 'The device inspector returned no result.'));
                else resolve(reply.result as T);
            });
            active.addEventListener('abort', abort, { once: true });
            void this.write({ jsonrpc: '2.0', id, method, params }).catch(error => { finish(); reject(error); });
        });
    }

    private async write(packet: object): Promise<void> {
        this.lifetime.signal.throwIfAborted();
        const line = JSON.stringify(packet) + '\n';
        if (Buffer.byteLength(line) > requestLimit) throw new Error('The device inspector request exceeds 1 MiB.');
        await new Promise<void>((resolve, reject) => this.input.write(line, error => error ? reject(error) : resolve()));
    }

    private listen(stream: Readable): () => void {
        let line = '';
        let bytes = 0;
        const data = (chunk: string) => {
            const parts = chunk.split('\n');
            for (const [index, part] of parts.entries()) {
                bytes += Buffer.byteLength(part);
                if (bytes > lineLimit) { this.close(new Error('The app console response exceeds 24 MiB.')); return; }
                line += part;
                if (index === parts.length - 1) break;
                this.receive(line.trim());
                line = '';
                bytes = 0;
            }
        };
        stream.setEncoding('utf8').on('data', data);
        stream.on('error', error => this.close(error));
        return () => { stream.removeListener('data', data); stream.resume(); };
    }

    private receive(line: string): void {
        if (line.startsWith(receiptPrefix)) {
            try {
                const { launch, pid } = JSON.parse(line.slice(receiptPrefix.length));
                if (launch === this.launch && Number.isSafeInteger(pid) && pid > 0 && this.pid === undefined) {
                    this.remotePid = pid;
                    this.ready?.(pid);
                }
            } catch { /* Ignore incomplete or unrelated launch receipts. */ }
            return;
        }
        if (line.startsWith('{')) {
            try {
                const reply = JSON.parse(line) as Reply;
                if (reply.jsonrpc === '2.0') {
                    if (typeof reply.id === 'number') this.pending.get(reply.id)?.(reply);
                    return;
                }
            } catch { /* Ordinary app or Xcode console output. */ }
        }
        if (line && !line.includes(this.launch) && !line.includes(mcpEndpointPrefix) && !/Authorization:\s*Bearer/i.test(line)) {
            this.report(line.slice(0, 8192));
        }
    }
}
