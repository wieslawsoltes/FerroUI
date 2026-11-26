import { execFile } from 'node:child_process';
import * as path from 'node:path';
import { promisify } from 'node:util';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

const execute = promisify(execFile);
const lsof = '/usr/sbin/lsof';

const run = async (command: string, args: string[], signal: AbortSignal) => (await execute(command, args,
    { timeout: 5_000, maxBuffer: 16 * 1024, signal })).stdout.trim();

async function installedExecutable(device: string, bundleId: string, signal: AbortSignal): Promise<string> {
    const bundle = await run('/usr/bin/xcrun', ['simctl', 'get_app_container', device, bundleId, 'app'], signal);
    if (!path.isAbsolute(bundle)) throw new Error('Xcode did not return the installed simulator bundle.');
    const executable = await run('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', path.join(bundle, 'Info.plist')], signal);
    if (!executable || executable === '.' || executable === '..' || path.basename(executable) !== executable) {
        throw new Error('The installed simulator bundle has no valid executable.');
    }
    return path.join(bundle, executable);
}

/** The app container includes the device identity, even when multiple simulators run the same bundle ID. */
export async function simulatorAppIsRunning(device: string, bundleId: string, signal: AbortSignal): Promise<boolean> {
    const executable = await installedExecutable(device, bundleId, signal).catch(error => {
        signal.throwIfAborted();
        if ((error as { stderr?: string }).stderr?.includes('NSPOSIXErrorDomain, code=2')) return undefined;
        throw error;
    });
    if (!executable) return false;
    try {
        return (await run(lsof, ['-nP', '-t', '-a', '-d', 'txt', '--', executable], signal)).split('\n').some(pid => /^[1-9]\d*$/.test(pid));
    } catch (error) {
        signal.throwIfAborted();
        const result = error as { code?: number; stderr?: string; stdout?: string };
        if (result.code === 1 && !result.stdout && !result.stderr) return false;
        throw error;
    }
}

/** Find the process that owns both the authenticated inspector and the selected simulator executable. */
export async function simulatorAppPid(device: string, bundleId: string, endpoint: RuntimeMcpEndpoint,
    signal: AbortSignal): Promise<number> {
    try {
        const url = new URL(endpoint.url);
        const [pid, executable] = await Promise.all([
            run(lsof, ['-nP', '-t', '-a', `-iTCP@${url.hostname}:${url.port || '80'}`, '-sTCP:LISTEN'], signal),
            installedExecutable(device, bundleId, signal),
        ]);
        if (!/^[1-9]\d*$/.test(pid) || !Number.isSafeInteger(Number(pid))) throw new Error();
        const owner = await run(lsof, ['-nP', '-t', '-a', '-p', pid, '-d', 'txt', '--', executable], signal);
        if (owner !== pid) throw new Error();
        signal.throwIfAborted();
        return Number(pid);
    } catch {
        signal.throwIfAborted();
        throw new Error('Could not identify the running executable for this iOS simulator inspector. Stop the session and run the app again.');
    }
}
