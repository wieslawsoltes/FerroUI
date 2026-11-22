import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import type { RuntimePanels } from './runtimePanels';
import type { AppRuntime } from './runtimeTargets';

class AppItem extends vscode.TreeItem {
    constructor(readonly session: RuntimeSummary) { super(session.label); }
}

export class AppSessions implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly reload: HotReload, private readonly connections: RuntimeConnections,
        private readonly panels: RuntimePanels) {
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
            reload.onDidChangeRuns(refresh), connections.onDidChange(refresh),
            vscode.commands.registerCommand('ferroui.showAppSessions', () => vscode.commands.executeCommand('ferroui.appSessions.focus')),
            command('preview', session => this.open(session.runtime)),
            managed('select'), managed('rebuild'), managed('debug'), managed('showOutput'),
            command('stop', session => session.app_session
                ? vscode.commands.executeCommand('ferroui.hotReload.stop', session.app_session)
                : vscode.commands.executeCommand('ferroui.mcp.detach', session.runtime)),
        ];
    }

    list() { return { active: this.reload.active ?? null, managed: this.reload.runs().map(run => run.state), runtimes: this.connections.targets() }; }

    attach(url: string, token: string, name?: string, signal?: AbortSignal) { return this.connections.attach(url, token, name, signal); }
    detach(runtime: AppRuntime): void { this.connections.detach(runtime); }
    open(runtime: AppRuntime): void { this.panels.open(runtime); }

    private item(session: RuntimeSummary): AppItem {
        const item = new AppItem(session);
        const run = session.app_session ? this.reload.stateFor(session.app_session) : undefined;
        const selected = session.app_session !== undefined && session.app_session === this.reload.active;
        item.id = session.runtime;
        item.description = [session.pid ? `PID ${session.pid}` : undefined, session.status].filter(Boolean).join(' · ');
        item.tooltip = `${session.label}\n${item.description}${selected ? '\nSelected managed app' : ''}`;
        item.accessibilityInformation = { label: `${session.label}. ${item.description}${selected ? '. Selected managed app' : ''}` };
        const problem = run && 'problem' in run && run.problem;
        item.iconPath = new vscode.ThemeIcon(problem ? 'warning' : selected ? 'play-circle' : session.kind === 'attached' ? 'plug' : 'run');
        item.contextValue = run && 'kind' in run && run.kind === 'registered' ? 'app-session-registered' : `app-session-${session.kind}`;
        item.command = { command: 'ferroui.appSessions.preview', title: 'Open Live App', arguments: [session.runtime] };
        return item;
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); this.changed.dispose(); }
}
