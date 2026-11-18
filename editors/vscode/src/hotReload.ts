import * as vscode from 'vscode';
import { describeCargoTarget, discoverCargoTargets, selectCargoTarget, type CargoSelection, type CargoTarget } from './cargoTarget';
import { errorMessage } from './protocol';
import { ReloadLog } from './reloadLog';
import { managedProcessVariable, ReloadDebugger } from './reloadDebugger';
import { ReloadHistory } from './reloadHistory';
import { RegisteredReload } from './registeredReload';
import type { ProjectMetadata } from './reloadProcess';
import { ReloadSession } from './reloadSession';
import type { RuntimeMcpEndpoint } from './runtimeMcp';
import { localFolders, workspaceFolders } from './workspacePaths';

export class HotReload implements vscode.Disposable {
    readonly history = new ReloadHistory();
    private readonly output = vscode.window.createOutputChannel('FerroUI Hot Reload');
    private readonly log = new ReloadLog(this.output);
    private readonly debugger = new ReloadDebugger(this.output);
    private readonly indicator = vscode.window.createStatusBarItem('ferroui.hotReload', vscode.StatusBarAlignment.Left, 35);
    private readonly subscriptions: vscode.Disposable[];
    private readonly runtimeChanged = new vscode.EventEmitter<void>();
    readonly onDidChangeRuntime = this.runtimeChanged.event;
    private readonly built = new vscode.EventEmitter<ProjectMetadata>();
    readonly onDidBuild = this.built.event;
    runtimeMcp?: RuntimeMcpEndpoint;
    private session?: ReloadSession;
    private registered?: RegisteredReload;
    private choosing = false;
    private generation = 0;
    private message = 'Stopped';
    private shutdown = Promise.resolve();

    constructor() {
        this.indicator.name = 'FerroUI Hot Reload';
        this.indicator.command = 'ferroui.hotReload.showOutput';
        this.subscriptions = [
            vscode.commands.registerCommand('ferroui.hotReload.start', () => this.startInteractive()),
            vscode.commands.registerCommand('ferroui.hotReload.runAndDebug', () => this.startInteractive(true)),
            vscode.commands.registerCommand('ferroui.hotReload.startRegistered', () => this.startRegisteredInteractive()),
            vscode.commands.registerCommand('ferroui.hotReload.rebuild', () => {
                if (this.session) return this.session.build();
                if (this.registered) return this.output.show(true);
                return this.startInteractive();
            }),
            vscode.commands.registerCommand('ferroui.hotReload.stop', () => this.stop()),
            vscode.commands.registerCommand('ferroui.hotReload.showOutput', () => this.output.show(true)),
            vscode.commands.registerCommand('ferroui.hotReload.debug', () => this.attachDebugger()),
            vscode.commands.registerCommand('ferroui.hotReload.processId', () => String(this.requirePid())),
        ];
    }

    get state() {
        const target = this.session?.target;
        return { target: target?.label ?? (this.registered ? 'Registered patch supervisor' : null), pid: this.session?.pid ?? this.registered?.pid ?? null, status: this.message,
            selection: target ? describeCargoTarget(target) : null, mcpAvailable: this.runtimeMcp !== undefined };
    }

    recentOutput(lines: number) { return { ...this.state, output: this.log.tail(lines) }; }
    containsXaml(file: string, source: string): boolean { return this.session?.containsXaml(file, source) ?? false; }
    async targets(selection: CargoSelection) {
        const targets = await discoverCargoTargets(selection);
        return { targets: targets.map(target => ({ ...describeCargoTarget(target), features: target.features })) };
    }

    async agentControl(action: string, selection: CargoSelection, signal: AbortSignal, debugConfiguration?: string): Promise<object> {
        if (action === 'start' || action === 'start_debug') await this.start(async () => {
            const targets = await discoverCargoTargets(selection);
            if (targets.length !== 1) throw new Error('Choose a single Cargo target from ferroui_reload_targets.');
            return targets[0];
        }, signal, action === 'start_debug' ? folder => selectDebugConfiguration(folder, false, debugConfiguration) : undefined);
        else if (action === 'start_registered') {
            const folders = workspaceFolders(selection.folder);
            if (folders.length !== 1) throw new Error('Choose a single workspace folder for the registered supervisor.');
            await this.startRegistered(folders[0], signal);
        }
        else if (action === 'stop') await this.stop();
        else if (action === 'show_output') this.output.show(true);
        else if (action === 'rebuild') {
            if (!this.session) throw new Error('Start a managed FerroUI application first.');
            this.session.build();
        }
        return this.state;
    }

