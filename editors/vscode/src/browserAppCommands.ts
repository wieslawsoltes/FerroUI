import * as vscode from 'vscode';
import type { BrowserSessions } from './browserSessions';
import { hasBrowserProfiles, type BrowserMode, type BrowserRequest } from './browserTargets';
import { connectApp } from './connectApp';
import type { AppRuntime } from './runtimeTargets';

const actions = [
    { label: 'Run Browser App', mode: 'run', iconPath: new vscode.ThemeIcon('play') },
    { label: 'Attach to Browser App', mode: 'attach', iconPath: new vscode.ThemeIcon('debug-disconnect') },
] as const;

export class BrowserAppCommands implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly sessions: BrowserSessions, open: (runtime: AppRuntime) => void) {
        const context = () => { void vscode.commands.executeCommand('setContext', 'ferroui.browserAppProfiles', hasBrowserProfiles()); };
        context();
        this.subscriptions = [
            vscode.workspace.onDidChangeWorkspaceFolders(context),
            vscode.workspace.onDidChangeConfiguration(event => { if (event.affectsConfiguration('ferroui.browser.apps')) context(); }),
            vscode.commands.registerCommand('ferroui.appSessions.browser', async () => {
                const action = await vscode.window.showQuickPick(actions, { title: 'Open Browser App', placeHolder: 'Choose a browser workflow' });
                if (action) await vscode.commands.executeCommand(`ferroui.browser.${action.mode}`);
            }),
            ...actions.map(({ label, mode }) => vscode.commands.registerCommand(`ferroui.browser.${mode}`, (selection?: BrowserRequest) =>
                connectApp('Connecting to the browser app', () => selection ?? this.chooseApp(mode, label),
                    (request, signal) => this.sessions.open(request, mode, signal), open))),
        ];
    }

    private async chooseApp(mode: BrowserMode, title: string): Promise<BrowserRequest | undefined> {
        const { apps } = this.sessions.targets();
        if (!apps.length) throw new Error('Add a profile to ferroui.browser.apps in Workspace Settings.');
        const available = apps.filter(app => mode === 'run' || app.debugPort !== undefined);
        if (!available.length) throw new Error('Add a local Chromium debug port to the browser app profile.');
        const pick = await vscode.window.showQuickPick(available.map(app => ({ label: app.label, description: app.url,
            detail: vscode.Uri.parse(app.folder).fsPath, app })), { title, placeHolder: 'Choose the configured app', matchOnDescription: true, matchOnDetail: true });
        if (pick) return { profile: pick.app.id, folder: pick.app.folder };
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
