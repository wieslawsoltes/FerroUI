export const cdpMessageLimit = 24 * 1024 * 1024;

export interface BrowserEvent {
    method: string;
    params?: unknown;
    sessionId?: string;
}

interface Reply {
    id: number;
    result?: unknown;
    error?: { message: string };
}

export class BrowserCdp {
    private next = 0;
    private readonly pending = new Map<number, (reply?: Reply) => void>();
    private readonly listeners = new Set<(event: BrowserEvent) => void>();
    private readonly life = new AbortController();
    readonly closed: Promise<void>;
    private ended!: () => void;

    constructor(private readonly write: (packet: string) => void, private readonly stop: () => void) {
        this.closed = new Promise(resolve => { this.ended = resolve; });
    }

    request<T>(method: string, params: object, signal: AbortSignal, sessionId?: string): Promise<T> {
        const deadline = AbortSignal.any([this.life.signal, signal, AbortSignal.timeout(6_000)]);
        deadline.throwIfAborted();
        const id = ++this.next;
        return new Promise<T>((resolve, reject) => {
            const finish = () => {
                this.pending.delete(id);
                deadline.removeEventListener('abort', aborted);
            };
            const aborted = () => { finish(); reject(deadline.reason); };
            const receive = (reply?: Reply) => {
                finish();
                if (!reply) reject(new Error('The browser debugger closed.'));
                else if (reply.error) reject(new Error(`Chromium could not run ${method}: ${reply.error.message}`));
                else resolve(reply.result as T);
            };
            this.pending.set(id, receive);
            deadline.addEventListener('abort', aborted, { once: true });
            try { this.write(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })); }
            catch (error) { finish(); reject(error); }
        });
    }

    onEvent(listener: (event: BrowserEvent) => void): void {
        this.listeners.add(listener);
    }

    accept(packet: string): void {
        if (this.life.signal.aborted) return;
        if (packet.length > cdpMessageLimit) { this.close(); return; }
        try {
            const message = JSON.parse(packet);
            if (typeof message?.id === 'number') this.pending.get(message.id)?.(message as Reply);
            else if (typeof message?.method === 'string') this.listeners.forEach(listener => listener(message as BrowserEvent));
        } catch { this.close(); }
    }

    close(): void {
        if (this.life.signal.aborted) return;
        this.life.abort(new Error('The browser debugger closed.'));
        try { this.stop(); }
        finally { this.ended(); this.listeners.clear(); }
    }
}
