import { execFile } from 'node:child_process';
import { constants } from 'node:fs';
import { access, realpath, stat } from 'node:fs/promises';
import { homedir } from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import type { AndroidApp, AndroidDevice } from './androidTargets';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

const execute = promisify(execFile);
const adbName = process.platform === 'win32' ? 'adb.exe' : 'adb';

export async function androidBridgePath(signal: AbortSignal): Promise<string> {
    const configured = vscode.workspace.getConfiguration('ferroui.android').get<string>('adbPath', '').trim();
    if (configured && !path.isAbsolute(configured)) throw new Error('Set ferroui.android.adbPath to an absolute ADB executable path.');
    const sdk = process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT;
    const defaultSdk = process.platform === 'darwin' ? path.join(homedir(), 'Library', 'Android', 'sdk')
        : process.platform === 'win32' && process.env.LOCALAPPDATA ? path.join(process.env.LOCALAPPDATA, 'Android', 'Sdk') : path.join(homedir(), 'Android', 'Sdk');
    const sdkPaths = [sdk, defaultSdk].filter((root): root is string => typeof root === 'string' && path.isAbsolute(root));
    const candidates = configured ? [configured] : sdkPaths.map(root => path.join(root, 'platform-tools', adbName)).concat(adbName);
    for (const candidate of candidates) {
        try {
            if (path.isAbsolute(candidate)) await access(candidate, constants.X_OK);
            else await adb(candidate, ['version'], signal, 4_000);
            return candidate;
        } catch { signal.throwIfAborted(); }
    }
    throw new Error('Install Android SDK Platform Tools on the workspace host or set ferroui.android.adbPath.');
}

export async function androidDevices(executable: string, signal: AbortSignal): Promise<AndroidDevice[]> {
    const output = await adb(executable, ['devices', '-l'], signal);
    return output.split(/\r?\n/).flatMap(line => {
        const match = /^(\S{1,128})\s+(device|offline|unauthorized|no permissions|recovery|sideload|bootloader)(?:\s+(.*))?$/.exec(line);
        if (!match) return [];
        const [, id, status, description = ''] = match;
        const name = /(?:^|\s)model:([A-Za-z0-9_.-]+)/.exec(description)?.[1]?.replaceAll('_', ' ') ?? id;
        return [{ id, name, status, emulator: id.startsWith('emulator-') }];
    });
}

export interface AndroidReceipt {
    pid: number;
    port: number;
    token: string;
}

export class AndroidBridge {
    constructor(private readonly executable: string, readonly device: AndroidDevice) {}

    async install(app: AndroidApp, signal: AbortSignal): Promise<void> {
        if (!app.apkPath) return;
        const apk = await realpath(app.apkPath).catch(() => undefined);
        if (!apk || path.extname(apk).toLowerCase() !== '.apk' || !(await stat(apk).catch(() => undefined))?.isFile()) {
            throw new Error(`Build or select the APK for ${app.packageId}.`);
        }
        const installed = await this.command(['install', '-r', '-t', '--pkg', app.packageId, apk], signal, 25_000);
        if (!/^Success\s*$/m.test(installed)) throw new Error(`Android could not install the APK for ${app.packageId}.`);
    }

    async checkDebuggable(packageId: string, signal: AbortSignal): Promise<void> {
        const uid = await this.command(['shell', 'run-as', packageId, 'id', '-u'], signal).catch(() => '');
        signal.throwIfAborted();
        if (!/^\d+$/.test(uid.trim()) || Number(uid) <= 0) {
            throw new Error(`Install a debuggable build of ${packageId} on the chosen device.`);
        }
    }

    async launch(app: AndroidApp): Promise<void> {
        const started = await this.command(['shell', 'am', 'start', '-S', '-W', '-n', `${app.packageId}/${app.activity}`],
            AbortSignal.timeout(12_000), 12_000);
        if (!/^Status: ok\s*$/m.test(started)) throw new Error(`Android could not start ${app.activity} in ${app.packageId}.`);
    }

    async pid(packageId: string, signal: AbortSignal): Promise<number | undefined> {
        const output = await this.command(['shell', 'pidof', '-s', packageId], signal).catch(() => '');
        signal.throwIfAborted();
        const value = output.trim();
        return /^\d{1,10}$/.test(value) && Number(value) > 0 ? Number(value) : undefined;
    }

    async receipt(packageId: string, signal: AbortSignal): Promise<AndroidReceipt | undefined> {
        const output = await this.command(['exec-out', 'run-as', packageId, 'cat', 'files/ferroui-live-app.json'], signal, 3_000, 4_096)
            .catch(() => '');
        signal.throwIfAborted();
        try {
            const { version, pid, port, token } = JSON.parse(output);
            if (version === 1 && Number.isSafeInteger(pid) && pid > 0 && validPort(port)
                && typeof token === 'string' && /^[a-f0-9]{64}$/i.test(token)) return { pid, port, token };
        } catch { /* The app has yet to supply a complete receipt. */ }
    }

    async forward(receipt: AndroidReceipt): Promise<AndroidForward> {
        const port = Number((await this.command(['forward', '--no-rebind', 'tcp:0', `tcp:${receipt.port}`], AbortSignal.timeout(4_000))).trim());
        if (!validPort(port)) throw new Error('ADB did not allocate a local inspector port.');
        return { endpoint: { url: `http://127.0.0.1:${port}/mcp`, token: receipt.token, devicePort: receipt.port },
            close: () => this.removeForward(port, receipt.port) };
    }

    async stop(packageId: string): Promise<void> {
        await this.command(['shell', 'am', 'force-stop', packageId], AbortSignal.timeout(4_000));
    }

    private async removeForward(local: number, remote: number): Promise<void> {
        const signal = AbortSignal.timeout(4_000);
        const output = await adb(this.executable, ['forward', '--list'], signal);
        const owned = output.split(/\r?\n/).some(line => line.trim().split(/\s+/).join(' ') === `${this.device.id} tcp:${local} tcp:${remote}`);
        if (owned) await this.command(['forward', '--remove', `tcp:${local}`], signal);
    }

    private command(args: string[], signal: AbortSignal, timeout?: number, maxBuffer?: number): Promise<string> {
        return adb(this.executable, ['-s', this.device.id, ...args], signal, timeout, maxBuffer);
    }
}

export interface AndroidForward {
    endpoint: RuntimeMcpEndpoint;
    close(): Promise<void>;
}

function validPort(port: unknown): port is number {
    return typeof port === 'number' && Number.isInteger(port) && port > 0 && port < 65_536;
}

async function adb(executable: string, args: string[], signal: AbortSignal, timeout = 6_000, maxBuffer = 64 * 1024): Promise<string> {
    try {
        // The assigned forward belongs to the workspace host's local ADB server.
        const env = { ...process.env, ADB_SERVER_SOCKET: undefined, ANDROID_ADB_SERVER_ADDRESS: undefined };
        const { stdout } = await execute(executable, args, { signal, timeout, maxBuffer, windowsHide: true, env });
        return stdout;
    } catch {
        signal.throwIfAborted();
        throw new Error(`ADB could not complete ${args[0] === '-s' ? args[2] : args[0]} on the workspace host.`);
    }
}
