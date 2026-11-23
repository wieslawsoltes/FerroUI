import * as vscode from 'vscode';
import type { AndroidSessions } from './androidSessions';
import { hasAndroidProfiles, type AndroidMode, type AndroidRequest } from './androidTargets';
import type { IosSimulatorSessions } from './iosSimulatorSessions';
import type { IosSimulatorRun } from './iosSimulatorTargets';
import { errorMessage } from './protocol';
import type { RuntimeSummary } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';

export class DeviceAppCommands implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly ios: IosSimulatorSessions, private readonly android: AndroidSessions,
        private readonly open: (runtime: AppRuntime) => void) {
        const context = () => {
            void vscode.commands.executeCommand('setContext', 'ferroui.mobileAppProfiles', this.ios.supported || hasAndroidProfiles());
        };
        void vscode.commands.executeCommand('setContext', 'ferroui.iosSimulatorHost', ios.supported);
        context();
        this.subscriptions = [
            vscode.workspace.onDidChangeWorkspaceFolders(context),
            vscode.workspace.onDidChangeConfiguration(event => { if (event.affectsConfiguration('ferroui.android.apps')) context(); }),
            vscode.commands.registerCommand('ferroui.appSessions.mobile', () => this.chooseAction()),
            vscode.commands.registerCommand('ferroui.iosSimulator.run', (selection?: IosSimulatorRun) =>
                this.connect('Connecting to the iOS simulator app', () => selection ?? this.chooseIosApp(), (request, signal) => this.ios.start(request, signal))),
            ...(['run', 'attach'] as const).map(mode => vscode.commands.registerCommand(`ferroui.android.${mode}`, (selection?: AndroidRequest) =>
                this.connect('Connecting to the Android app', () => selection ?? this.chooseAndroidApp(mode), (request, signal) => this.android.open(request, mode, signal)))),
        ];
    }

    private async chooseAction(): Promise<void> {
        const actions: { label: string; command: string }[] = [];
        if (this.ios.supported) actions.push({ label: 'Run iOS Simulator App', command: 'ferroui.iosSimulator.run' });
        if (hasAndroidProfiles()) actions.push(
            { label: 'Run Android App', command: 'ferroui.android.run' },
            { label: 'Attach to Android App', command: 'ferroui.android.attach' },
        );
        const action = actions.length === 1 ? actions[0]
            : await vscode.window.showQuickPick(actions, { title: 'Open Mobile App', placeHolder: 'Choose a device workflow' });
        if (action) await vscode.commands.executeCommand(action.command);
    }

    private async connect<T>(title: string, choose: () => T | Promise<T | undefined>,
        launch: (request: T, signal: AbortSignal) => Promise<RuntimeSummary>): Promise<void> {
        try {
            const request = await choose();
            if (!request) return;
            const runtime = await vscode.window.withProgress({ title, location: vscode.ProgressLocation.Notification, cancellable: true },
                async (_, token) => {
                    const controller = new AbortController();
                    const subscription = token.onCancellationRequested(() => controller.abort());
                    if (token.isCancellationRequested) controller.abort();
                    try { return await launch(request, controller.signal); }
                    catch (error) { if (!controller.signal.aborted) throw error; }
                    finally { subscription.dispose(); }
                });
            if (runtime) this.open(runtime.runtime);
        } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private async chooseIosApp(): Promise<IosSimulatorRun | undefined> {
        const { supported, devices, apps } = await this.ios.targets(undefined, AbortSignal.timeout(10_000));
        if (!supported) throw new Error('iOS simulator sessions require a macOS workspace host.');
        if (!apps.length) throw new Error('Add a launch profile to ferroui.iosSimulator.apps in Workspace Settings.');
        const booted = devices.filter(device => device.booted);
        if (!booted.length) throw new Error('Boot an iOS simulator in Xcode or Simulator on the workspace Mac.');
        const app = await vscode.window.showQuickPick(apps.map(app => ({
            label: app.label, description: app.bundleId, detail: vscode.Uri.parse(app.folder).fsPath, app,
        })), { title: 'Run iOS Simulator App', placeHolder: 'Choose the configured app to launch or restart' });
        if (!app) return;
        const device = await vscode.window.showQuickPick(booted.map(device => ({
            label: device.name, description: device.system, detail: device.id, device,
        })), { title: 'Select an iOS Simulator', placeHolder: 'Choose the run destination', matchOnDescription: true, matchOnDetail: true });
        if (device) return { device: device.device.id, profile: app.app.id, folder: app.app.folder };
    }

    private async chooseAndroidApp(mode: AndroidMode): Promise<AndroidRequest | undefined> {
        if (!hasAndroidProfiles()) throw new Error('Add a profile to ferroui.android.apps in Workspace Settings.');
        const { devices, apps } = await this.android.targets(undefined, AbortSignal.timeout(12_000));
        const available = devices.filter(device => device.status === 'device');
        if (!available.length) throw new Error('Start an Android emulator or authorize a connected device in ADB.');
        const app = await vscode.window.showQuickPick(apps.map(app => ({
            label: app.label, description: app.packageId, detail: vscode.Uri.parse(app.folder).fsPath, app,
        })), { title: mode === 'run' ? 'Run Android App' : 'Attach to Android App', placeHolder: 'Choose the configured Android app',
            matchOnDescription: true, matchOnDetail: true });
        if (!app) return;
        const device = await vscode.window.showQuickPick(available.map(device => ({
            label: device.name, description: device.emulator ? 'Emulator' : 'Device', detail: device.id, device,
        })), { title: 'Select an Android Device', placeHolder: 'Choose the app’s device', matchOnDescription: true, matchOnDetail: true });
        if (device) return { device: device.device.id, profile: app.app.id, folder: app.app.folder };
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
