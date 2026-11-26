import type { Readable, Writable } from 'node:stream';
import { BrowserCdp, cdpMessageLimit } from './browserCdp';

/** Chromium sends one UTF-8 JSON packet per zero-terminated frame on its debug pipe. */
export function browserPipe(out: Writable, input: Readable): BrowserCdp {
    const cdp = new BrowserCdp(packet => { out.write(packet + '\0'); }, () => { out.destroy(); input.destroy(); });
    let parts: Buffer[] = [];
    let size = 0;
    input.on('data', (chunk: Buffer) => {
        let offset = 0;
        while (offset < chunk.length) {
            const delimiter = chunk.indexOf(0, offset);
            const end = delimiter === -1 ? chunk.length : delimiter;
            size += end - offset;
            if (size > cdpMessageLimit) { cdp.close(); return; }
            parts.push(chunk.subarray(offset, end));
            if (delimiter === -1) return;
            cdp.accept(Buffer.concat(parts, size).toString('utf8'));
            parts = [];
            size = 0;
            offset = delimiter + 1;
        }
    });
    input.once('close', () => cdp.close());
    input.once('error', () => cdp.close());
    out.once('error', () => cdp.close());
    return cdp;
}

export async function browserSocket(url: string, signal: AbortSignal): Promise<BrowserCdp> {
    signal.throwIfAborted();
    const socket = new WebSocket(url);
    const cdp = new BrowserCdp(packet => socket.send(packet), () => {
        if (socket.readyState < WebSocket.CLOSING) socket.close();
    });
    const close = () => cdp.close();
    socket.addEventListener('close', close, { once: true });
    socket.addEventListener('error', close, { once: true });
    socket.addEventListener('message', ({ data }) => {
        if (typeof data === 'string') cdp.accept(data);
        else cdp.close();
    });
    try {
        await new Promise<void>((resolve, reject) => {
            const deadline = AbortSignal.any([signal, AbortSignal.timeout(6_000)]);
            const finish = () => {
                socket.removeEventListener('open', connected);
                deadline.removeEventListener('abort', cancel);
            };
            const connected = () => { finish(); resolve(); };
            const cancel = () => { finish(); reject(deadline.reason); };
            void cdp.closed.then(() => { finish(); reject(new Error('Could not connect to the selected Chromium page.')); });
            socket.addEventListener('open', connected, { once: true });
            deadline.addEventListener('abort', cancel, { once: true });
        });
        signal.throwIfAborted();
        return cdp;
    } catch (error) { cdp.close(); throw error; }
}
