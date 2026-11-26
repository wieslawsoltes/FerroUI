import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import { AndroidBridge, androidBridgePath, androidDevices } from './androidBridge';
import { AndroidDevelopment } from './androidDevelopment';
import { openAndroidProcess, type AndroidProcess } from './androidProcess';
import { AndroidSessionDebugger } from './androidSessionDebugger';
import { androidProfiles, type AndroidApp, type AndroidMode, type AndroidRequest } from './androidTargets';
import { detachedDebugger } from './debuggerPause';
import { selectDebugConfiguration, type MobileLaunchDebug } from './reloadDebugConfiguration';
import type { ReloadHistory } from './reloadHistory';
import { ReloadLog } from './reloadLog';
import type { RuntimeConnections, RuntimeDebugger, RuntimeSummary } from './runtimeConnections';
import type { AndroidRuntimeTarget, AppRuntime } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Session {
    app: AndroidApp;
    mode: AndroidMode;
    bridge: AndroidBridge;
    stop: AbortController;
    ready?: Promise<RuntimeSummary>;
    closing?: Promise<void>;
    process?: AndroidProcess;
    development?: AndroidDevelopment;
    runtime?: AppRuntime;
    debugger?: AndroidSessionDebugger;
    output?: { channel: vscode.OutputChannel; log: ReloadLog };
}

interface DebugOptions { configuration?: string; interactive?: boolean; detach?: boolean; signal?: AbortSignal }

