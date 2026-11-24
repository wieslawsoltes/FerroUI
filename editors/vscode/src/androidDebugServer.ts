import { randomUUID } from 'node:crypto';
import { posix } from 'node:path';
import { createInterface } from 'node:readline';
import { setTimeout as delay } from 'node:timers/promises';
import type { AndroidBridge, AndroidPortForward, AndroidReceipt } from './androidBridge';
import { OwnedProcess } from './ownedProcess';

export interface AndroidDebugServer {
    port: number;
    closed: Promise<number | null>;
    close(): Promise<void>;
}

export async function startAndroidDebugServer(bridge: AndroidBridge, packageId: string, receipt: AndroidReceipt,
    output: (message: string) => void, signal: AbortSignal): Promise<AndroidDebugServer> {
    const active = AbortSignal.any([signal, AbortSignal.timeout(15_000)]);
    const remote = receipt.debuggerPath;
    if (!remote) throw new Error('Install a debuggable APK that includes the FerroUI LLDB server. Build the sample with scripts/android/apk.sh inspector --debugger.');
    const runAs = (args: string[], cancellation: AbortSignal) => bridge.command(['shell', 'run-as', packageId, ...args], cancellation);
    const [apks, pwd] = await Promise.all([
        bridge.command(['shell', 'pm', 'path', packageId], active),
        runAs(['pwd'], active),
    ]);
    const installed = apks.split(/\r?\n/).some(line => line.startsWith('package:')
        && remote.startsWith(`${posix.dirname(line.trim().slice('package:'.length))}/lib/`));
    if (!installed) throw new Error('The LLDB server must be installed with the selected Android package.');
    const appData = pwd.trim();
    if (!/^\/data\/(?:data|user\/\d+)\/[A-Za-z0-9_.]+$/.test(appData) || !appData.endsWith(`/${packageId}`)) {
        throw new Error('Android could not verify the private directory of this debug app.');
    }
    const pipe = `${appData}/.ferroui-lldb-${randomUUID()}`;

    let port: AndroidPortForward | undefined;
    let process: OwnedProcess | undefined;
    let remotePid: string | undefined;
    let cleanup: Promise<void> | undefined;
    const close = () => cleanup ??= (async () => {
        if (remotePid && !process?.hasExited) {
            const owner = await runAs(['readlink', `/proc/${remotePid}/exe`], AbortSignal.timeout(4_000)).catch(() => '');
            if (owner.trim() === remote) await runAs(['kill', '-TERM', remotePid], AbortSignal.timeout(4_000)).catch(() => {});
        }
        await process?.stop();
        await Promise.all([port?.close(), runAs(['rm', '-f', pipe], AbortSignal.timeout(4_000))]);
    })();

    try {
        active.throwIfAborted();
        await runAs(['test', '-x', remote], active).catch(() => {
            active.throwIfAborted();
            throw new Error('The installed APK must extract its FerroUI LLDB server. Rebuild the debug APK with native library extraction.');
        });
        await runAs(['mkfifo', '-m', '600', pipe], active);
        if (!await bridge.matches(packageId, receipt, active)) throw new Error('The Android app process changed before the debugger could attach.');

        // Both ends of LLDB's readiness pipe run with the app UID. Only GDB traffic uses ADB forwarding.
        const start = `'printf "FERROUI_LLDB_PID=%s\\n" "$$"; exec ${remote} gdbserver 127.0.0.1:0 --named-pipe ${pipe} --attach ${receipt.pid}'`;
        const child = bridge.spawnShell(['run-as', packageId, 'sh', '-c', start]);
        process = new OwnedProcess(child);
        const lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
        lines.on('line', line => {
            const pid = /^FERROUI_LLDB_PID=([1-9]\d{0,9})$/.exec(line)?.[1];
            if (pid) remotePid ??= pid;
            else if (line) output(line.slice(0, 2_048));
        });
        child.stderr.on('data', (chunk: Buffer) => output(chunk.toString('utf8').trim().slice(0, 2_048)));
        const exited = new AbortController();
        const ended = () => { lines.close(); exited.abort(new Error('Android LLDB server exited. See the FerroUI Android Debugger output.')); };
        void process.closed.then(ended, ended);
        const readiness = AbortSignal.any([active, exited.signal, AbortSignal.timeout(8_000)]);
        const response = await bridge.command(['exec-out', 'run-as', packageId, 'cat', pipe], readiness, 8_000, 128);
        const devicePort = Number(/^([1-9]\d{0,4})\0$/.exec(response)?.[1] ?? 0);
        if (!devicePort || devicePort > 65_535) throw new Error('The Android LLDB server did not announce a loopback port.');
        while (!remotePid) await delay(50, undefined, { signal: readiness }).catch(() => readiness.throwIfAborted());
        port = await bridge.forwardPort(`tcp:${devicePort}`);
        readiness.throwIfAborted();
        return { port: port.port, closed: process.closed, close };
    } catch (error) {
        await close().catch(problem => output(String(problem)));
        signal.throwIfAborted();
        throw error;
    }
}
