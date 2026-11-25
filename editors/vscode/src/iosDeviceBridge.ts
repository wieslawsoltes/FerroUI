import { execFile } from 'node:child_process';
import { mkdtemp, readFile, realpath, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import type { IosAppProfile } from './iosAppProfiles';

export const xcrun = '/usr/bin/xcrun';
const execute = promisify(execFile);
const deviceId = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i;

export interface IosDevice {
    id: string;
    name: string;
    system: string;
    status: 'ready' | 'unpaired' | 'developerMode' | 'unavailable';
}

interface DeviceRecord {
    identifier?: unknown;
    hardwareProperties?: { platform?: unknown };
    deviceProperties?: { name?: unknown; osVersionNumber?: unknown; developerModeStatus?: unknown };
    connectionProperties?: { pairingState?: unknown; tunnelState?: unknown; transportType?: unknown };
}

export function deviceInventory(value: unknown): IosDevice[] {
    const devices = (value as { devices?: unknown } | null)?.devices;
    if (!Array.isArray(devices)) throw new Error('Xcode did not return a CoreDevice inventory.');
    return devices.flatMap((device: DeviceRecord | null) => {
        const id = device?.identifier;
        const platform = device?.hardwareProperties?.platform;
        const properties = device?.deviceProperties;
        const connection = device?.connectionProperties;
        if (typeof id !== 'string' || !deviceId.test(id) || platform !== 'iOS' && platform !== 'iPadOS'
            || typeof properties?.name !== 'string' || !properties.name.trim()) return [];
        const version = properties.osVersionNumber;
        const status: IosDevice['status'] = connection?.pairingState !== 'paired' ? 'unpaired'
            : properties.developerModeStatus !== 'enabled' ? 'developerMode'
            : connection.tunnelState === 'connected' || connection.tunnelState === 'disconnected' && typeof connection.transportType === 'string'
                ? 'ready' : 'unavailable';
        return [{ id, name: properties.name, system: typeof version === 'string' ? `${platform} ${version}` : platform, status }];
    });
}

export class IosDeviceBridge {
    constructor(readonly executable = xcrun) {}

    async devices(signal: AbortSignal): Promise<IosDevice[]> {
        try { return deviceInventory(await this.command(['list', 'devices'], 8_000, signal)); }
        catch {
            signal.throwIfAborted();
            throw new Error('The workspace Mac could not list iOS devices. Check the selected Xcode and its Devices and Simulators window.');
        }
    }

    async install(device: IosDevice, app: IosAppProfile, signal: AbortSignal): Promise<void> {
        if (!app.bundlePath) return;
        const bundle = await signedBundle(app.bundlePath, app.bundleId, signal);
        try { await this.command(['device', 'install', 'app', '--device', device.id, bundle], 35_000, signal); }
        catch {
            signal.throwIfAborted();
            throw new Error(`Xcode could not install ${app.label} on ${device.name}. Check its development signature and provisioning profile.`);
        }
    }

    private async command(args: string[], timeout: number, signal: AbortSignal): Promise<unknown> {
        signal.throwIfAborted();
        const scratch = await mkdtemp(path.join(tmpdir(), 'ferroui-coredevice-'));
        try {
            const output = path.join(scratch, 'result.json');
            await execute(this.executable, ['devicectl', ...args, '--timeout', String(timeout / 1000), '--json-output', output, '--quiet'],
                { timeout: timeout + 1000, signal, maxBuffer: 128 * 1024 });
            if ((await stat(output)).size > 4 * 1024 * 1024) throw new Error('The CoreDevice report exceeds 4 MiB.');
            return (JSON.parse(await readFile(output, 'utf8')) as { result: unknown }).result;
        } finally { await rm(scratch, { recursive: true, force: true }); }
    }
}

async function signedBundle(bundlePath: string, bundleId: string, signal: AbortSignal): Promise<string> {
    try {
        const bundle = await realpath(bundlePath);
        if (path.extname(bundle).toLowerCase() !== '.app' || !(await stat(bundle)).isDirectory()) throw new Error('Invalid app bundle');
        const info = path.join(bundle, 'Info.plist');
        const property = async (name: string) => (await execute('/usr/bin/plutil', ['-extract', name, 'raw', '-o', '-', info],
            { timeout: 5_000, maxBuffer: 4_096, signal })).stdout.trim();
        const [identifier, platform] = await Promise.all([property('CFBundleIdentifier'), property('CFBundleSupportedPlatforms.0')]);
        if (identifier !== bundleId || platform !== 'iPhoneOS') throw new Error('Bundle identifier or platform mismatch');
        await execute('/usr/bin/codesign', ['--verify', '--strict', bundle], { timeout: 10_000, maxBuffer: 8_192, signal });
        return bundle;
    } catch {
        signal.throwIfAborted();
        throw new Error(`Choose a development-signed iPhoneOS .app for ${bundleId}.`);
    }
}
