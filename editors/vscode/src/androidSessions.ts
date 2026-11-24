import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import { AndroidBridge, androidBridgePath, androidDevices, type AndroidForward, type AndroidReceipt } from './androidBridge';
import { AndroidDebugger } from './androidDebugger';
import { androidProfiles, type AndroidApp, type AndroidMode, type AndroidRequest } from './androidTargets';
import { selectDebugConfiguration } from './reloadDebugConfiguration';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Session {
    app: AndroidApp;
    mode: AndroidMode;
    bridge: AndroidBridge;
    stop: AbortController;
    ready?: Promise<RuntimeSummary>;
    closing?: Promise<void>;
    pid?: number;
    receipt?: AndroidReceipt;
    forward?: AndroidForward;
    runtime?: AppRuntime;
    debugger?: AndroidDebugger;
}

interface DebugOptions { configuration?: string; interactive?: boolean; detach?: boolean; signal?: AbortSignal }

export class AndroidSessions implements vscode.Disposable {
    private readonly lifetime = new AbortController();
    private readonly sessions = new Map<string, Session>();
    private debugOutput?: vscode.OutputChannel;
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, session] of this.sessions) {
            if (closed.has(session.app.folder)) this.retire(key, session, true);
        }
    });

    constructor(private readonly connections: RuntimeConnections) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        const active = this.requireWorkspace(signal);
        const profiles = androidProfiles(folder);
        const executable = await androidBridgePath(active);
        return { devices: await androidDevices(executable, active), apps: profiles.map(({ apkPath, symbolFiles, ...profile }) => ({
            ...profile, installsApk: apkPath !== undefined,
        })) };
    }

    async open(request: AndroidRequest, mode: AndroidMode, signal: AbortSignal): Promise<RuntimeSummary> {
        const timeout = AbortSignal.timeout(30_000);
        const active = this.requireWorkspace(AbortSignal.any([signal, timeout]));
        const apps = androidProfiles(request.folder).filter(profile => profile.id === request.profile);
        if (apps.length !== 1) throw new Error('Choose an unambiguous Android app profile.');
        const [app] = apps;
        const executable = await androidBridgePath(active);
        const device = (await androidDevices(executable, active)).find(device => device.id === request.device);
        if (device?.status !== 'device') throw new Error('Choose an Android device authorized in ADB.');
        active.throwIfAborted();
        workspaceFolders(app.folder);

        const key = `${device.id}:${app.packageId}`;
        const existing = this.sessions.get(key);
        if (existing) {
            const prior = existing.app;
            const same = prior.folder === app.folder && prior.id === app.id && prior.activity === app.activity && prior.apkPath === app.apkPath
                && prior.symbolFiles.length === app.symbolFiles.length
                && prior.symbolFiles.every((file, index) => file === app.symbolFiles[index]);
            const ready = existing.runtime && this.connections.describe(existing.runtime);
            if (same && ready && existing.mode === mode && !existing.stop.signal.aborted) return ready;
            throw new Error('Close the current Android session for this package and device before starting another.');
        }

        const session: Session = { app, mode, bridge: new AndroidBridge(executable, device), stop: new AbortController() };
        this.sessions.set(key, session);
        session.ready = this.connect(session, AbortSignal.any([active, session.stop.signal]));
        try {
            const runtime = await session.ready;
            void this.watch(key, session);
            return runtime;
        } catch (error) {
            await this.close(key, session, true).catch(problem => console.warn(problem));
            if (timeout.aborted && !signal.aborted && !this.lifetime.signal.aborted) {
                throw new Error('Timed out connecting the Android app. Check its debug build and Live App inspector.');
            }
            throw error;
        }
    }

    async stop(runtime: AppRuntime): Promise<void> {
        const match = [...this.sessions].find(([, session]) => session.runtime === runtime);
        if (!match) throw new Error('Choose a current Android app session.');
        await this.close(...match, true);
    }

    async debug(runtime: AppRuntime, generation: number, options: DebugOptions = {}) {
        this.requireWorkspace(options.signal ?? this.lifetime.signal);
        const session = [...this.sessions.values()].find(item => item.runtime === runtime);
        const accepted = this.connections.session(runtime);
        if (!session?.receipt || accepted.generation !== generation) throw new Error('Use the current accepted Android generation.');
        const signal = options.signal ? AbortSignal.any([session.stop.signal, options.signal]) : session.stop.signal;
        signal.throwIfAborted();
        if (options.detach) {
            await session.debugger?.close();
            return { runtime, generation, attached: false };
        }
        const [folder] = workspaceFolders(session.app.folder);
        const configuration = await selectDebugConfiguration(folder, { name: options.configuration,
            target: 'android', interactive: options.interactive ?? false });
        if (!configuration) return { runtime, generation, attached: false };
        signal.throwIfAborted();
        if (!accepted.current()) throw new Error('The Android app connection changed. Read its current session.');
        if (session.debugger) throw new Error('Stop the current Android debugger before attaching again.');
        const debuggerSession = new AndroidDebugger(session.bridge, session.app.packageId, session.receipt, this.debuggerOutput(), () => {
            if (session.debugger === debuggerSession) session.debugger = undefined;
        }, session.app.symbolFiles);
        session.debugger = debuggerSession;
        try {
            await debuggerSession.attach(folder, configuration, signal);
            if (!accepted.current()) throw new Error('The Android app connection changed while attaching the debugger.');
            return { runtime, generation, pid: session.receipt.pid, attached: true };
        } catch (error) {
            await debuggerSession.close();
            throw error;
        }
    }

    showDebugOutput(): void { this.debuggerOutput().show(); }

    private debuggerOutput() { return this.debugOutput ??= vscode.window.createOutputChannel('FerroUI Android Debugger'); }

    private async connect(session: Session, signal: AbortSignal): Promise<RuntimeSummary> {
        const { app, bridge, mode } = session;
        const { packageId } = app;
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
                // Android can start the app while the launch command is still finishing.
                const pid = await bridge.pid(packageId, AbortSignal.timeout(4_000)).catch(() => undefined);
                if (started || pid !== priorPid) session.pid = pid;
            }
        } else {
            session.pid = await bridge.pid(packageId, signal);
        }
        signal.throwIfAborted();
        if (!session.pid) throw new Error('The Android app process is unavailable on the selected device.');
        const receipt = await this.waitForReceipt(session, previousToken, signal);
        session.receipt = receipt;
        session.forward = await bridge.forward(receipt);
        signal.throwIfAborted();
        await verifyRuntime(session.forward.endpoint, signal);
        if (await bridge.pid(packageId, signal) !== receipt.pid) throw new Error('The Android app changed during connection.');
        workspaceFolders(app.folder);
        signal.throwIfAborted();
        const { device } = bridge;
        const runtime = this.connections.addPlatform({ kind: 'android', android: { device: device.id, name: device.name,
            emulator: device.emulator, profile: app.id, packageId, launched: mode === 'run', pid: receipt.pid,
            debuggable: Boolean(receipt.debuggerPath) } },
        `${app.label} · ${device.name}`, session.forward.endpoint);
        session.runtime = runtime.runtime;
        return runtime;
    }

    private async waitForReceipt(session: Session, previousToken: string | undefined, signal: AbortSignal): Promise<AndroidReceipt> {
        const limit = AbortSignal.timeout(10_000);
        const active = AbortSignal.any([limit, signal]);
        try {
            for (;;) {
                const receipt = await session.bridge.receipt(session.app.packageId, active);
                if (receipt && receipt.pid === session.pid && receipt.token !== previousToken) return receipt;
                if (session.mode === 'attach') break;
                await delay(150, undefined, { signal: active });
            }
        } catch (error) {
            signal.throwIfAborted();
            if (!limit.aborted) throw error;
        }
        throw new Error('Enable the Live App inspector in the running Android debug app.');
    }

    private async watch(key: string, session: Session): Promise<void> {
        const signal = session.stop.signal;
        try {
            for (;;) {
                await delay(2_000, undefined, { signal });
                if (await session.bridge.pid(session.app.packageId, signal) !== session.pid) {
                    this.retire(key, session, false);
                    return;
                }
            }
        } catch { if (!signal.aborted) this.retire(key, session, false); }
    }

    private retire(key: string, session: Session, terminate: boolean): void {
        void this.close(key, session, terminate).catch(problem => console.warn(problem));
    }

    private close(key: string, session: Session, terminate: boolean): Promise<void> {
        if (session.closing) return session.closing;
        session.stop.abort(new Error('The Android app session closed.'));
        if (session.runtime) this.connections.removePlatform(session.runtime);
        return session.closing = (async () => {
            await session.ready?.catch(() => {});
            await session.debugger?.close().catch(error => console.warn(error));
            try {
                if (terminate && session.mode === 'run' && session.pid) await this.stopOwnedApp(session);
            } finally {
                try { await session.forward?.close(); }
                finally { if (this.sessions.get(key) === session) this.sessions.delete(key); }
            }
        })();
    }

    private async stopOwnedApp(session: Session): Promise<void> {
        const { app, bridge, receipt, pid } = session;
        const signal = AbortSignal.timeout(4_000);
        if (await bridge.pid(app.packageId, signal) !== pid) return;
        if (receipt && (await bridge.receipt(app.packageId, signal))?.token !== receipt.token) return;
        await bridge.stop(app.packageId);
    }

    private requireWorkspace(signal: AbortSignal): AbortSignal {
        if (!vscode.workspace.isTrusted || !workspaceFolders().length) throw new Error('Open a trusted workspace on the host running ADB.');
        const active = AbortSignal.any([signal, this.lifetime.signal]);
        active.throwIfAborted();
        return active;
    }

    async shutdown(): Promise<void> {
        this.lifetime.abort();
        await Promise.allSettled([...this.sessions].map(([key, session]) => this.close(key, session, true)));
        this.debugOutput?.dispose();
        this.debugOutput = undefined;
    }

    dispose(): void { this.folders.dispose(); void this.shutdown(); }
}
