import { execFile } from 'node:child_process';
import * as path from 'node:path';
import { promisify } from 'node:util';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

const execute = promisify(execFile);
const lsof = '/usr/sbin/lsof';

/** Find the process that owns both the authenticated inspector and the selected simulator executable. */
export async function simulatorAppPid(device: string, bundleId: string, endpoint: RuntimeMcpEndpoint,
    signal: AbortSignal): Promise<number> {
    const run = async (command: string, args: string[]) => (await execute(command, args,
        { timeout: 5_000, maxBuffer: 16 * 1024, signal })).stdout.trim();
    try {
        const url = new URL(endpoint.url);
        const [pid, bundle] = await Promise.all([
            run(lsof, ['-nP', '-t', '-a', `-iTCP@${url.hostname}:${url.port || '80'}`, '-sTCP:LISTEN']),
            run('/usr/bin/xcrun', ['simctl', 'get_app_container', device, bundleId, 'app']),
        ]);
        if (!/^[1-9]\d*$/.test(pid) || !Number.isSafeInteger(Number(pid)) || !path.isAbsolute(bundle)) throw new Error();
        const executable = await run('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', path.join(bundle, 'Info.plist')]);
        if (!executable || executable === '.' || executable === '..' || path.basename(executable) !== executable) throw new Error();
        const owner = await run(lsof, ['-nP', '-t', '-a', '-p', pid, '-d', 'txt', '--', path.join(bundle, executable)]);
        if (owner !== pid) throw new Error();
        signal.throwIfAborted();
        return Number(pid);
    } catch {
        signal.throwIfAborted();
        throw new Error('Could not identify the running executable for this iOS simulator inspector. Stop the session and run the app again.');
    }
}
