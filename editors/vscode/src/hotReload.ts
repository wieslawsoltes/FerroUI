import * as vscode from 'vscode';
import { selectCargoTarget } from './cargoTarget';
import { errorMessage } from './protocol';
import { ReloadSession } from './reloadSession';

export class HotReload implements vscode.Disposable {
    private readonly output = vscode.window.createOutputChannel('FerroUI Hot Reload');
    private readonly indicator = vscode.window.createStatusBarItem('ferroui.hotReload', vscode.StatusBarAlignment.Left, 35);
    private readonly subscriptions: vscode.Disposable[];
    private session?: ReloadSession;
    private choosing = false;
    private generation = 0;
    private message = 'Stopped';

    constructor() {
        this.indicator.name = 'FerroUI Hot Reload';
        this.indicator.command = 'ferroui.hotReload.showOutput';
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.hotReload.start', () => this.start()),
            vscode.commands.registerCommand('ferroui.hotReload.rebuild', () => this.session ? this.session.build() : this.start()),
            vscode.commands.registerCommand('ferroui.hotReload.stop', () => this.stop()),
            vscode.commands.registerCommand('ferroui.hotReload.showOutput', () => this.output.show(true)),
        ];
    }

    get state() { return { target: this.session?.target.label ?? null, pid: this.session?.pid ?? null, status: this.message }; }

    async agentControl(action: string): Promise<object> {
        if (action === 'start') await this.start();
        else if (action === 'stop') this.stop();
        else if (action === 'rebuild') {
            if (!this.session) throw new Error('Start a managed FerroUI application first.');
            this.session.build();
        }
        return this.state;
    }

    private async start(): Promise<void> {
        if (this.choosing) return;
        const generation = ++this.generation;
        this.choosing = true;
        try {
            const target = await selectCargoTarget();
            if (!target || this.generation !== generation) return;
            this.session?.dispose();
            this.session = new ReloadSession(target, this.output, (message, problem) => {
                this.message = message;
                this.indicator.text = `$(flame) ${message}`;
                this.indicator.tooltip = `${target.label} · ${message}\nShow FerroUI Hot Reload output`;
                this.indicator.backgroundColor = problem ? new vscode.ThemeColor('statusBarItem.errorBackground') : undefined;
                this.indicator.show();
            });
            this.message = 'Build queued';
            this.session.build();
        } catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(`FerroUI hot reload could not start. ${errorMessage(error)}`);
        } finally {
            this.choosing = false;
        }
    }

    private stop(): void {
        this.generation++;
        this.session?.dispose();
        this.session = undefined;
        this.message = 'Stopped';
        this.indicator.hide();
    }

    dispose(): void {
        this.stop();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.indicator.dispose();
        this.output.dispose();
    }
}
