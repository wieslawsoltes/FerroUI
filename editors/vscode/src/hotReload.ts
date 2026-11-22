import * as vscode from 'vscode';
import { describeCargoTarget, discoverCargoTargets, selectCargoTarget, type CargoSelection } from './cargoTarget';
import { ManagedReload, recipeKey, type ReloadRecipe } from './managedReload';
import { errorMessage } from './protocol';
import { managedProcessVariable } from './reloadDebugger';
import { ReloadHistory } from './reloadHistory';
import type { ProjectMetadata } from './reloadProcess';
import { reloadWatchPattern } from './reloadScope';
import { localFolders, workspaceFolders } from './workspacePaths';

const stopped = { target: null, pid: null, status: 'Stopped', selection: null, mcpAvailable: false };

export class HotReload implements vscode.Disposable {
    readonly history = new ReloadHistory();
    private readonly output = vscode.window.createOutputChannel('FerroUI Hot Reload');
    private readonly indicator = vscode.window.createStatusBarItem('ferroui.hotReload', vscode.StatusBarAlignment.Left, 35);
    private readonly subscriptions: vscode.Disposable[];
    private readonly runtimeChanged = new vscode.EventEmitter<void>();
    readonly onDidChangeRuntime = this.runtimeChanged.event;
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeRuns = this.changed.event;
    private readonly built = new vscode.EventEmitter<ProjectMetadata>();
    readonly onDidBuild = this.built.event;
    private readonly live = new Map<string, ManagedReload>();
    private readonly retiring = new Map<string, Promise<void>>();
    private readonly byRecipe = new Map<string, ManagedReload>();
    private choosing = false;
    private disposed = false;
    private selectionEpoch = 0;
    active?: string;