    private async startInteractive(debug = false): Promise<void> {
        if (this.choosing) return;
        try {
            await this.start(() => selectCargoTarget(), undefined, debug ? selectDebugConfiguration : undefined);
        } catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(`FerroUI hot reload could not start. ${errorMessage(error)}`);
        }
    }

    private requirePid(): number {
        const pid = this.session?.pid ?? this.registered?.pid;
        if (!pid) throw new Error('Start a managed FerroUI application first.');
        return pid;
    }

    private async attachDebugger(): Promise<void> {
        try {
            const pid = this.requirePid();
            const folder = this.session?.target.folder ?? this.registered!.folder;
            const selected = await selectDebugConfiguration(folder);
            if (selected && this.requirePid() === pid) await this.debugger.attach(folder, selected, pid);
        } catch (error) {
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    private async start(resolveTarget: () => Promise<CargoTarget | undefined>, signal?: AbortSignal,
        debug?: (folder: vscode.WorkspaceFolder) => Promise<vscode.DebugConfiguration | undefined>): Promise<void> {
        if (this.choosing) throw new Error('A FerroUI Cargo target is already being selected.');
        const generation = ++this.generation;
        this.choosing = true;
        try {
            const target = await resolveTarget();
            signal?.throwIfAborted();
            if (!target || this.generation !== generation) return;
            if (debug) {
                const configuration = await debug(target.folder);
                signal?.throwIfAborted();
                if (!configuration || this.generation !== generation) return;
                this.debugger.prepare(target.folder, configuration);
            } else this.debugger.detach();
            void this.retireProcesses();
            this.log.reset();
            this.session = new ReloadSession(target, this.log, this.history, (message, problem) => {
                this.report(target.label, message, problem);
            }, endpoint => {
                if (endpoint?.url === this.runtimeMcp?.url && endpoint?.token === this.runtimeMcp?.token) return;
                this.runtimeMcp = endpoint;
                this.runtimeChanged.fire();
            }, metadata => this.built.fire(metadata), pid => this.debugger.activated(pid));
            this.message = 'Build queued';
            this.session.build();
        } finally {
            this.choosing = false;
        }
    }

    private async startRegisteredInteractive(): Promise<void> {
        try {
            const document = vscode.window.activeTextEditor?.document;
            const folders = localFolders();
            const folder = (document && vscode.workspace.getWorkspaceFolder(document.uri))
                ?? (folders.length === 1 ? folders[0] : await vscode.window.showWorkspaceFolderPick());
            if (folder) await this.startRegistered(folder);
        } catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    private async startRegistered(folder: vscode.WorkspaceFolder, signal?: AbortSignal): Promise<void> {
        if (this.choosing) throw new Error('A FerroUI target is already being selected.');
        this.choosing = true;
        try {
            const generation = this.generation + 1;
            signal?.throwIfAborted();
            const session = await RegisteredReload.start(folder, this.log, (message, pid, problem) => {
                if (this.generation !== generation) return;
                this.report('Registered patch supervisor', message, problem);
                if (pid) this.debugger.activated(pid);
            });
            if (signal?.aborted || generation !== this.generation + 1) { session.dispose(); signal?.throwIfAborted(); return; }
            this.stop();
            this.registered = session;
            this.log.reset();
            this.report('Registered patch supervisor', 'Starting registered target');
        } finally { this.choosing = false; }
    }

    private report(target: string, message: string, problem?: boolean): void {
        this.message = message;
        this.indicator.text = `$(flame) ${message}`;
        this.indicator.tooltip = `${target} · ${message}\nShow FerroUI Hot Reload output`;
        this.indicator.backgroundColor = problem ? new vscode.ThemeColor('statusBarItem.errorBackground') : undefined;
        this.indicator.show();
    }

    stop(): Promise<void> {
        this.generation++;
        this.debugger.detach();
        this.message = 'Stopped';
        this.indicator.hide();
        return this.retireProcesses();
    }

    private retireProcesses(): Promise<void> {
        const previous = [this.session?.dispose(), this.registered?.dispose()];
        this.session = undefined;
        this.registered = undefined;
        this.shutdown = Promise.all([this.shutdown, ...previous]).then(() => undefined);
        return this.shutdown;
    }

    dispose(): void {
        void this.stop();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.indicator.dispose();
        this.debugger.dispose();
        this.output.dispose();
        this.runtimeChanged.dispose();
        this.built.dispose();
        this.history.dispose();
    }
}

async function selectDebugConfiguration(folder: vscode.WorkspaceFolder, interactive = true, name?: string): Promise<vscode.DebugConfiguration | undefined> {
    const configurations = vscode.workspace.getConfiguration('launch', folder.uri)
        .get<vscode.DebugConfiguration[]>('configurations', [])
        .filter(configuration => configuration.request === 'attach' && JSON.stringify(configuration).includes(managedProcessVariable));
    if (!configurations.length) throw new Error(`Add a Rust attach configuration with ${managedProcessVariable} as its process ID.`);
    if (name !== undefined) {
        const matches = configurations.filter(configuration => configuration.name === name);
        if (matches.length !== 1) throw new Error('Choose one attach configuration by its name in launch.json.');
        return matches[0];
    }
    if (configurations.length === 1) return configurations[0];
    if (!interactive) throw new Error('Specify debug_configuration using the attach configuration name in launch.json.');
    return (await vscode.window.showQuickPick(configurations.map(configuration => ({ label: configuration.name, configuration })),
        { placeHolder: 'Debug the managed FerroUI application' }))?.configuration;
}
