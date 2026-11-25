import * as vscode from 'vscode';
import type { IosAppRun } from './iosAppProfiles';
import type { IosBundle } from './iosBuild';
import { IosDeviceBridge, type IosDevice } from './iosDeviceBridge';
import { iosDeviceDevelopment } from './iosDeviceDevelopment';
import { launchIosDeviceApp, type IosDeviceProcess } from './iosDeviceProcess';
import { iosDeviceApps, type IosDeviceApp } from './iosDeviceTargets';
import type { MobileDevelopment } from './mobileDevelopment';
import { iosDeviceAttachCommands, iosDeviceVariable, selectDebugConfiguration, type MobileLaunchDebug } from './reloadDebugConfiguration';
import { ReloadDebugger } from './reloadDebugger';
import type { ReloadHistory } from './reloadHistory';
import { ReloadLog } from './reloadLog';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Launch {
    app: IosDeviceApp;
    device: IosDevice;
    stop: AbortController;
    output: ReloadLog;
    runtime?: AppRuntime;
    process?: IosDeviceProcess;
    development?: MobileDevelopment<IosBundle, IosDeviceProcess>;
    debugger?: ReloadDebugger;
    attaching?: boolean;
    starting?: Promise<RuntimeSummary>;
    closing?: Promise<void>;
}

