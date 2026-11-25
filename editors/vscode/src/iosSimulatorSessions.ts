import * as vscode from 'vscode';
import { buildSimulatorApp, type SimulatorBundle } from './iosSimulatorBuild';
import { simulatorAppIsRunning, simulatorAppPid } from './iosSimulatorDebug';
import { deploySimulatorApp, installSimulatorApp, iosSimulators, launchSimulatorApp, type ManagedSimulatorProcess } from './iosSimulatorProcess';
import { iosSimulatorApps, type IosSimulator, type IosSimulatorApp, type IosSimulatorRun } from './iosSimulatorTargets';
import { MobileDevelopment } from './mobileDevelopment';
import type { OwnedProcess } from './ownedProcess';
import { errorMessage } from './protocol';
import { selectDebugConfiguration, type MobileLaunchDebug } from './reloadDebugConfiguration';
import { ReloadDebugger } from './reloadDebugger';
import type { ReloadHistory } from './reloadHistory';
import { ReloadLog } from './reloadLog';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime, IosRuntimeTarget } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Launch {
    app: IosSimulatorApp;
    device: IosSimulator;
    cancellation: AbortController;
    process?: OwnedProcess;
    runtime?: AppRuntime;
    debugger?: ReloadDebugger;
    development?: MobileDevelopment<SimulatorBundle, ManagedSimulatorProcess>;
    output?: { channel: vscode.OutputChannel; log: ReloadLog };
    attaching?: boolean;
    stopping?: Promise<void>;
}

interface DebugOptions { configuration?: string; interactive?: boolean; detach?: boolean; signal?: AbortSignal }

export class IosSimulatorSessions implements vscode.Disposable {
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

