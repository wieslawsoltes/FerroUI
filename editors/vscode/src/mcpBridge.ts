import { randomBytes, timingSafeEqual } from 'node:crypto';
import { createServer, type Socket } from 'node:net';
import { errorMessage } from './protocol';

const maxRequest = 1024 * 1024 + 256;
const maxResponse = 16 * 1024 * 1024;

export class McpBridge {
    private readonly token = randomBytes(32).toString('hex');
    private readonly peers = new Set<Socket>();
    private readonly server = createServer(socket => this.connect(socket));
    private listening?: Promise<string>;
    private disposed = false;

    constructor(private readonly tools: {
        definitions(): object[];
        call(name: string, args: unknown, signal: AbortSignal): Promise<unknown>;
    }) {
        this.server.maxConnections = 16;
    }

    async environment(): Promise<Record<string, string>> {
        this.listening ??= new Promise((resolve, reject) => {
            const failed = (error: Error) => { this.listening = undefined; reject(error); };
            this.server.once('error', failed);
            this.server.listen(0, '127.0.0.1', () => {
                this.server.off('error', failed);
                this.server.unref();
                const address = this.server.address();
                if (!address || typeof address === 'string') { reject(new Error('FerroUI MCP bridge did not bind')); return; }
                if (this.disposed) this.server.close();
                resolve(`127.0.0.1:${address.port}`);
            });
        });
        return { FERROUI_EDITOR_MCP_ADDRESS: await this.listening, FERROUI_EDITOR_MCP_TOKEN: this.token };
    }

    private connect(socket: Socket): void {
        if (this.disposed) { socket.destroy(); return; }
        this.peers.add(socket);
        const controller = new AbortController();
        const deadline = setTimeout(() => socket.destroy(), 35_000);
        socket.on('error', () => socket.destroy());
        socket.on('close', () => {
            clearTimeout(deadline);
            controller.abort(new Error('The MCP client cancelled or disconnected.'));
            this.peers.delete(socket);
        });
        let queued: Buffer[] = [];
        let length = 0;
        let received = false;
        socket.on('data', (chunk: Buffer) => {
            if (received) { socket.destroy(); return; }
            const end = chunk.indexOf(0x0a);
            length += end === -1 ? chunk.length : end;
            if (length > maxRequest || (end !== -1 && end !== chunk.length - 1)) { socket.destroy(); return; }
            if (end === -1) { queued.push(chunk); return; }
            received = true;
            queued.push(chunk.subarray(0, end));
            const text = Buffer.concat(queued, length).toString('utf8');
            queued = [];
            void this.respond(socket, text, controller.signal);
        });
    }

    private async respond(socket: Socket, text: string, signal: AbortSignal): Promise<void> {
        let body: { type?: string; token?: string; name?: string; arguments?: unknown };
        try { body = JSON.parse(text); } catch { socket.destroy(); return; }
        if (typeof body?.token !== 'string' || !/^[0-9a-f]{64}$/.test(body.token)
            || !timingSafeEqual(Buffer.from(body.token), Buffer.from(this.token))) { socket.destroy(); return; }
        try {
            let result: unknown;
            if (body.type === 'catalog') result = this.tools.definitions();
            else if (body.type === 'call' && typeof body.name === 'string') result = await this.tools.call(body.name, body.arguments, signal);
            else throw new Error('Unknown FerroUI bridge request');
            if (signal.aborted) return;
            const output = JSON.stringify({ result });
            if (Buffer.byteLength(output) > maxResponse) throw new Error('The FerroUI tool response exceeds 16 MiB');
            socket.end(`${output}\n`);
        } catch (error) {
            if (!signal.aborted) socket.end(`${JSON.stringify({ error: errorMessage(error) })}\n`);
        }
    }

    dispose(): void {
        this.disposed = true;
        for (const socket of this.peers) socket.destroy();
        if (this.server.listening) this.server.close();
    }
}