export class IosDeviceSessions implements vscode.Disposable {
    readonly supported = process.platform === 'darwin';
    private readonly lifetime = new AbortController();
    private readonly launches = new Map<string, Launch>();
    private readonly saves = vscode.workspace.onDidSaveTextDocument(document => {
        if (document.uri.scheme !== 'file') return;
        for (const launch of this.launches.values()) launch.development?.observe(document.uri.fsPath, () => document.getText());
    });
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, launch] of this.launches) {
            if (closed.has(launch.app.folder)) this.retire(key, launch);
        }
    });

    constructor(private readonly connections: RuntimeConnections, private readonly history: ReloadHistory,
        private readonly bridge = new IosDeviceBridge()) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        if (!this.supported) return { supported: false, devices: [], apps: [] };
        this.requireWorkspace();
        const apps = iosDeviceApps(folder).map(({ id, bundleId, label, folder, bundlePath, build }) => ({
            id, bundleId, label, folder, installsBundle: bundlePath !== undefined || build !== undefined, buildsBundle: !!build,
        }));
        const devices = await this.bridge.devices(AbortSignal.any([signal, this.lifetime.signal]));
        return { supported: true, devices: devices.map(({ id, name, system, status }) => ({ id, name, system, status })), apps };
    }

    async open(request: IosAppRun, signal: AbortSignal, debug?: MobileLaunchDebug): Promise<RuntimeSummary> {
        this.requireWorkspace();
        const active = AbortSignal.any([signal, this.lifetime.signal]);
        active.throwIfAborted();
        const matches = iosDeviceApps(request.folder).filter(app => app.id === request.profile);
        if (matches.length !== 1) throw new Error('Choose an unambiguous iOS device launch profile.');
        const [app] = matches;
        const [folder] = workspaceFolders(app.folder);
        const configuration = debug && await selectDebugConfiguration(folder, { interactive: false, ...debug, target: 'iosDevice' });
        const device = (await this.bridge.devices(active)).find(item => item.id.toLowerCase() === request.device.toLowerCase());
        if (device?.status !== 'ready') throw new Error('Choose a connected, paired iPhone or iPad with Developer Mode enabled.');
        active.throwIfAborted();
        const key = `${device.id}:${app.bundleId}`;
        const existing = this.launches.get(key);
        if (existing) {
            const runtime = existing.runtime && this.connections.describe(existing.runtime);
            if (!runtime || existing.closing || JSON.stringify(existing.app) !== JSON.stringify(app)) {
                throw new Error('Close the current iOS session for this bundle and device before starting another.');
            }
            if (existing.development && (runtime.generation === null || existing.development.deploying)) {
                if (configuration) this.debuggerSession(existing).prepareIfIdle(folder, this.configuration(existing, configuration));
                return runtime;
            }
            if (configuration) await this.debug(runtime.runtime, runtime.generation!, { configuration, signal: active });
            return this.current(runtime.runtime, runtime.generation!);
        }
        const label = `${app.label} · ${device.name}`;
        const output = new ReloadLog(vscode.window.createOutputChannel(`FerroUI iOS Device · ${label}`));
        const launch: Launch = { app, device, stop: new AbortController(), output };
        this.launches.set(key, launch);
        try {
            if (app.build) {
                if (configuration) this.debuggerSession(launch).prepareIfIdle(folder, this.configuration(launch, configuration));
                return this.startDevelopment(launch, label);
            }
            launch.starting = this.start(key, launch, label, AbortSignal.any([active, launch.stop.signal]));
            const runtime = await launch.starting;
            if (configuration) await this.debug(runtime.runtime, runtime.generation!, { configuration, signal: active });
            return this.current(runtime.runtime, runtime.generation!);
        } catch (error) {
            await this.close(key, launch).catch(problem => console.warn(problem));
            throw error;
        }
    }

    private async start(key: string, launch: Launch, label: string, signal: AbortSignal): Promise<RuntimeSummary> {
        const { app, device, output } = launch;
        if (app.bundlePath) output.appendLine(`Installing ${app.label}`);
        await this.bridge.install(device, app, signal);
        output.appendLine(`Starting ${app.label}`);
        const process = launch.process = await launchIosDeviceApp(this.bridge.executable, device, app, output, signal);
        signal.throwIfAborted();
        const runtime = this.connections.addPlatform({ kind: 'ios-device', iosDevice: { ...this.target(launch), pid: process.pid } },
            label, process.endpoint, generation => generation !== undefined && launch.debugger?.isAttachedTo(generation) === true);
        launch.runtime = runtime.runtime;
        const ended = () => this.retire(key, launch);
        void process.console.closed.then(ended, ended);
        output.appendLine(`${app.label} · PID ${process.pid} · inspector ready`);
        return runtime;
    }

    private startDevelopment(launch: Launch, label: string): RuntimeSummary {
        const pending = this.connections.beginIosDevice({ ...this.target(launch), development: true }, label,
            generation => generation !== undefined && launch.debugger?.isAttachedTo(generation) === true);
        const runtime = launch.runtime = pending.runtime;
        launch.development = iosDeviceDevelopment({
            app: launch.app, device: launch.device, bridge: this.bridge, connections: this.connections, runtime,
            history: this.history, output: launch.output, cancellation: launch.stop.signal, debugger: () => launch.debugger,
        });
        launch.development.rebuild();
        return pending;
    }

    private target({ app, device }: Launch) {
        return { device: device.id, name: device.name, system: device.system, profile: app.id, bundleId: app.bundleId };
    }

    async debug(runtime: AppRuntime, generation: number, options: {
        configuration?: string | vscode.DebugConfiguration; interactive?: boolean; detach?: boolean; signal?: AbortSignal
    }) {
        this.requireWorkspace();
        const [, launch] = this.lookup(runtime);
        this.current(runtime, generation);
        if (launch.attaching) throw new Error('Wait for the current iOS device debugger operation to finish.');
        const signal = options.signal ? AbortSignal.any([launch.stop.signal, options.signal]) : launch.stop.signal;
        signal.throwIfAborted();
        launch.attaching = true;
        try {
            if (options.detach) await launch.debugger?.detach();
            else {
                const [folder] = workspaceFolders(launch.app.folder);
                const configuration = typeof options.configuration === 'object' ? options.configuration
                    : await selectDebugConfiguration(folder, { name: options.configuration, interactive: options.interactive ?? false, target: 'iosDevice' });
                if (configuration) await this.attachDebugger(launch, configuration, generation, signal);
            }
            signal.throwIfAborted();
            const current = this.current(runtime, generation);
            return { runtime, generation, pid: current.pid, attached: current.debuggerAttached === true };
        } finally {
            launch.attaching = false;
        }
    }

    private async attachDebugger(launch: Launch, configuration: vscode.DebugConfiguration, generation: number, signal?: AbortSignal): Promise<void> {
        signal?.throwIfAborted();
        launch.stop.signal.throwIfAborted();
        const { runtime, development } = launch;
        const process = development?.current ?? launch.process;
        if (!runtime || !process || process.console.hasExited || development?.deploying || this.connections.revision(runtime) !== generation) {
            throw new Error('Use the current accepted iOS device process.');
        }
        const [folder] = workspaceFolders(launch.app.folder);
        const debuggerSession = this.debuggerSession(launch);
        if (debuggerSession.follows(generation)) return;
        await debuggerSession.attach(folder, this.configuration(launch, configuration), process.pid, generation);
        if (signal?.aborted || this.connections.revision(runtime) !== generation) {
            await debuggerSession.detach();
            signal?.throwIfAborted();
            throw new Error('The iOS device session changed while attaching the debugger.');
        }
    }

    private debuggerSession(launch: Launch): ReloadDebugger {
        return launch.debugger ??= new ReloadDebugger(launch.output, {
            attachError: 'Xcode LLDB could not attach to the iOS device app.',
            onDidChange: () => this.connections.notifyDebuggerChanged(launch.runtime),
            programForGeneration: generation => launch.runtime && this.connections.revision(launch.runtime) === generation
                ? launch.development?.current?.programPath : undefined,
        });
    }

    private configuration(launch: Launch, configuration: vscode.DebugConfiguration): vscode.DebugConfiguration {
        const attachCommands = iosDeviceAttachCommands.map(command => command.replaceAll(iosDeviceVariable, launch.device.id));
        return { ...configuration, attachCommands };
    }

    rebuild(runtime: AppRuntime): RuntimeSummary {
        this.requireWorkspace();
        const [, launch] = this.lookup(runtime);
        if (!launch.development) throw new Error('Choose a source-built iOS device app session.');
        launch.development.rebuild();
        return this.connections.describe(runtime)!;
    }

    recentOutput(runtime: AppRuntime, lines: number): string { return this.lookup(runtime)[1].output.tail(lines); }
    showOutput(runtime: AppRuntime): void { this.lookup(runtime)[1].output.show(); }
    stop(runtime: AppRuntime): Promise<void> { return this.close(...this.lookup(runtime)); }

    private current(runtime: AppRuntime, generation: number): RuntimeSummary {
        const current = this.connections.describe(runtime);
        if (current?.generation !== generation) throw new Error('Use the current accepted iOS device generation.');
        return current;
    }

    private lookup(runtime: AppRuntime): [string, Launch] {
        const launch = [...this.launches].find(([, candidate]) => candidate.runtime === runtime);
        if (!launch || launch[1].closing) throw new Error('Choose a current iOS device session.');
        return launch;
    }

    private retire(key: string, launch: Launch): void { void this.close(key, launch).catch(error => console.warn(error)); }

    private close(key: string, launch: Launch): Promise<void> {
        if (launch.closing) return launch.closing;
        launch.stop.abort(new Error('The iOS device app session closed.'));
        if (launch.runtime) this.connections.removePlatform(launch.runtime);
        return launch.closing = (async () => {
            try {
                await launch.starting?.catch(() => {});
                await launch.debugger?.close();
            } finally {
                try {
                    if (launch.development) await launch.development.close(true);
                    else await launch.process?.close();
                }
                finally {
                    launch.output.dispose();
                    if (this.launches.get(key) === launch) this.launches.delete(key);
                }
            }
        })();
    }

    private requireWorkspace(): void {
        if (!this.supported) throw new Error('iOS device sessions require a macOS workspace host.');
        if (!vscode.workspace.isTrusted || !workspaceFolders().length) throw new Error('Open a trusted workspace on the Mac running Xcode.');
        this.lifetime.signal.throwIfAborted();
    }

    async shutdown(): Promise<void> {
        this.lifetime.abort();
        await Promise.allSettled([...this.launches].map(([key, launch]) => this.close(key, launch)));
    }

    dispose(): void { this.saves.dispose(); this.folders.dispose(); void this.shutdown(); }
}
