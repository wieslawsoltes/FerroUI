import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import type { IosAppProfile } from './iosAppProfiles';
import type { IosDevice } from './iosDeviceBridge';
import { OwnedProcess } from './ownedProcess';
import type { ProcessOutput } from './reloadLog';
import { RuntimeConsole } from './runtimeConsole';
import type { RuntimeEndpoint } from './runtimeClient';
import { verifyRuntime } from './runtimeReady';

export interface IosDeviceProcess {
    pid: number;
    endpoint: RuntimeEndpoint;
    console: OwnedProcess;
    programPath?: string;
    close(terminate?: boolean): Promise<void>;
}

export async function launchIosDeviceApp(executable: string, device: IosDevice, app: IosAppProfile, output: ProcessOutput,
    signal: AbortSignal): Promise<IosDeviceProcess> {
    signal.throwIfAborted();
    const launch = randomBytes(16).toString('hex');
    const child = spawn(executable, ['devicectl', 'device', 'process', 'launch', '--console', '--device', device.id, '--', app.bundleId, ...app.args], {
        detached: true, stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env, DEVICECTL_CHILD_FERROUI_EDITOR_SOURCE: '1', DEVICECTL_CHILD_FERROUI_MCP_DEVICE_LAUNCH: launch },
    });
    const console = new OwnedProcess(child);
    const channel = new RuntimeConsole(child.stdin, [child.stdout, child.stderr], launch, line => output.appendLine(line));
    // CoreDevice forwards catchable signals. Its console owns the remote app after its launch receipt.
    const stop = (terminate = true) => console.stop(terminate && channel.pid !== undefined ? 'SIGTERM' : 'SIGKILL');
    channel.signal.addEventListener('abort', () => {
        if (!console.hasExited) void stop();
    }, { once: true });
    const ended = () => channel.close(new Error('The iOS app console exited. Check the device and its launch arguments.'));
    void console.closed.then(ended, ended);
    const close = async (terminate = true) => {
        const stopped = stop(terminate);
        channel.close();
        await stopped;
    };
    const timeout = AbortSignal.timeout(20_000);
    const active = AbortSignal.any([signal, timeout]);
    try {
        const pid = await channel.waitForReceipt(active);
        const endpoint = { channel };
        await verifyRuntime(endpoint, signal);
        signal.throwIfAborted();
        if (console.hasExited) throw new Error('The iOS app ended before its first frame could be accepted.');
        return { pid, endpoint, console, close };
    } catch (error) {
        await close();
        signal.throwIfAborted();
        if (timeout.aborted && channel.pid === undefined) {
            throw new Error('Launch a stopped development app with its FerroUI stdio inspector enabled. The CoreDevice console did not receive its launch receipt.');
        }
        throw error;
    }
}
