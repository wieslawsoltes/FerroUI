import * as vscode from 'vscode';
import { iosAppProfiles, type IosAppProfile, type IosAppRun } from './iosAppProfiles';
import { IosDeviceBridge, type IosDevice } from './iosDeviceBridge';
import { launchIosDeviceApp, type IosDeviceProcess } from './iosDeviceProcess';
import { iosDeviceAttachCommands, iosDeviceVariable, selectDebugConfiguration, type MobileLaunchDebug } from './reloadDebugConfiguration';
import { ReloadDebugger } from './reloadDebugger';
import { ReloadLog } from './reloadLog';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Launch {
    app: IosAppProfile;
    device: IosDevice;
    stop: AbortController;
    output: ReloadLog;
    runtime?: AppRuntime;
    process?: IosDeviceProcess;
    debugger?: ReloadDebugger;
    attaching?: boolean;
    starting?: Promise<RuntimeSummary>;
    closing?: Promise<void>;
}

export function hasIosDeviceProfiles(): boolean {
    return workspaceFolders().some(folder => {
        const profiles = vscode.workspace.getConfiguration('ferroui.iosDevice', folder.uri).get<unknown>('apps');
        return Array.isArray(profiles) && profiles.length > 0;
    });
}

export class IosDeviceSessions implements vscode.Disposable {
    readonly supported = process.platform === 'darwin';
    private readonly lifetime = new AbortController();
    private readonly launches = new Map<string, Launch>();
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, launch] of this.launches) {
            if (closed.has(launch.app.folder)) this.retire(key, launch);
        }
    });

    constructor(private readonly connections: RuntimeConnections, private readonly bridge = new IosDeviceBridge()) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        if (!this.supported) return { supported: false, devices: [], apps: [] };
        this.requireWorkspace();
        const apps = iosAppProfiles('iosDevice', folder).map(({ id, bundleId, label, folder, bundlePath }) => ({
            id, bundleId, label, folder, installsBundle: bundlePath !== undefined,
        }));
        const devices = await this.bridge.devices(AbortSignal.any([signal, this.lifetime.signal]));
        return { supported: true, devices, apps };
    }

    async open(request: IosAppRun, signal: AbortSignal, debug?: MobileLaunchDebug): Promise<RuntimeSummary> {
        this.requireWorkspace();
        const active = AbortSignal.any([signal, this.lifetime.signal]);
        active.throwIfAborted();
        const matches = iosAppProfiles('iosDevice', request.folder).filter(app => app.id === request.profile);
        if (matches.length !== 1) throw new Error('Choose an unambiguous iOS device launch profile.');
        const [app] = matches;
        if (app.build !== undefined) throw new Error('Choose a development-signed app bundle or a development app already installed on the iOS device.');
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
            if (configuration) await this.debug(runtime.runtime, runtime.generation!, { configuration, signal: active });
            return this.current(runtime.runtime, runtime.generation!);
        }
        const label = `${app.label} · ${device.name}`;
        const output = new ReloadLog(vscode.window.createOutputChannel(`FerroUI iOS Device · ${label}`));
        const launch: Launch = { app, device, stop: new AbortController(), output };
        this.launches.set(key, launch);
        launch.starting = this.start(key, launch, label, AbortSignal.any([active, launch.stop.signal]));
        try {
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
        const runtime = this.connections.addPlatform({ kind: 'ios-device', iosDevice: {
            device: device.id, name: device.name, system: device.system, profile: app.id, bundleId: app.bundleId, pid: process.pid,
        } }, label, process.endpoint, generation => generation !== undefined && launch.debugger?.isAttachedTo(generation) === true);
        launch.runtime = runtime.runtime;
        const ended = () => this.retire(key, launch);
        void process.console.closed.then(ended, ended);
        output.appendLine(`${app.label} · PID ${process.pid} · inspector ready`);
        return runtime;
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
        const { runtime, process } = launch;
        if (!runtime || !process || process.console.hasExited || this.connections.revision(runtime) !== generation) {
            throw new Error('Use the current accepted iOS device process.');
        }
        const [folder] = workspaceFolders(launch.app.folder);
        const debuggerSession = launch.debugger ??= new ReloadDebugger(launch.output, {
            attachError: 'Xcode LLDB could not attach to the iOS device app.',
            onDidChange: () => this.connections.notifyDebuggerChanged(runtime),
        });
        if (debuggerSession.follows(generation)) return;
        const attachCommands = iosDeviceAttachCommands.map(command => command.replaceAll(iosDeviceVariable, launch.device.id));
        await debuggerSession.attach(folder, { ...configuration, attachCommands }, process.pid, generation);
        if (signal?.aborted || this.connections.revision(runtime) !== generation) {
            await debuggerSession.detach();
            signal?.throwIfAborted();
            throw new Error('The iOS device session changed while attaching the debugger.');
        }
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
                try { await launch.process?.close(); }
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

    dispose(): void { this.folders.dispose(); void this.shutdown(); }
}
