import * as vscode from 'vscode';
import { detachedDebugger } from './debuggerPause';
import type { IosAppRun } from './iosAppProfiles';
import type { IosBundle } from './iosBuild';
import { attachIosDeviceInspector, iosInspectorCredentials, watchIosDeviceInspector, type IosDeviceAttachment, type IosDeviceAttachRequest } from './iosDeviceAttachment';
import { IosDeviceBridge, type IosDevice } from './iosDeviceBridge';
import { iosDeviceDevelopment } from './iosDeviceDevelopment';
import { launchIosDeviceApp, type IosDeviceProcess } from './iosDeviceProcess';
import { iosDeviceApp, iosDeviceApps, type IosDeviceApp } from './iosDeviceTargets';
import { iosUsbTool } from './iosDeviceUsb';
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
    attachment?: IosDeviceAttachment;
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
        const app = iosDeviceApp(request);
        const [folder] = workspaceFolders(app.folder);
        const configuration = debug && await selectDebugConfiguration(folder, { interactive: false, ...debug, target: 'iosDevice' });
        const device = await this.readyDevice(request.device, active);
        const key = `${device.id}:${app.bundleId}`;
        const existing = this.launches.get(key);
        if (existing) {
            const runtime = existing.runtime && this.connections.describe(existing.runtime);
            if (!runtime || existing.closing || existing.attachment || JSON.stringify(existing.app) !== JSON.stringify(app)) {
                throw new Error('Close the current iOS session for this bundle and device before starting another.');
            }
            if (existing.development && (runtime.generation === null || existing.development.deploying)) {
                if (configuration) this.debuggerSession(existing).prepareIfIdle(folder, this.configuration(existing, configuration));
                return runtime;
            }
            if (configuration) await this.debug(runtime.runtime, runtime.generation!, { configuration, signal: active });
            return this.current(runtime.runtime, runtime.generation!);
        }
        const launch = this.createLaunch(key, app, device);
        try {
            if (app.build) {
                if (configuration) this.debuggerSession(launch).prepareIfIdle(folder, this.configuration(launch, configuration));
                return this.startDevelopment(launch, this.label(launch));
            }
            launch.starting = this.start(key, launch, AbortSignal.any([active, launch.stop.signal]));
            const runtime = await launch.starting;
            if (configuration) await this.debug(runtime.runtime, runtime.generation!, { configuration, signal: active });
            return this.current(runtime.runtime, runtime.generation!);
        } catch (error) {
            await this.close(key, launch).catch(problem => console.warn(problem));
            throw error;
        }
    }

    async attach(request: IosDeviceAttachRequest, signal: AbortSignal): Promise<RuntimeSummary> {
        this.requireWorkspace();
        if (!iosInspectorCredentials(request.port, request.token)) throw new Error('Enter the app’s MCP port and 64-digit hexadecimal bearer token.');
        const active = AbortSignal.any([signal, this.lifetime.signal]);
        const app = iosDeviceApp(request);
        const device = await this.readyDevice(request.device, active);
        const key = `${device.id}:${app.bundleId}`;
        const existing = this.launches.get(key);
        if (existing) {
            const runtime = existing.runtime && this.connections.describe(existing.runtime);
            if (!runtime?.generation || existing.closing || existing.app.folder !== app.folder || existing.app.id !== app.id
                || !existing.attachment?.matches(request.port, request.token)) {
                throw new Error('Close the current iOS session for this bundle and device before attaching another.');
            }
            const attached = AbortSignal.any([active, existing.stop.signal]);
            await existing.attachment.verify(attached);
            attached.throwIfAborted();
            return this.current(runtime.runtime, runtime.generation);
        }
        const launch = this.createLaunch(key, app, device);
        launch.starting = this.startAttachment(key, launch, request, AbortSignal.any([active, launch.stop.signal]));
        try { return await launch.starting; }
        catch (error) {
            await this.close(key, launch).catch(problem => console.warn(problem));
            throw error;
        }
    }

    private async startAttachment(key: string, launch: Launch, request: IosDeviceAttachRequest, signal: AbortSignal): Promise<RuntimeSummary> {
        const configured = vscode.workspace.getConfiguration('ferroui.iosDevice').get<string>('inetcatPath', '');
        const executable = await iosUsbTool(configured, signal);
        const attachment = launch.attachment = await attachIosDeviceInspector(this.bridge, launch.device, launch.app, request, executable, signal);
        signal.throwIfAborted();
        const runtime = this.publish(launch, attachment.pid, attachment.endpoint, true);
        launch.output.appendLine(`${launch.app.label} · PID ${attachment.pid} · connected over USB`);
        const active = launch.stop.signal;
        void watchIosDeviceInspector(attachment, active).then(() => {
            if (!active.aborted) this.retire(key, launch);
        }, error => {
            if (!active.aborted) {
                launch.output.appendLine(String(error));
                this.retire(key, launch);
            }
        });
        return runtime;
    }

    private async start(key: string, launch: Launch, signal: AbortSignal): Promise<RuntimeSummary> {
        const { app, device, output } = launch;
        if (app.bundlePath) output.appendLine(`Installing ${app.label}`);
        await this.bridge.install(device, app, signal);
        output.appendLine(`Starting ${app.label}`);
        const process = launch.process = await launchIosDeviceApp(this.bridge.executable, device, app, output, signal);
        signal.throwIfAborted();
        const runtime = this.publish(launch, process.pid, process.endpoint);
        const ended = () => this.retire(key, launch);
        void process.console.closed.then(ended, ended);
        output.appendLine(`${app.label} · PID ${process.pid} · inspector ready`);
        return runtime;
    }

    private publish(launch: Launch, pid: number, endpoint: IosDeviceProcess['endpoint'], attached?: true): RuntimeSummary {
        const runtime = this.connections.addPlatform({ kind: 'ios-device', iosDevice: { ...this.target(launch), pid, attached } },
            this.label(launch), endpoint, generation => launch.debugger?.statusFor(generation) ?? detachedDebugger);
        launch.runtime = runtime.runtime;
        return runtime;
    }

    private startDevelopment(launch: Launch, label: string): RuntimeSummary {
        const pending = this.connections.beginIosDevice({ ...this.target(launch), development: true }, label,
            generation => launch.debugger?.statusFor(generation) ?? detachedDebugger);
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

    private label({ app, device }: Launch): string { return `${app.label} · ${device.name}`; }

    private createLaunch(key: string, app: IosDeviceApp, device: IosDevice): Launch {
        const output = new ReloadLog(vscode.window.createOutputChannel(`FerroUI iOS Device · ${app.label} · ${device.name}`));
        const launch = { app, device, stop: new AbortController(), output };
        this.launches.set(key, launch);
        return launch;
    }

    private async readyDevice(id: string, signal: AbortSignal): Promise<IosDevice> {
        const device = (await this.bridge.devices(signal)).find(item => item.id.toLowerCase() === id.toLowerCase());
        signal.throwIfAborted();
        if (device?.status !== 'ready') throw new Error('Choose a connected, paired iPhone or iPad with Developer Mode enabled.');
        return device;
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
        const { runtime, development, attachment } = launch;
        const process = development?.current ?? launch.process;
        const pid = attachment?.pid ?? process?.pid;
        if (!runtime || !pid || process?.console.hasExited || development?.deploying || this.connections.revision(runtime) !== generation) {
            throw new Error('Use the current accepted iOS device process.');
        }
        const [folder] = workspaceFolders(launch.app.folder);
        const debuggerSession = this.debuggerSession(launch);
        if (debuggerSession.follows(generation)) return;
        if (attachment) {
            await attachment.verify(signal ?? launch.stop.signal);
            signal?.throwIfAborted();
            this.current(runtime, generation);
        }
        await debuggerSession.attach(folder, this.configuration(launch, configuration), pid, generation);
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
                    else if (launch.attachment) await launch.attachment.close();
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
