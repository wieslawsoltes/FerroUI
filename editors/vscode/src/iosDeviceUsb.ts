import { spawn } from 'node:child_process';
import { constants } from 'node:fs';
import { access, realpath, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { Duplex } from 'node:stream';
import { OwnedProcess } from './ownedProcess';

export async function iosUsbTool(configured: string, signal: AbortSignal): Promise<string> {
    const selected = configured.trim();
    if (selected && !path.isAbsolute(selected)) throw new Error('Set ferroui.iosDevice.inetcatPath to an absolute inetcat executable path.');
    const directories = ['/opt/homebrew/bin', '/usr/local/bin', '/opt/local/bin', ...(process.env.PATH ?? '').split(path.delimiter)];
    const candidates = selected ? [selected] : directories.filter(path.isAbsolute).map(directory => path.join(directory, 'inetcat'));
    for (const candidate of new Set(candidates)) {
        signal.throwIfAborted();
        try {
            const executable = await realpath(candidate);
            await access(executable, constants.X_OK);
            if ((await stat(executable)).isFile()) return executable;
        } catch { signal.throwIfAborted(); }
    }
    throw new Error('Install inetcat from libusbmuxd on the workspace Mac or set ferroui.iosDevice.inetcatPath.');
}

/** Each HTTP connection owns one byte stream and one USB helper. */
export class IosUsbStreams {
    private readonly lifetime = new AbortController();
    private readonly processes = new Set<OwnedProcess>();

    constructor(private readonly executable: string, private readonly udid: string, private readonly port: number) {}

    open = (signal: AbortSignal): Duplex => {
        const active = AbortSignal.any([this.lifetime.signal, signal]);
        active.throwIfAborted();
        const env = { ...process.env };
        delete env.USBMUXD_SOCKET_ADDRESS;
        const child = spawn(this.executable, ['--local', '--udid', this.udid, String(this.port)], {
            detached: true, stdio: ['pipe', 'pipe', 'ignore'], env,
        });
        const owned = new OwnedProcess(child);
        const stream = Duplex.from({ readable: child.stdout, writable: child.stdin });
        this.processes.add(owned);
        const abort = () => stream.destroy();
        active.addEventListener('abort', abort, { once: true });
        stream.once('close', () => {
            active.removeEventListener('abort', abort);
            void owned.stop();
        });
        void owned.closed.then(() => this.processes.delete(owned), () => {
            this.processes.delete(owned);
            stream.destroy(new Error('The iOS USB helper could not open a connection.'));
        });
        return stream;
    };

    async close(): Promise<void> {
        this.lifetime.abort();
        await Promise.all([...this.processes].map(process => process.stop()));
    }
}
