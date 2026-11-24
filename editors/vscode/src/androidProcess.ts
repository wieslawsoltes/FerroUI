import { setTimeout as delay } from 'node:timers/promises';
import type { AndroidBridge, AndroidForward, AndroidReceipt } from './androidBridge';
import type { AndroidApp, AndroidMode } from './androidTargets';
import { verifyRuntime } from './runtimeReady';

export interface AndroidProcess {
    receipt: AndroidReceipt;
    forward: AndroidForward;
    close(terminate: boolean): Promise<void>;
}

export async function openAndroidProcess(app: AndroidApp, bridge: AndroidBridge, mode: AndroidMode, signal: AbortSignal): Promise<AndroidProcess> {
    const { packageId } = app;
    let pid: number | undefined;
    let receipt: AndroidReceipt | undefined;
    let forward: AndroidForward | undefined;
    const close = async (terminate: boolean) => {
        try {
            if (terminate && mode === 'run' && pid) {
                const check = AbortSignal.timeout(4_000);
                const owned = receipt ? await bridge.matches(packageId, receipt, check) : await bridge.pid(packageId, check) === pid;
                if (owned) await bridge.stop(packageId);
            }
        } finally { await forward?.close(); }
    };
    try {
        if (mode === 'run') await bridge.install(app, signal);
        await bridge.checkDebuggable(packageId, signal);
        let previousToken: string | undefined;
        if (mode === 'run') {
            const [priorPid, priorReceipt] = await Promise.all([bridge.pid(packageId, signal), bridge.receipt(packageId, signal)]);
            previousToken = priorReceipt?.token;
            signal.throwIfAborted();
            let started = false;
            try {
                await bridge.launch(app);
                started = true;
            } finally {
                // The activity can start before ADB finishes its launch command.
                const launched = await bridge.pid(packageId, AbortSignal.timeout(4_000)).catch(() => undefined);
                if (started || launched !== priorPid) pid = launched;
            }
        } else {
            pid = await bridge.pid(packageId, signal);
        }
        signal.throwIfAborted();
        if (!pid) throw new Error('The Android app process is unavailable on the selected device.');
        receipt = await waitForReceipt(bridge, packageId, pid, previousToken, mode, signal);
        forward = await bridge.forward(receipt);
        signal.throwIfAborted();
        await verifyRuntime(forward.endpoint, signal);
        if (!await bridge.matches(packageId, receipt, signal)) throw new Error('The Android app changed during connection.');
        signal.throwIfAborted();
        return { receipt, forward, close };
    } catch (error) {
        await close(true).catch(problem => console.warn(problem));
        throw error;
    }
}

async function waitForReceipt(bridge: AndroidBridge, packageId: string, pid: number, previousToken: string | undefined,
    mode: AndroidMode, signal: AbortSignal): Promise<AndroidReceipt> {
    const limit = AbortSignal.timeout(10_000);
    const active = AbortSignal.any([limit, signal]);
    try {
        for (;;) {
            const receipt = await bridge.receipt(packageId, active);
            if (receipt && receipt.pid === pid && receipt.token !== previousToken) return receipt;
            if (mode === 'attach') break;
            await delay(150, undefined, { signal: active });
        }
    } catch (error) {
        signal.throwIfAborted();
        if (!limit.aborted) throw error;
    }
    throw new Error('Enable the Live App inspector in the running Android debug app.');
}