    constructor(private readonly connections: RuntimeConnections, private readonly history: ReloadHistory) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        if (!this.supported) return { supported: false, devices: [], apps: [] };
        this.requireWorkspace();
        const profiles = iosSimulatorApps(folder);
        const devices = await iosSimulators(AbortSignal.any([signal, this.lifetime.signal]));
        return { supported: true, devices, apps: profiles.map(({ id, bundleId, label, folder, bundlePath, build }) => ({
            id, bundleId, label, folder, installsBundle: bundlePath !== undefined || build !== undefined, buildsBundle: Boolean(build),
        })) };
    }

    async start(request: IosSimulatorRun, signal: AbortSignal, debug?: MobileLaunchDebug): Promise<RuntimeSummary> {
        this.requireWorkspace();
        signal.throwIfAborted();
        const apps = iosSimulatorApps(request.folder).filter(app => app.id === request.profile);
        const [app] = apps;
        if (apps.length !== 1) throw new Error('Choose an unambiguous iOS simulator launch profile.');
        const [folder] = workspaceFolders(app.folder);
        const configuration = debug && await selectDebugConfiguration(folder, { interactive: false, ...debug, target: 'simulator' });
        const timeout = AbortSignal.timeout(30_000);
        const active = AbortSignal.any([signal, this.lifetime.signal, timeout]);
        active.throwIfAborted();
        const device = (await iosSimulators(active)).find(device => device.id === request.device);
        if (!device?.booted) throw new Error('Choose an iOS simulator that is booted on the workspace Mac.');
        active.throwIfAborted();
        const key = `${device.id}:${app.bundleId}`;
        const existing = this.launches.get(key);
        if (existing) {
            const prior = existing.app;
            if (prior.folder !== app.folder || prior.id !== app.id || prior.bundlePath !== app.bundlePath
                || JSON.stringify(prior.build) !== JSON.stringify(app.build)
                || prior.args.length !== app.args.length || prior.args.some((arg, index) => arg !== app.args[index])) {
                throw new Error('Stop the current session for this bundle and simulator before launching it with another configuration.');
            }
            const ready = existing.runtime && this.connections.describe(existing.runtime);
            if (!existing.cancellation.signal.aborted && !existing.process?.hasExited && ready) {
                if (configuration) {
                    if (existing.attaching) throw new Error('Wait for the current simulator debugger operation to finish.');
                    this.debugger(existing).prepareIfIdle(folder, configuration);
                    this.resumeDebugger(existing);
                }
                return ready;
            }
            throw new Error('This simulator app is starting or stopping. Select its current App Sessions entry when ready.');
        }

        const launch: Launch = { app, device, cancellation: new AbortController() };
        this.launches.set(key, launch);
        const startup = AbortSignal.any([active, launch.cancellation.signal]);
        try {
            if (configuration) this.debugger(launch).prepareIfIdle(folder, configuration);
            if (app.build) return this.startDevelopment(launch);
            await installSimulatorApp(device, app, startup);
            const started = launchSimulatorApp(device, app, startup);
            launch.process = started.process;
            const ended = () => this.retire(key, launch);
            void started.process.closed.then(ended, ended);
            const endpoint = await started.endpoint;
            await verifyRuntime(endpoint, startup);
            startup.throwIfAborted();
            workspaceFolders(app.folder);
            if (started.process.hasExited) throw new Error('The simulator app exited before Live App connected.');
            const runtime = this.connections.addPlatform({ kind: 'simulator', simulator: this.target(launch) }, this.label(launch), endpoint,
                () => launch.debugger?.attached ?? false);
            launch.runtime = runtime.runtime;
            this.resumeDebugger(launch);
            return runtime;
        } catch (error) {
            await this.stopLaunch(key, launch);
            if (timeout.aborted && !signal.aborted && !this.lifetime.signal.aborted) {
                throw new Error('Timed out deploying the simulator app and connecting to its inspector.');
            }
            throw error;
        }
    }

    async debug(runtime: AppRuntime, generation: number, options: DebugOptions = {}) {
        this.requireWorkspace();
        const launch = this.launch(runtime);
        const { endpoint, current, generation: acceptedGeneration } = this.connections.session(runtime);
        if (!('url' in endpoint) || acceptedGeneration !== generation) {
            throw new Error('Use the current accepted iOS simulator generation.');
        }
        if (launch.attaching) throw new Error('The debugger is already connecting to this simulator app.');
        const signal = options.signal ? AbortSignal.any([launch.cancellation.signal, options.signal]) : launch.cancellation.signal;
        const requireCurrent = () => {
            signal.throwIfAborted();
            if (!current() || launch.process?.hasExited || launch.development?.deploying) {
                throw new Error('The iOS simulator connection changed while attaching the debugger.');
            }
        };
        launch.attaching = true;
        try {
            requireCurrent();
            if (options.detach) {
                const debuggerSession = launch.debugger;
                launch.debugger = undefined;
                await debuggerSession?.close();
                return { runtime, generation, attached: false };
            }
            const [folder] = workspaceFolders(launch.app.folder);
            const configuration = await selectDebugConfiguration(folder, { interactive: options.interactive ?? false,
                name: options.configuration, target: 'simulator' });
            if (!configuration) return { runtime, generation, attached: false };
            requireCurrent();
            const pid = await simulatorAppPid(launch.device.id, launch.app.bundleId, endpoint, signal);
            requireCurrent();
            const debuggerSession = this.debugger(launch);
            if (!debuggerSession.follows(pid)) await debuggerSession.attach(folder, configuration, pid);
            try { requireCurrent(); }
            catch (error) { debuggerSession.detach(); throw error; }
            return { runtime, generation, pid, attached: true };
        } finally { launch.attaching = false; }
    }

    rebuild(runtime: AppRuntime): RuntimeSummary {
        this.requireWorkspace();
        const launch = this.launch(runtime);
        if (!launch.development) throw new Error('Choose a source-built iOS simulator app session.');
        launch.development.rebuild();
        return this.connections.describe(runtime)!;
    }

    recentOutput(runtime: AppRuntime, lines: number): string {
        this.requireWorkspace();
        return this.launch(runtime).output?.log.tail(lines) ?? '';
    }

    showOutput(runtime: AppRuntime): void { this.output(this.launch(runtime)).channel.show(); }

    async stop(runtime: AppRuntime): Promise<void> {
        const entry = [...this.launches.entries()].find(([, launch]) => launch.runtime === runtime);
        if (!entry) throw new Error('Choose a current iOS simulator app session.');
        await this.stopLaunch(...entry);
    }

    private retire(key: string, launch: Launch): void {
        void this.stopLaunch(key, launch).catch(error => console.warn(error));
    }

    private startDevelopment(launch: Launch): RuntimeSummary {
        const { app, device, cancellation } = launch;
        const pending = this.connections.beginSimulator({ ...this.target(launch), development: true }, this.label(launch), () => {
            const pid = launch.development?.current?.pid;
            return pid !== undefined && launch.debugger?.isAttachedTo(pid) === true;
        });
        const { runtime } = pending;
        launch.runtime = runtime;
        const output = this.output(launch).log;
        const development = new MobileDevelopment<SimulatorBundle, ManagedSimulatorProcess>({
            platform: 'iOS simulator',
            label: app.label,
            reloadOnSave: app.build?.reloadOnSave ?? false,
            build: (signal, starting) => buildSimulatorApp(app, output, signal, starting),
            install: (artifact, signal) => deploySimulatorApp(device, { ...app, bundlePath: artifact.bundlePath }, signal),
            owns: async (process, signal) => {
                if (process.console.hasExited) return false;
                return await simulatorAppPid(device.id, app.bundleId, process.endpoint, signal).catch(() => {
                    signal.throwIfAborted();
                    return undefined;
                }) === process.pid;
            },
            running: signal => simulatorAppIsRunning(device.id, app.bundleId, signal),
            beforeInstall: async () => { await launch.debugger?.detachForReplacement(); },
            publish: (status, process) => {
                this.connections.updateSimulator(runtime, status, process);
                if (process) {
                    const ended = () => {
                        if (development.current !== process || development.deploying || cancellation.signal.aborted) return;
                        launch.debugger?.detach();
                        void development.exited(process).catch(error => output.appendLine(String(error)));
                    };
                    void process.console.closed.then(ended, ended);
                }
            },
            resumeDebugger: process => launch.debugger?.activated(process.pid),
        }, runtime, this.history, output, cancellation.signal);
        launch.development = development;
        development.rebuild();
        return pending;
    }

    private launch(runtime: AppRuntime): Launch {
        const launch = [...this.launches.values()].find(item => item.runtime === runtime);
        if (!launch) throw new Error('Choose a current iOS simulator app session.');
        return launch;
    }

    private label({ app, device }: Launch): string { return `${app.label} · ${device.name}`; }

    private debugger(launch: Launch): ReloadDebugger {
        return launch.debugger ??= new ReloadDebugger(this.output(launch).log, {
            attachError: 'The configured Rust debugger could not attach to this simulator app.',
            onDidChange: () => this.connections.notifyDebuggerChanged(launch.runtime),
        });
    }

    private resumeDebugger(launch: Launch): void {
        const { debugger: debuggerSession, development, runtime, cancellation: { signal } } = launch;
        if (!debuggerSession || !runtime || signal.aborted) return;
        if (development) {
            if (development.current && !development.deploying) debuggerSession.activated(development.current.pid);
            return;
        }
        if (launch.attaching) return;
        const { endpoint, current } = this.connections.session(runtime);
        if (!('url' in endpoint)) return;
        launch.attaching = true;
        void simulatorAppPid(launch.device.id, launch.app.bundleId, endpoint, signal).then(pid => {
            if (current() && !signal.aborted && !launch.process?.hasExited) debuggerSession.activated(pid);
        }).catch(error => {
            if (!signal.aborted) this.output(launch).log.appendLine(errorMessage(error));
        }).finally(() => { launch.attaching = false; });
    }

    private target({ app, device }: Launch): IosRuntimeTarget {
        return { device: device.id, name: device.name, system: device.system, profile: app.id, bundleId: app.bundleId };
    }

    private output(launch: Launch) {
        if (!launch.output) {
            const channel = vscode.window.createOutputChannel(`FerroUI iOS Simulator · ${this.label(launch)}`);
            launch.output = { channel, log: new ReloadLog(channel) };
        }
        return launch.output;
    }

    private stopLaunch(key: string, launch: Launch): Promise<void> {
        if (launch.stopping) return launch.stopping;
        launch.cancellation.abort(new Error('The simulator launch stopped.'));
        if (launch.runtime) this.connections.removePlatform(launch.runtime);
        return launch.stopping = (async () => {
            try { await launch.debugger?.close(); }
            finally {
                try {
                    if (launch.development) await launch.development.close(true);
                    else await launch.process?.stop();
                } finally {
                    launch.output?.channel.dispose();
                    if (this.launches.get(key) === launch) this.launches.delete(key);
                }
            }
        })();
    }

    private requireWorkspace(): void {
        if (!this.supported) throw new Error('iOS simulator sessions require a macOS workspace host.');
        if (!vscode.workspace.isTrusted || !workspaceFolders().length) throw new Error('Open a trusted workspace on the Mac running Xcode.');
        this.lifetime.signal.throwIfAborted();
    }

    async shutdown(): Promise<void> {
        this.lifetime.abort();
        await Promise.allSettled([...this.launches].map(([key, launch]) => this.stopLaunch(key, launch)));
    }

    dispose(): void { this.folders.dispose(); this.saves.dispose(); void this.shutdown(); }
}
