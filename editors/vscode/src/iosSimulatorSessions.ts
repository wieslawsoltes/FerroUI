import * as vscode from 'vscode';
import { installSimulatorApp, iosSimulators, launchSimulatorApp } from './iosSimulatorProcess';
import { iosSimulatorApps, type IosSimulatorApp, type IosSimulatorRun } from './iosSimulatorTargets';
import type { OwnedProcess } from './ownedProcess';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime } from './runtimeTargets';
import { workspaceFolders } from './workspacePaths';

interface Launch {
    app: IosSimulatorApp;
    cancellation: AbortController;
    process?: OwnedProcess;
    runtime?: AppRuntime;
}

export class IosSimulatorSessions implements vscode.Disposable {
    readonly supported = process.platform === 'darwin';
    private readonly lifetime = new AbortController();
    private readonly launches = new Map<string, Launch>();
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, launch] of this.launches) {
            if (closed.has(launch.app.folder)) void this.stopLaunch(key, launch);
        }
    });

    constructor(private readonly connections: RuntimeConnections) {}

    async targets(folder: string | undefined, signal: AbortSignal) {
        if (!this.supported) return { supported: false, devices: [], apps: [] };
        this.requireWorkspace();
        const profiles = iosSimulatorApps(folder);
        const devices = await iosSimulators(AbortSignal.any([signal, this.lifetime.signal]));
        return { supported: true, devices, apps: profiles.map(({ id, bundleId, label, folder, bundlePath }) => ({
            id, bundleId, label, folder, installsBundle: bundlePath !== undefined,
        })) };
    }

    async start(request: IosSimulatorRun, signal: AbortSignal): Promise<RuntimeSummary> {
        this.requireWorkspace();
        const timeout = AbortSignal.timeout(30_000);
        const active = AbortSignal.any([signal, this.lifetime.signal, timeout]);
        active.throwIfAborted();
        const apps = iosSimulatorApps(request.folder).filter(app => app.id === request.profile);
        const [app] = apps;
        if (apps.length !== 1) throw new Error('Choose an unambiguous iOS simulator launch profile.');
        const device = (await iosSimulators(active)).find(device => device.id === request.device);
        if (!device?.booted) throw new Error('Choose an iOS simulator that is booted on the workspace Mac.');
        active.throwIfAborted();
        const key = `${device.id}:${app.bundleId}`;
        const existing = this.launches.get(key);
        if (existing) {
            const prior = existing.app;
            if (prior.folder !== app.folder || prior.id !== app.id || prior.bundlePath !== app.bundlePath
                || prior.args.length !== app.args.length || prior.args.some((arg, index) => arg !== app.args[index])) {
                throw new Error('Stop the current session for this bundle and simulator before launching it with another configuration.');
            }
            const ready = existing.runtime && this.connections.describe(existing.runtime);
            if (!existing.cancellation.signal.aborted && !existing.process?.hasExited && ready) return ready;
            throw new Error('This simulator app is starting or stopping. Select its current App Sessions entry when ready.');
        }

        const launch: Launch = { app, cancellation: new AbortController() };
        this.launches.set(key, launch);
        const startup = AbortSignal.any([active, launch.cancellation.signal]);
        try {
            await installSimulatorApp(device, app, startup);
            const started = launchSimulatorApp(device, app, startup);
            launch.process = started.process;
            const ended = () => {
                launch.cancellation.abort(new Error('The simulator app exited.'));
                this.removeLaunch(key, launch);
            };
            void started.process.closed.then(ended, ended);
            const endpoint = await started.endpoint;
            await verifyRuntime(endpoint, startup);
            startup.throwIfAborted();
            workspaceFolders(app.folder);
            if (started.process.hasExited) throw new Error('The simulator app exited before Live App connected.');
            const runtime = this.connections.addPlatform({ kind: 'simulator', simulator: { device: device.id, name: device.name,
                system: device.system, profile: app.id, bundleId: app.bundleId } }, `${app.label} · ${device.name}`, endpoint);
            launch.runtime = runtime.runtime;
            return runtime;
        } catch (error) {
            await this.stopLaunch(key, launch);
            if (timeout.aborted && !signal.aborted && !this.lifetime.signal.aborted) {
                throw new Error('Timed out deploying the simulator app and connecting to its inspector.');
            }
            throw error;
        }
    }

    async stop(runtime: AppRuntime): Promise<void> {
        const entry = [...this.launches.entries()].find(([, launch]) => launch.runtime === runtime);
        if (!entry) throw new Error('Choose a current iOS simulator app session.');
        await this.stopLaunch(...entry);
    }

    private async stopLaunch(key: string, launch: Launch): Promise<void> {
        launch.cancellation.abort(new Error('The simulator launch stopped.'));
        if (launch.runtime) this.connections.removePlatform(launch.runtime);
        await launch.process?.stop();
        this.removeLaunch(key, launch);
    }

    private removeLaunch(key: string, launch: Launch): void {
        if (this.launches.get(key) !== launch) return;
        this.launches.delete(key);
        if (launch.runtime) this.connections.removePlatform(launch.runtime);
    }

    private requireWorkspace(): void {
        if (!this.supported) throw new Error('iOS simulator sessions require a macOS workspace host.');
        if (!vscode.workspace.isTrusted || !workspaceFolders().length) throw new Error('Open a trusted workspace on the Mac running Xcode.');
        this.lifetime.signal.throwIfAborted();
    }

    async shutdown(): Promise<void> {
        this.lifetime.abort();
        await Promise.all([...this.launches].map(([key, launch]) => this.stopLaunch(key, launch)));
    }

    dispose(): void { this.folders.dispose(); void this.shutdown(); }
}