    constructor() {
        this.indicator.name = 'FerroUI Hot Reload';
        this.indicator.command = 'ferroui.hotReload.select';
        const watcher = vscode.workspace.createFileSystemWatcher(reloadWatchPattern);
        const observe = (uri: vscode.Uri, fromWatcher = false) => { for (const run of this.live.values()) run.observe(uri, fromWatcher); };
        const watched = (uri: vscode.Uri) => observe(uri, true);
        this.subscriptions = [
            watcher, watcher.onDidChange(watched), watcher.onDidCreate(watched), watcher.onDidDelete(watched),
            vscode.workspace.onDidSaveTextDocument(document => observe(document.uri)),
            vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
                for (const run of this.live.values()) {
                    if (removed.some(folder => folder.uri.toString() === run.folder.uri.toString())) void this.stopOne(run);
                }
            }),
            vscode.commands.registerCommand('ferroui.hotReload.start', () => this.interactive(() => this.startInteractive())),
            vscode.commands.registerCommand('ferroui.hotReload.runAndDebug', () => this.interactive(() => this.startInteractive(true))),
            vscode.commands.registerCommand('ferroui.hotReload.startRegistered', () => this.interactive(() => this.startRegisteredInteractive())),
            vscode.commands.registerCommand('ferroui.hotReload.rebuild', (id?: string) => this.interactive(async () => {
                const run = this.find(id);
                if (run) run.rebuild();
                else await this.startInteractive();
            })),
            vscode.commands.registerCommand('ferroui.hotReload.stop', (id?: string) => this.interactive(async () => {
                const run = this.find(id);
                if (run) await this.stopOne(run);
            })),
            vscode.commands.registerCommand('ferroui.hotReload.stopAll', () => this.stop()),
            vscode.commands.registerCommand('ferroui.hotReload.select', (id?: string) => this.interactive(() => this.pick(id))),
            vscode.commands.registerCommand('ferroui.hotReload.showOutput', (id?: string) => this.interactive(async () => {
                const run = this.find(id);
                if (run) run.showOutput();
                else this.output.show(true);
            })),
            vscode.commands.registerCommand('ferroui.hotReload.debug', (id?: string) => this.interactive(async () => {
                const run = this.require(id);
                const configuration = await selectDebugConfiguration(run.folder);
                if (configuration && this.live.get(run.id) === run) await run.attach(configuration);
            })),
            vscode.commands.registerCommand('ferroui.hotReload.processId', () => {
                const pid = this.require().pid;
                if (!pid) throw new Error('Wait for the selected FerroUI application to start.');
                return String(pid);
            }),
        ];
    }

    get state() { return this.stateFor(); }

    runs(): ManagedReload[] { return [...this.live.values()]; }

    stateFor(session?: string) { return this.find(session)?.state ?? stopped; }

    recentOutput(lines: number, session?: string) {
        return { ...this.stateFor(session), output: this.find(session)?.recentOutput(lines) ?? '' };
    }

    xamlRevision(file: string, session?: string): string | undefined {
        return this.live.get(session ?? this.active ?? '')?.xamlRevision(file);
    }

    async targets(selection: CargoSelection) {
        const targets = await discoverCargoTargets(selection);
        return { targets: targets.map(target => ({ ...describeCargoTarget(target), features: target.features })) };
    }

    async agentControl(action: string, selection: CargoSelection, signal: AbortSignal, debugConfiguration?: string, session?: string): Promise<object> {
        const epoch = this.selectionEpoch;
        signal.throwIfAborted();
        if (action === 'start' || action === 'start_debug') {
            const targets = await discoverCargoTargets(selection);
            if (targets.length !== 1) throw new Error('Choose a single Cargo target from ferroui_reload_targets.');
            const target = targets[0];
            const configuration = action === 'start_debug' ? await selectDebugConfiguration(target.folder, false, debugConfiguration) : undefined;
            return (await this.launch({ kind: 'cargo', target }, epoch, configuration, signal)).state;
        }
        if (action === 'start_registered') {
            const folders = workspaceFolders(selection.folder);
            if (folders.length !== 1) throw new Error('Choose a single workspace folder for the registered supervisor.');
            return (await this.launch({ kind: 'registered', folder: folders[0] }, epoch, undefined, signal)).state;
        }
        if (action === 'stop_all') { await this.stop(); return this.state; }
        if (action === 'stop' && !session && !this.live.size) return stopped;
        if (action === 'show_output' && !session && !this.live.size) { this.output.show(true); return stopped; }
        if (!session && this.live.size > 1) throw new Error('Pass app_session from ferroui_app_sessions to address a running app.');
        const run = this.require(session);
        switch (action) {
            case 'select': this.select(run); break;
            case 'stop': await this.stopOne(run); return { ...stopped, session: run.id };
            case 'rebuild': run.rebuild(); break;
            case 'show_output': run.showOutput(); break;
            case 'attach_debug': {
                const configuration = await selectDebugConfiguration(run.folder, false, debugConfiguration);
                signal.throwIfAborted();
                if (configuration) await run.attach(configuration);
                break;
            }
        }
        return run.state;
    }

    private find(id?: string): ManagedReload | undefined {
        const run = this.live.get(id ?? this.active ?? '');
        if (!run && id !== undefined) throw new Error('This FerroUI app session has ended. Choose a current session.');
        return run;
    }

    private require(id?: string): ManagedReload {
        const run = this.find(id);
        if (!run) throw new Error('Start a managed FerroUI application first.');
        return run;
    }

    private async pick(id?: string): Promise<void> {
        if (id) { this.select(this.require(id)); return; }
        if (this.live.size < 2) { this.find()?.showOutput(); return; }
        const choices = this.runs().map(run => ({ label: run.label, description: run.state.status,
            detail: run.pid !== undefined ? `PID ${run.pid} · ${run.folder.name}` : run.folder.name, run }));
        const selected = await vscode.window.showQuickPick(choices, { title: 'Select FerroUI App Session', matchOnDetail: true });
        if (selected && this.live.get(selected.run.id) === selected.run) this.select(selected.run);
    }

    private select(run: ManagedReload): void {
        this.active = run.id;
        this.refresh();
        this.runtimeChanged.fire();
    }

    private async startInteractive(debug = false): Promise<void> {
        if (this.choosing) return;
        this.choosing = true;
        const epoch = this.selectionEpoch;
        try {
            const target = await selectCargoTarget();
            if (!target) return;
            const configuration = debug ? await selectDebugConfiguration(target.folder) : undefined;
            if (debug && !configuration) return;
            await this.launch({ kind: 'cargo', target }, epoch, configuration);
        } finally { this.choosing = false; }
    }

    private async startRegisteredInteractive(): Promise<void> {
        const epoch = this.selectionEpoch;
        const document = vscode.window.activeTextEditor?.document;
        const folders = localFolders();
        const folder = (document && vscode.workspace.getWorkspaceFolder(document.uri))
            ?? (folders.length === 1 ? folders[0] : await vscode.window.showWorkspaceFolderPick());
        if (folder) await this.launch({ kind: 'registered', folder }, epoch);
    }

    private async launch(recipe: ReloadRecipe, epoch: number, configuration?: vscode.DebugConfiguration, signal?: AbortSignal): Promise<ManagedReload> {
        const key = recipeKey(recipe);
        await this.retiring.get(key);
        signal?.throwIfAborted();
        if (this.disposed || this.selectionEpoch !== epoch || !vscode.workspace.isTrusted) {
            throw new Error('The workspace app selection changed before launch.');
        }
        const folder = recipe.kind === 'cargo' ? recipe.target.folder : recipe.folder;
        if (!localFolders().some(item => item.uri.toString() === folder.uri.toString())) throw new Error('The selected workspace folder has closed.');
        const existing = this.byRecipe.get(key);
        if (existing) {
            this.select(existing);
            if (configuration) await existing.attach(configuration);
            existing.ensureStarted();
            return existing;
        }
        const run = new ManagedReload(recipe, this.history, () => this.refresh(), () => this.runtimeChanged.fire(), meta => this.built.fire(meta));
        this.live.set(run.id, run);
        this.byRecipe.set(run.key, run);
        this.select(run);
        try {
            await run.start(configuration);
            signal?.throwIfAborted();
            if (this.live.get(run.id) !== run) throw new Error('The app session stopped before launch completed.');
            return run;
        } catch (error) { await this.stopOne(run); throw error; }
    }

    private stopOne(run: ManagedReload): Promise<void> {
        if (this.live.get(run.id) !== run) return this.retiring.get(run.key) ?? Promise.resolve();
        this.live.delete(run.id);
        this.byRecipe.delete(run.key);
        if (this.active === run.id) this.active = this.runs().at(-1)?.id;
        const cleanup = run.stop().finally(() => { this.retiring.delete(run.key); });
        this.retiring.set(run.key, cleanup);
        this.refresh();
        this.runtimeChanged.fire();
        return cleanup;
    }

    async stop(): Promise<void> {
        this.selectionEpoch++;
        await Promise.all([...this.retiring.values(), ...this.runs().map(run => this.stopOne(run))]);
    }

    private refresh(): void {
        const run = this.find();
        if (run) {
            const { status, problem } = run.state;
            this.indicator.text = `$(flame) ${this.live.size === 1 ? status : `${run.label} · ${this.live.size} apps`}`;
            this.indicator.tooltip = `${run.label} · ${status}\n${this.live.size === 1 ? 'Show FerroUI Hot Reload output' : 'Select a FerroUI app session'}`;
            this.indicator.backgroundColor = problem ? new vscode.ThemeColor('statusBarItem.errorBackground') : undefined;
            this.indicator.show();
        } else this.indicator.hide();
        this.changed.fire();
    }

    private async interactive(action: () => Promise<void>): Promise<void> {
        try { await action(); }
        catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(errorMessage(error));
        }
    }

    dispose(): void {
        this.disposed = true;
        this.subscriptions.forEach(subscription => subscription.dispose());
        void this.stop().finally(() => {
            this.indicator.dispose();
            this.output.dispose();
            this.runtimeChanged.dispose();
            this.built.dispose();
            this.changed.dispose();
            this.history.dispose();
        });
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
        { placeHolder: 'Debug this FerroUI application session' }))?.configuration;
}
