import * as vscode from 'vscode';
import type { AndroidSessions } from './androidSessions';
import { BrowserAppCommands } from './browserAppCommands';
import type { BrowserSessions } from './browserSessions';
import { DeviceAppCommands } from './deviceAppCommands';
import type { HotReload } from './hotReload';
import type { IosDeviceSessions } from './iosDeviceSessions';
import type { IosSimulatorSessions } from './iosSimulatorSessions';
import { errorMessage } from './protocol';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import type { RuntimePanels } from './runtimePanels';
import { RuntimeMcpActions } from './runtimeMcpActions';
import type { RuntimeViewChange } from './runtimeProtocol';
import type { AppRuntime } from './runtimeTargets';

class AppItem extends vscode.TreeItem {
    constructor(readonly session: RuntimeSummary) { super(session.label); }
}

export class AppSessions implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    private readonly subscriptions: vscode.Disposable[];
    readonly mcp: RuntimeMcpActions;

    constructor(private readonly reload: HotReload, private readonly connections: RuntimeConnections,
        private readonly panels: RuntimePanels, readonly simulators: IosSimulatorSessions, readonly iosDevices: IosDeviceSessions, readonly android: AndroidSessions,
        readonly browser: BrowserSessions) {
        this.mcp = new RuntimeMcpActions(connections);
        const view = vscode.window.createTreeView<AppItem>('ferroui.appSessions', { treeDataProvider: {
            onDidChangeTreeData: this.changed.event,
            getChildren: item => item ? [] : this.connections.targets().map(session => this.item(session)),
            getTreeItem: item => item,
        } });
        let hasManagedApps = false;
        const refresh = () => {
            const count = this.connections.targets().length;
            view.badge = count ? { value: count, tooltip: `${count} FerroUI app sessions` } : undefined;
            const managing = this.reload.runs().length > 0;
            if (managing !== hasManagedApps) {
                hasManagedApps = managing;
                void vscode.commands.executeCommand('setContext', 'ferroui.hasManagedApps', managing);
            }
            this.changed.fire();
        };
        const command = (name: string, action: (session: RuntimeSummary) => unknown) =>
            vscode.commands.registerCommand(`ferroui.appSessions.${name}`, (item?: AppItem | AppRuntime) => {
                const runtime = typeof item === 'string' ? item : item?.session.runtime;
                if (!runtime) return;
                const current = this.connections.describe(runtime);
                if (current) return action(current);
                void vscode.window.showInformationMessage('This FerroUI app session has ended.');
            });
        const managed = (name: string) => command(name, session => session.app_session
            && vscode.commands.executeCommand(`ferroui.hotReload.${name}`, session.app_session));
        this.subscriptions = [view, view.onDidChangeVisibility(({ visible }) => { if (visible) refresh(); }),
            reload.onDidChangeRuns(refresh), connections.onDidChange(refresh), connections.onDidChangeDebugger(() => this.changed.fire()),
            vscode.commands.registerCommand('ferroui.showAppSessions', () => vscode.commands.executeCommand('ferroui.appSessions.focus')),
            new DeviceAppCommands(simulators, iosDevices, android, runtime => this.open(runtime)),
            new BrowserAppCommands(browser, runtime => this.open(runtime)),
            command('preview', session => this.open(session.runtime)),
            managed('select'),
            command('debug', session => this.debug(session)),
            command('detachDebug', session => this.debug(session, true)),
            command('rebuild', session => {
                if (session.simulator?.development) return this.simulators.rebuild(session.runtime);
                if (session.android?.development) return this.android.rebuild(session.runtime);
                if (session.browser?.development) return this.browser.rebuild(session.runtime);
                if (session.app_session) return vscode.commands.executeCommand('ferroui.hotReload.rebuild', session.app_session);
            }),
            command('showOutput', session => {
                if (session.simulator) return this.simulators.showOutput(session.runtime);
                if (session.iosDevice) return this.iosDevices.showOutput(session.runtime);
                if (session.android) return this.android.showOutput(session.runtime);
                if (session.browser) return this.browser.showOutput(session.runtime);
                if (session.app_session) return vscode.commands.executeCommand('ferroui.hotReload.showOutput', session.app_session);
            }),
            command('stop', session => this.stop(session)),
        ];
    }

    list() { return { active: this.reload.active ?? null, managed: this.reload.runs().map(run => run.state), runtimes: this.connections.targets() }; }

    attach(url: string, token: string, name?: string, signal?: AbortSignal) { return this.connections.attach(url, token, name, signal); }
    detach(runtime: AppRuntime): void { this.connections.detach(runtime); }
    open(runtime: AppRuntime): void { this.panels.open(runtime); }
    view(runtime: AppRuntime) { return this.panels.view(runtime); }
    controlView(runtime: AppRuntime, generation: number, revision: number, change: RuntimeViewChange, signal: AbortSignal) {
        return this.panels.control(runtime, generation, revision, change, signal);
    }

    private async debug(session: RuntimeSummary, detach = false): Promise<void> {
        try {
            if (session.app_session) {
                if (detach) await this.reload.detachDebugger(session.app_session);
                else await vscode.commands.executeCommand('ferroui.hotReload.debug', session.app_session);
            } else if (session.generation !== null) {
                const options = { interactive: !detach, detach };
                if (session.browser) await this.browser.debug(session.runtime, session.generation, options);
                else if (session.simulator) await this.simulators.debug(session.runtime, session.generation, options);
                else if (session.iosDevice) await this.iosDevices.debug(session.runtime, session.generation, options);
                else if (session.android) await this.android.debug(session.runtime, session.generation, options);
            }
        } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private stop(session: RuntimeSummary) {
        if (session.app_session) return vscode.commands.executeCommand('ferroui.hotReload.stop', session.app_session);
        if (session.kind === 'simulator') return this.simulators.stop(session.runtime);
        if (session.kind === 'ios-device') return this.iosDevices.stop(session.runtime);
        if (session.kind === 'android') return this.android.stop(session.runtime);
        if (session.kind === 'browser') return this.browser.stop(session.runtime);
        return vscode.commands.executeCommand('ferroui.mcp.detach', session.runtime);
    }

    private item(session: RuntimeSummary): AppItem {
        const item = new AppItem(session);
        const run = session.app_session ? this.reload.stateFor(session.app_session) : undefined;
        const selected = session.app_session !== undefined && session.app_session === this.reload.active;
        item.id = session.runtime;
        const debuggerStatus = session.browser?.paused ? 'Paused in debugger'
            : session.debuggerAttached ? 'Debugger attached' : undefined;
        item.description = [debuggerStatus, session.pid ? `PID ${session.pid}` : undefined, session.status].filter(Boolean).join(' · ');
        const details = [session.label, item.description, session.debuggerAttached === false && !debuggerStatus ? 'Debugger detached' : undefined,
            selected ? 'Selected managed app' : undefined].filter(Boolean);
        item.tooltip = details.join('\n');
        item.accessibilityInformation = { label: details.join('. ') };
        const problem = (run && 'problem' in run && run.problem)
            || ((session.browser?.development || session.android?.development || session.simulator?.development) && session.status.includes(' failed'));
        const debuggerIcon = session.browser?.paused ? 'debug-pause' : session.debuggerAttached ? 'debug-alt' : undefined;
        const targetIcon = selected ? 'play-circle'
            : { attached: 'plug', simulator: 'device-mobile', 'ios-device': 'device-mobile', android: 'device-mobile', browser: 'globe', managed: 'run' }[session.kind];
        item.iconPath = new vscode.ThemeIcon(problem ? 'warning' : debuggerIcon ?? targetIcon);
        if (session.simulator?.development) {
            item.contextValue = session.available ? 'app-session-simulator-development-debug' : 'app-session-simulator-development';
        } else if (session.browser) {
            const development = session.browser.development ? '-development' : '';
            const debug = session.browser.debuggable && session.available ? '-debug' : '';
            item.contextValue = `app-session-browser${development}${debug}`;
        } else if (session.android) {
            const development = session.android.development ? '-development' : '';
            const debug = session.android.debuggable && session.available ? '-debug' : '';
            item.contextValue = `app-session-android${development}${debug}`;
        } else {
            item.contextValue = run && 'kind' in run && run.kind === 'registered' ? 'app-session-registered' : `app-session-${session.kind}`;
        }
        item.command = { command: 'ferroui.appSessions.preview', title: 'Open Live App', arguments: [session.runtime] };
        return item;
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); this.changed.dispose(); }
}
