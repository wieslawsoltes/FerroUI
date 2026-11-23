import { execFile, spawn } from 'node:child_process';
import { realpath, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { promisify } from 'node:util';
import { OwnedProcess } from './ownedProcess';
import { runtimeEndpointReader, type RuntimeMcpEndpoint } from './runtimeMcp';
import type { IosSimulator, IosSimulatorApp } from './iosSimulatorTargets';

const xcrun = '/usr/bin/xcrun';
const execute = promisify(execFile);
const udid = /^[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}$/;
const iosRuntime = /^com\.apple\.CoreSimulator\.SimRuntime\.iOS-(\d+(?:-\d+)*)$/;

export async function iosSimulators(signal: AbortSignal): Promise<IosSimulator[]> {
    try {
        const { stdout } = await execute(xcrun, ['simctl', 'list', 'devices', 'available', '--json'],
            { timeout: 8_000, maxBuffer: 1024 * 1024, signal });
        const devices = JSON.parse(stdout).devices;
        return Object.entries(devices as Record<string, unknown>).flatMap(([runtime, list]) => {
            const version = iosRuntime.exec(runtime)?.[1];
            if (!version || !Array.isArray(list)) return [];
            return list.flatMap(device => device?.isAvailable === true && typeof device.name === 'string'
                && typeof device.udid === 'string' && udid.test(device.udid)
                ? [{ id: device.udid, name: device.name, system: `iOS ${version.replaceAll('-', '.')}`, booted: device.state === 'Booted' }]
                : []);
        });
    } catch {
        signal.throwIfAborted();
        throw new Error('The workspace Mac could not list iOS simulators. Check the selected Xcode and its installed iOS runtime.');
    }
}

export async function installSimulatorApp(device: IosSimulator, app: IosSimulatorApp, signal: AbortSignal): Promise<void> {
    if (!app.bundlePath) return;
    let bundle: string;
    try {
        bundle = await realpath(app.bundlePath);
        if (!(await stat(bundle)).isDirectory() || path.extname(bundle).toLowerCase() !== '.app') throw new Error('Not an app bundle');
        const info = path.join(bundle, 'Info.plist');
        const { stdout } = await execute('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', info],
            { maxBuffer: 4_096, timeout: 5_000, signal });
        if (stdout.trim() !== app.bundleId) throw new Error('Bundle identifier mismatch');
    } catch {
        signal.throwIfAborted();
        throw new Error(`Build the simulator .app for the configured bundle identifier ${app.bundleId}.`);
    }
    try {
        await execute(xcrun, ['simctl', 'install', device.id, bundle], { timeout: 30_000, maxBuffer: 64 * 1024, signal });
    } catch {
        signal.throwIfAborted();
        throw new Error(`Xcode could not install ${app.label} on ${device.name}. Choose a simulator build for that runtime.`);
    }
}

export function launchSimulatorApp(device: IosSimulator, app: IosSimulatorApp, signal: AbortSignal) {
    signal.throwIfAborted();
    const child = spawn(xcrun, ['simctl', 'launch', '--console', '--terminate-running-process', device.id, app.bundleId, ...app.args], {
        detached: true, stdio: ['ignore', 'pipe', 'pipe'],
        env: { ...process.env, SIMCTL_CHILD_FERROUI_EDITOR_SOURCE: '1' },
    });
    const consoleProcess = new OwnedProcess(child);
    const endpoint = awaitEndpoint([child.stdout, child.stderr], consoleProcess, signal);
    return { process: consoleProcess, endpoint };
}

function awaitEndpoint(streams: NodeJS.ReadableStream[], process: OwnedProcess, signal: AbortSignal): Promise<RuntimeMcpEndpoint> {
    const deadline = AbortSignal.any([signal, AbortSignal.timeout(20_000)]);
    return new Promise((resolve, reject) => {
        const listeners = streams.map(stream => {
            stream.setEncoding('utf8');
            const read = runtimeEndpointReader();
            let partial = '';
            let discard = false;
            return { stream, data: (chunk: string) => {
                const lines = chunk.split('\n');
                for (const [index, fragment] of lines.entries()) {
                    if (!discard) {
                        if (partial.length + fragment.length <= 8_192) partial += fragment;
                        else discard = true;
                    }
                    if (index === lines.length - 1) break;
                    const endpoint = discard ? undefined : read(partial);
                    partial = '';
                    discard = false;
                    if (endpoint) {
                        cleanup();
                        resolve(endpoint);
                        break;
                    }
                }
            } };
        });
        const cleanup = () => {
            for (const { stream, data } of listeners) {
                stream.removeListener('data', data);
                stream.resume();
            }
            deadline.removeEventListener('abort', aborted);
        };
        const fail = (error: Error) => { cleanup(); reject(error); };
        const aborted = () => fail(signal.aborted ? signal.reason
            : new Error('The simulator app did not announce its MCP inspector. Enable the inspector in the app and its launch arguments.'));
        deadline.addEventListener('abort', aborted, { once: true });
        for (const { stream, data } of listeners) stream.on('data', data);
        void process.closed.then(
            () => fail(new Error('The simulator app exited before its inspector connected. Check the bundle and launch arguments.')),
            () => fail(new Error('Xcode could not launch the simulator app.')),
        );
    });
}
