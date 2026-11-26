import { setTimeout as delay } from 'node:timers/promises';
import type { IosAppProfile, IosAppRun } from './iosAppProfiles';
import type { IosDevice, IosDeviceBridge } from './iosDeviceBridge';
import { IosUsbStreams } from './iosDeviceUsb';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeMcpEndpoint } from './runtimeMcp';
import { verifyRuntime } from './runtimeReady';

export interface IosDeviceAttachRequest extends IosAppRun { port: number; token: string }

export interface IosDeviceAttachment {
    pid: number;
    endpoint: RuntimeMcpEndpoint;
    matches(port: number, token: string): boolean;
    processIsCurrent(signal: AbortSignal): Promise<boolean>;
    verify(signal: AbortSignal): Promise<void>;
    close(): Promise<void>;
}

export function iosInspectorCredentials(port: unknown, token: unknown): boolean {
    return typeof port === 'number' && Number.isInteger(port) && port > 0 && port <= 65_535
        && typeof token === 'string' && /^[0-9a-f]{64}$/i.test(token);
}

export async function attachIosDeviceInspector(bridge: Pick<IosDeviceBridge, 'appProcesses'>, device: IosDevice, app: IosAppProfile,
    { port, token }: Pick<IosDeviceAttachRequest, 'port' | 'token'>, executable: string, signal: AbortSignal): Promise<IosDeviceAttachment> {
    if (!iosInspectorCredentials(port, token)) throw new Error('Enter the app’s MCP port and 64-digit hexadecimal bearer token.');
    if (!device.udid) throw new Error('Xcode did not identify this iOS device for a USB connection.');
    const processes = await bridge.appProcesses(device, app.bundleId, signal);
    if (processes.length !== 1) throw new Error('Start the selected app independently and leave its inspector running.');
    const [pid] = processes;
    const usb = new IosUsbStreams(executable, device.udid, port);
    const processIsCurrent = async (signal: AbortSignal) => {
        const found = await bridge.appProcesses(device, app.bundleId, signal);
        return found.length === 1 && found[0] === pid;
    };
    try {
        const candidate: RuntimeMcpEndpoint = {
            url: `http://127.0.0.1:${port}/mcp`, token, openStream: usb.open, expectedProcess: { pid },
        };
        const identity = await verifyRuntime(candidate, signal);
        if (!identity || !await processIsCurrent(signal)) throw new Error('The iOS device app changed while connecting.');
        signal.throwIfAborted();
        const endpoint = { ...candidate, expectedProcess: identity };
        const verify = async (signal: AbortSignal) => {
            if (!await processIsCurrent(signal)) throw new Error('The iOS app process has changed. Attach again with its current inspector.');
            const client = new RuntimeClient(endpoint);
            const abort = () => client.dispose();
            signal.addEventListener('abort', abort, { once: true });
            try {
                signal.throwIfAborted();
                await client.tools(signal);
                signal.throwIfAborted();
            } finally {
                signal.removeEventListener('abort', abort);
                client.dispose();
            }
        };
        return {
            pid, endpoint, processIsCurrent, verify,
            matches: (askedPort, askedToken) => askedPort === port && askedToken === token,
            close: () => usb.close(),
        };
    } catch (error) {
        await usb.close();
        signal.throwIfAborted();
        throw error;
    }
}

export async function watchIosDeviceInspector(attachment: IosDeviceAttachment, signal: AbortSignal, interval = 5000): Promise<void> {
    let failures = 0;
    try {
        while (!signal.aborted) {
            await delay(interval, undefined, { signal });
            let current: boolean;
            try {
                current = await attachment.processIsCurrent(signal);
            } catch {
                if (++failures === 2) return;
                continue;
            }
            if (!current) return;
            failures = 0;
        }
    } catch (error) {
        if (!signal.aborted) throw error;
    }
}