export class AndroidSessions implements vscode.Disposable {
    private readonly lifetime = new AbortController();
    private readonly sessions = new Map<string, Session>();
    private readonly saves = vscode.workspace.onDidSaveTextDocument(document => {
        if (document.uri.scheme !== 'file') return;
        for (const session of this.sessions.values()) session.development?.observe(document.uri.fsPath, () => document.getText());
    });
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, session] of this.sessions) {
            if (closed.has(session.app.folder)) this.retire(key, session, true);
        }
    });

    constructor(private readonly connections: RuntimeConnections, private readonly history: ReloadHistory) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        const active = this.requireWorkspace(signal);
        const profiles = androidProfiles(folder);
        const executable = await androidBridgePath(active);
        return { devices: await androidDevices(executable, active), apps: profiles.map(({ apkPath, symbolFiles, build, ...profile }) => ({
            ...profile, installsApk: apkPath !== undefined || build !== undefined, buildsApk: Boolean(build),
        })) };
    }

    async open(request: AndroidRequest, mode: AndroidMode, signal: AbortSignal, debug?: MobileLaunchDebug): Promise<RuntimeSummary> {
        const requested = this.requireWorkspace(signal);
        const apps = androidProfiles(request.folder).filter(profile => profile.id === request.profile);
        if (apps.length !== 1) throw new Error('Choose an unambiguous Android app profile.');
        const [app] = apps;
        const [folder] = workspaceFolders(app.folder);
        if (debug && app.build && !app.build.debugger) throw new Error('Enable build.debugger in the Android app profile before running with a debugger.');
        const configuration = debug && await selectDebugConfiguration(folder, { interactive: false, ...debug, target: 'android' });
        const timeout = AbortSignal.timeout(30_000);
        const active = AbortSignal.any([requested, timeout]);
        active.throwIfAborted();
        const executable = await androidBridgePath(active);
        const device = (await androidDevices(executable, active)).find(device => device.id === request.device);
        if (device?.status !== 'device') throw new Error('Choose an Android device authorized in ADB.');
        active.throwIfAborted();

        const key = `${device.id}:${app.packageId}`;
        const existing = this.sessions.get(key);
        if (existing) {
            const prior = existing.app;
            const same = prior.folder === app.folder && prior.id === app.id && prior.activity === app.activity && prior.apkPath === app.apkPath
                && JSON.stringify(prior.build) === JSON.stringify(app.build)
                && prior.symbolFiles.length === app.symbolFiles.length
                && prior.symbolFiles.every((file, index) => file === app.symbolFiles[index]);
            const ready = existing.runtime && this.connections.describe(existing.runtime);
            if (same && ready && existing.mode === mode && !existing.stop.signal.aborted) {
                if (configuration) {
                    this.debugger(existing).prepare(folder, configuration);
                    this.resumeDebugger(existing);
                }
                return ready;
            }
            throw new Error('Close the current Android session for this package and device before starting another.');
        }

        const session: Session = { app, mode, bridge: new AndroidBridge(executable, device), stop: new AbortController() };
        this.sessions.set(key, session);
        try {
            if (configuration) this.debugger(session).prepare(folder, configuration);
            if (mode === 'run' && app.build) {
                const runtime = this.connections.beginAndroid({ ...this.target(session), development: true }, this.label(session),
                    this.debuggerControl(session));
                session.runtime = runtime.runtime;
                session.development = new AndroidDevelopment(app, session.bridge, this.connections, runtime.runtime,
                    this.history, this.output(session).log, session.stop.signal, this.debugger(session));
                session.development.rebuild();
                void this.watch(key, session);
                return runtime;
            }
            session.ready = this.connect(session, AbortSignal.any([active, session.stop.signal]));
            const runtime = await session.ready;
            this.resumeDebugger(session);
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

    rebuild(runtime: AppRuntime): RuntimeSummary {
        this.requireWorkspace(this.lifetime.signal);
        const session = this.session(runtime);
        if (!session.development) throw new Error('Choose a source-built Android app session.');
        session.development.rebuild();
        return this.connections.describe(runtime)!;
    }

    recentOutput(runtime: AppRuntime, lines: number): string {
        this.requireWorkspace(this.lifetime.signal);
        return this.session(runtime).output?.log.tail(lines) ?? '';
    }

    async debug(runtime: AppRuntime, generation: number, options: DebugOptions = {}) {
        this.requireWorkspace(options.signal ?? this.lifetime.signal);
        const session = this.session(runtime);
        const process = this.process(session);
        const accepted = this.connections.session(runtime);
        if (!process || accepted.generation !== generation) throw new Error('Use the current accepted Android generation.');
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
        if (!accepted.current() || session.development?.deploying) throw new Error('The Android app connection is changing. Read its current session.');
        await this.debugger(session).attach(folder, configuration, process.receipt,
            session.development?.symbols ?? session.app.symbolFiles, signal);
        if (!accepted.current()) throw new Error('The Android app connection changed while attaching the debugger.');
        return { runtime, generation, pid: process.receipt.pid, attached: true };
    }

    showOutput(runtime: AppRuntime): void { this.output(this.session(runtime)).channel.show(); }

    private debugger(session: Session): AndroidSessionDebugger {
        return session.debugger ??= new AndroidSessionDebugger(session.bridge, session.app.packageId, this.output(session).log,
            () => this.connections.notifyDebuggerChanged(session.runtime));
    }

    private resumeDebugger(session: Session): void {
        const process = this.process(session);
        if (process && !session.development?.deploying) {
            session.debugger?.resume(process.receipt, session.development?.symbols ?? session.app.symbolFiles, session.stop.signal);
        }
    }

    private output(session: Session) {
        if (!session.output) {
            const channel = vscode.window.createOutputChannel(`FerroUI Android · ${this.label(session)}`);
            session.output = { channel, log: new ReloadLog(channel) };
        }
        return session.output;
    }

    private session(runtime: AppRuntime): Session {
        const session = [...this.sessions.values()].find(item => item.runtime === runtime);
        if (!session) throw new Error('Choose a current Android app session.');
        return session;
    }

    private process(session: Session): AndroidProcess | undefined { return session.development?.current ?? session.process; }

    private debuggerStatus(session: Session) {
        const receipt = this.process(session)?.receipt;
        if (receipt?.debuggerPath) return session.debugger?.statusFor(receipt) ?? detachedDebugger;
    }

    private debuggerControl(session: Session): RuntimeDebugger {
        return {
            statusFor: () => this.debuggerStatus(session),
            continueStopped: signal => session.debugger?.continueStopped(signal),
        };
    }

    private label(session: Session): string { return `${session.app.label} · ${session.bridge.device.name}`; }

    private target(session: Session): AndroidRuntimeTarget {
        const { app, bridge: { device }, mode } = session;
        const receipt = this.process(session)?.receipt;
        return { device: device.id, name: device.name, emulator: device.emulator, profile: app.id, packageId: app.packageId,
            launched: mode === 'run', pid: receipt?.pid, debuggable: Boolean(receipt?.debuggerPath) };
    }

    private async connect(session: Session, signal: AbortSignal): Promise<RuntimeSummary> {
        const process = session.process = await openAndroidProcess(session.app, session.bridge, session.mode, signal);
        workspaceFolders(session.app.folder);
        signal.throwIfAborted();
        const runtime = this.connections.addPlatform({ kind: 'android', android: this.target(session) },
            this.label(session), process.forward.endpoint, this.debuggerControl(session));
        session.runtime = runtime.runtime;
        return runtime;
    }

    private async watch(key: string, session: Session): Promise<void> {
        const signal = session.stop.signal;
        try {
            for (;;) {
                await delay(2_000, undefined, { signal });
                const current = this.process(session);
                if (!current || session.development?.deploying) continue;
                const pid = await session.bridge.pid(session.app.packageId, signal);
                if (session.development?.deploying || this.process(session) !== current || pid === current.receipt.pid) continue;
                if (session.development) {
                    await session.debugger?.close();
                    await session.development.exited(current);
                } else {
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
                if (session.development) await session.development.close(terminate);
                else await session.process?.close(terminate);
            } finally {
                session.output?.channel.dispose();
                if (this.sessions.get(key) === session) this.sessions.delete(key);
            }
        })();
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
    }

    dispose(): void { this.folders.dispose(); this.saves.dispose(); void this.shutdown(); }
}
