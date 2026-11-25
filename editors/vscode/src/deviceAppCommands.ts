import * as vscode from 'vscode';
import type { AndroidSessions } from './androidSessions';
import { hasAndroidProfiles, type AndroidMode, type AndroidRequest } from './androidTargets';
import { connectApp } from './connectApp';
import type { IosAppRun } from './iosAppProfiles';
import type { IosDeviceSessions } from './iosDeviceSessions';
import { hasIosDeviceProfiles } from './iosDeviceTargets';
import type { IosSimulatorSessions } from './iosSimulatorSessions';
import type { IosSimulatorRun } from './iosSimulatorTargets';
import type { AppRuntime } from './runtimeTargets';

export class DeviceAppCommands implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly ios: IosSimulatorSessions, private readonly devices: IosDeviceSessions, private readonly android: AndroidSessions,
        open: (runtime: AppRuntime) => void) {
        const context = () => {
            void vscode.commands.executeCommand('setContext', 'ferroui.mobileAppProfiles', this.ios.supported || hasAndroidProfiles());
        };
        void vscode.commands.executeCommand('setContext', 'ferroui.iosSimulatorHost', ios.supported);
        void vscode.commands.executeCommand('setContext', 'ferroui.iosDeviceHost', devices.supported);
        context();
        this.subscriptions = [
            vscode.workspace.onDidChangeWorkspaceFolders(context),
            vscode.workspace.onDidChangeConfiguration(event => { if (event.affectsConfiguration('ferroui.android.apps')) context(); }),
            vscode.commands.registerCommand('ferroui.appSessions.mobile', () => this.chooseAction()),
            ...(['run', 'debug'] as const).map(mode => vscode.commands.registerCommand(`ferroui.iosSimulator.${mode}`, (selection?: IosSimulatorRun) =>
                connectApp('Starting the iOS simulator app', () => selection ?? this.chooseIosApp(mode),
                    (request, signal) => this.ios.start(request, signal, mode === 'debug' ? { interactive: true } : undefined), open))),
            ...(['run', 'debug'] as const).map(mode => vscode.commands.registerCommand(`ferroui.iosDevice.${mode}`, (selection?: IosAppRun) =>
                connectApp('Connecting to the iOS device app', () => selection ?? this.chooseIosDeviceApp(mode),
                    (request, signal) => this.devices.open(request, signal, mode === 'debug' ? { interactive: true } : undefined), open))),
            ...(['run', 'debug', 'attach'] as const).map(mode => vscode.commands.registerCommand(`ferroui.android.${mode}`, (selection?: AndroidRequest) =>
                connectApp('Connecting to the Android app', () => selection ?? this.chooseAndroidApp(mode),
                    (request, signal) => this.android.open(request, mode === 'attach' ? 'attach' : 'run', signal,
                        mode === 'debug' ? { interactive: true } : undefined), open))),
        ];
    }

    private async chooseAction(): Promise<void> {
        const actions: { label: string; command: string }[] = [];
        if (this.ios.supported) actions.push(
            { label: 'Run iOS Simulator App', command: 'ferroui.iosSimulator.run' },
            { label: 'Run and Debug iOS Simulator App', command: 'ferroui.iosSimulator.debug' },
        );
        if (this.devices.supported && hasIosDeviceProfiles()) actions.push(
            { label: 'Run iOS Device App', command: 'ferroui.iosDevice.run' },
            { label: 'Run and Debug iOS Device App', command: 'ferroui.iosDevice.debug' },
        );
        if (hasAndroidProfiles()) actions.push(
            { label: 'Run Android App', command: 'ferroui.android.run' },
            { label: 'Run and Debug Android App', command: 'ferroui.android.debug' },
            { label: 'Attach to Android App', command: 'ferroui.android.attach' },
        );
        const action = actions.length === 1 ? actions[0]
            : await vscode.window.showQuickPick(actions, { title: 'Open Mobile App', placeHolder: 'Choose a device workflow' });
        if (action) await vscode.commands.executeCommand(action.command);
    }

    private async chooseIosApp(mode: 'run' | 'debug'): Promise<IosSimulatorRun | undefined> {
        const { supported, devices, apps } = await this.ios.targets(undefined, AbortSignal.timeout(10_000));
        if (!supported) throw new Error('iOS simulator sessions require a macOS workspace host.');
        if (!apps.length) throw new Error('Add a launch profile to ferroui.iosSimulator.apps in Workspace Settings.');
        const booted = devices.filter(device => device.booted);
        if (!booted.length) throw new Error('Boot an iOS simulator in Xcode or Simulator on the workspace Mac.');
        const app = await vscode.window.showQuickPick(apps.map(app => ({
            label: app.label, description: app.buildsBundle ? 'Build from Cargo' : app.bundleId, detail: vscode.Uri.parse(app.folder).fsPath, app,
        })), { title: mode === 'debug' ? 'Run and Debug iOS Simulator App' : 'Run iOS Simulator App',
            placeHolder: 'Choose the configured app' });
        if (!app) return;
        const device = await vscode.window.showQuickPick(booted.map(device => ({
            label: device.name, description: device.system, detail: device.id, device,
        })), { title: 'Select an iOS Simulator', placeHolder: 'Choose the run destination', matchOnDescription: true, matchOnDetail: true });
        if (device) return { device: device.device.id, profile: app.app.id, folder: app.app.folder };
    }

    private async chooseAndroidApp(mode: AndroidMode | 'debug'): Promise<AndroidRequest | undefined> {
        if (!hasAndroidProfiles()) throw new Error('Add a profile to ferroui.android.apps in Workspace Settings.');
        const { devices, apps } = await this.android.targets(undefined, AbortSignal.timeout(12_000));
        const available = devices.filter(device => device.status === 'device');
        if (!available.length) throw new Error('Start an Android emulator or authorize a connected device in ADB.');
        const app = await vscode.window.showQuickPick(apps.map(app => ({
            label: app.label, description: app.packageId, detail: vscode.Uri.parse(app.folder).fsPath, app,
        })), { title: { run: 'Run Android App', debug: 'Run and Debug Android App', attach: 'Attach to Android App' }[mode],
            placeHolder: 'Choose the configured Android app',
            matchOnDescription: true, matchOnDetail: true });
        if (!app) return;
        const device = await vscode.window.showQuickPick(available.map(device => ({
            label: device.name, description: device.emulator ? 'Emulator' : 'Device', detail: device.id, device,
        })), { title: 'Select an Android Device', placeHolder: 'Choose the app’s device', matchOnDescription: true, matchOnDetail: true });
        if (device) return { device: device.device.id, profile: app.app.id, folder: app.app.folder };
    }

    private async chooseIosDeviceApp(mode: 'run' | 'debug'): Promise<IosAppRun | undefined> {
        if (!hasIosDeviceProfiles()) throw new Error('Add a launch profile to ferroui.iosDevice.apps in Workspace Settings.');
        const { supported, devices, apps } = await this.devices.targets(undefined, AbortSignal.timeout(10_000));
        if (!supported) throw new Error('iOS device sessions require a macOS workspace host.');
        const available = devices.filter(device => device.status === 'ready');
        if (!available.length) throw new Error('Connect an iPhone or iPad with Developer Mode enabled in Xcode’s Devices and Simulators window.');
        const app = await vscode.window.showQuickPick(apps.map(app => ({
            label: app.label, description: app.buildsBundle ? 'Build and sign from Cargo' : app.bundleId, detail: vscode.Uri.parse(app.folder).fsPath, app,
        })), { title: mode === 'debug' ? 'Run and Debug iOS Device App' : 'Run iOS Device App', placeHolder: 'Choose the configured app' });
        if (!app) return;
        const device = await vscode.window.showQuickPick(available.map(device => ({
            label: device.name, description: device.system, detail: device.id, device,
        })), { title: 'Select an iOS Device', placeHolder: 'Choose the app’s device', matchOnDescription: true, matchOnDetail: true });
        if (device) return { device: device.device.id, profile: app.app.id, folder: app.app.folder };
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
