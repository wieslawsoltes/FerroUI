import * as vscode from 'vscode';
import { describeCargoTarget, discoverCargoTargets, selectCargoTarget, type CargoSelection, type CargoTarget } from './cargoTarget';
import { errorMessage } from './protocol';
import { ReloadLog } from './reloadLog';
import { ReloadSession } from './reloadSession';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

export class HotReload implements vscode.Disposable {
    private readonly output = vscode.window.createOutputChannel('FerroUI Hot Reload');
    private readonly log = new ReloadLog(this.output);
    private readonly indicator = vscode.window.createStatusBarItem('ferroui.hotReload', vscode.StatusBarAlignment.Left, 35);
    private readonly subscriptions: vscode.Disposable[];
    private readonly runtimeChanged = new vscode.EventEmitter<void>();
    readonly onDidChangeRuntime = this.runtimeChanged.event;
    runtimeMcp?: RuntimeMcpEndpoint;
    private session?: ReloadSession;
    private choosing = false;
    private generation = 0;
    private message = 'Stopped';

    constructor() {
        this.indicator.name = 'FerroUI Hot Reload';
        this.indicator.command = 'ferroui.hotReload.showOutput';
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.hotReload.start', () => this.startInteractive()),
            vscode.commands.registerCommand('ferroui.hotReload.rebuild', () => this.session ? this.session.build() : this.startInteractive()),
            vscode.commands.registerCommand('ferroui.hotReload.stop', () => this.stop()),
            vscode.commands.registerCommand('ferroui.hotReload.showOutput', () => this.output.show(true)),
        ];
    }

    get state() {
        const target = this.session?.target;
        return { target: target?.label ?? null, pid: this.session?.pid ?? null, status: this.message,
            selection: target ? describeCargoTarget(target) : null, mcpAvailable: this.runtimeMcp !== undefined };
    }

    recentOutput(lines: number) { return { ...this.state, output: this.log.tail(lines) }; }
    async targets(selection: CargoSelection) {
        const targets = await discoverCargoTargets(selection);
        return { targets: targets.map(target => ({ ...describeCargoTarget(target), features: target.features })) };
    }

    async agentControl(action: string, selection: CargoSelection, signal: AbortSignal): Promise<object> {
        if (action === 'start') await this.start(async () => {
            const targets = await discoverCargoTargets(selection);
            if (targets.length !== 1) throw new Error('Choose a single Cargo target from ferroui_reload_targets.');
            return targets[0];
        }, signal);
        else if (action === 'stop') this.stop();
        else if (action === 'show_output') this.output.show(true);
        else if (action === 'rebuild') {
            if (!this.session) throw new Error('Start a managed FerroUI application first.');
            this.session.build();
        }
        return this.state;
    }

    private async startInteractive(): Promise<void> {
        if (this.choosing) return;
        try {
            await this.start(selectCargoTarget);
        } catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(`FerroUI hot reload could not start. ${errorMessage(error)}`);
        }
    }

    private async start(resolveTarget: () => Promise<CargoTarget | undefined>, signal?: AbortSignal): Promise<void> {
        if (this.choosing) throw new Error('A FerroUI Cargo target is already being selected.');
        const generation = ++this.generation;
        this.choosing = true;
        try {
            const target = await resolveTarget();
            signal?.throwIfAborted();
            if (!target || this.generation !== generation) return;
            this.session?.dispose();
            this.log.reset();
            this.session = new ReloadSession(target, this.log, (message, problem) => {
                this.message = message;
                this.indicator.text = `$(flame) ${message}`;
                this.indicator.tooltip = `${target.label} · ${message}\nShow FerroUI Hot Reload output`;
                this.indicator.backgroundColor = problem ? new vscode.ThemeColor('statusBarItem.errorBackground') : undefined;
                this.indicator.show();
            }, endpoint => {
                if (endpoint?.url === this.runtimeMcp?.url && endpoint?.token === this.runtimeMcp?.token) return;
                this.runtimeMcp = endpoint;
                this.runtimeChanged.fire();
            });
            this.message = 'Build queued';
            this.session.build();
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
        this.runtimeChanged.dispose();
    }
}
