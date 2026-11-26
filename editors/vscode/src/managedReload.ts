import { randomUUID } from 'node:crypto';
import * as vscode from 'vscode';
import { describeCargoTarget, type CargoTarget } from './cargoTarget';
import { RegisteredReload } from './registeredReload';
import { ReloadDebugger } from './reloadDebugger';
import type { ReloadHistory } from './reloadHistory';
import { ReloadLog } from './reloadLog';
import type { ProjectMetadata } from './reloadProcess';
import { ReloadSession } from './reloadSession';
import type { RuntimeMcpEndpoint } from './runtimeMcp';
import type { AppRuntime } from './runtimeTargets';
import { fileKey } from './workspacePaths';

export type ReloadRecipe = { kind: 'cargo'; target: CargoTarget } | { kind: 'registered'; folder: vscode.WorkspaceFolder };

export function recipeKey(recipe: ReloadRecipe): string {
    if (recipe.kind === 'registered') return JSON.stringify(['registered', fileKey(recipe.folder.uri.fsPath)]);
    const { target } = recipe;
    return JSON.stringify(['cargo', fileKey(target.cargoDirectory), fileKey(target.manifest), target.packageId, target.binary,
        [...target.features].sort(), target.args, target.cargo, fileKey(target.targetDirectory)]);
}

export class ManagedReload {
    readonly id = randomUUID();
    readonly runtime: AppRuntime = `managed:${this.id}`;
    readonly key: string;
    readonly label: string;
    readonly folder: vscode.WorkspaceFolder;
    private readonly output: vscode.OutputChannel;
    private readonly log: ReloadLog;
    private readonly debugger: ReloadDebugger;
    private session?: ReloadSession;
    private registered?: RegisteredReload;
    private starting?: Promise<void>;
    private closing?: Promise<void>;
    private stopping = false;
    private message = 'Build queued';
    private problem = false;
    endpoint?: RuntimeMcpEndpoint;
    private endpointProcess?: number;

    constructor(readonly recipe: ReloadRecipe, private readonly history: ReloadHistory,
        private readonly changed: () => void, private readonly connectionChanged: () => void,
        private readonly built: (metadata: ProjectMetadata) => void) {
        this.key = recipeKey(recipe);
        this.folder = recipe.kind === 'cargo' ? recipe.target.folder : recipe.folder;
        this.label = recipe.kind === 'cargo' ? recipe.target.label : `${recipe.folder.name} · Registered patches`;
        this.output = vscode.window.createOutputChannel(`FerroUI Hot Reload · ${this.label} (${this.id.slice(0, 8)})`);
        this.log = new ReloadLog(this.output);
        this.debugger = new ReloadDebugger(this.output, { onDidChange: changed });
    }

    get pid(): number | undefined { return this.session?.pid ?? this.registered?.pid; }
    get processGeneration(): number | undefined { return this.session?.processGeneration ?? this.registered?.processGeneration; }
    get debuggerStatus() { return this.debugger.statusFor(this.processGeneration); }

    get state() {
        const { attached, paused } = this.debuggerStatus;
        return { session: this.id, runtime: this.runtime, target: this.label, folder: this.folder.uri.toString(),
            kind: this.recipe.kind, pid: this.pid ?? null, status: this.message, problem: this.problem,
            process_generation: this.processGeneration ?? null, debuggerAttached: attached, debuggerPaused: paused,
            selection: this.recipe.kind === 'cargo' ? { ...describeCargoTarget(this.recipe.target), features: this.recipe.target.features } : null,
            mcpAvailable: this.endpoint !== undefined };
    }

    observe(uri: vscode.Uri, fromWatcher: boolean): void { void this.session?.observe(uri, fromWatcher); }
    xamlRevision(file: string): string | undefined { return this.session?.xamlRevision(file); }
    recentOutput(lines: number): string { return this.log.tail(lines); }
    showOutput(): void { this.output.show(true); }
    ensureStarted(): void { this.session?.ensureStarted(); }

    rebuild(): void {
        if (!this.session) throw new Error('Use the registered supervisor to rebuild this session.');
        this.session.build();
    }

    async attach(configuration: vscode.DebugConfiguration, generation?: number): Promise<void> {
        if (this.stopping) throw new Error('The app session stopped.');
        if (generation !== undefined) this.requireProcess(generation);
        const pid = this.pid;
        const currentGeneration = this.processGeneration;
        if (pid && currentGeneration !== undefined) await this.debugger.attachToRunningApp(this.folder, configuration, {
            pid, generation: currentGeneration, requireCurrent: () => this.requireProcess(currentGeneration),
        });
        else this.debugger.prepare(this.folder, configuration);
    }

    async detachDebugger(generation: number | undefined): Promise<void> {
        this.requireProcess(generation);
        await this.debugger.detachFromRunningApp();
    }

    continueDebugger(generation: number, signal?: AbortSignal): Promise<void> {
        this.requireProcess(generation);
        return this.debugger.continueStopped(signal);
    }

    private requireProcess(generation: number | undefined): void {
        if (this.stopping || generation === undefined || generation !== this.processGeneration) {
            throw new Error('Read the current accepted process generation with ferroui_reload_status.');
        }
    }

    start(configuration?: vscode.DebugConfiguration): Promise<void> {
        if (configuration) this.debugger.prepare(this.folder, configuration);
        if (this.recipe.kind === 'registered') {
            this.report('Starting registered target');
            this.starting = RegisteredReload.start(this.folder, this.log, (message, process, problem) => {
                this.report(message, problem);
                if (!this.stopping && process) this.debugger.activated(process.pid, undefined, process.generation);
            }).then(async registered => {
                if (this.stopping) await registered.dispose();
                else { this.registered = registered; this.changed(); }
            });
            return this.starting;
        }
        this.session = new ReloadSession(this.recipe.target, { session: this.id, target: this.label },
            this.log, this.history, (message, problem) => this.report(message, problem), (endpoint, processGeneration) => {
                if (this.stopping || (processGeneration === this.endpointProcess
                    && endpoint?.url === this.endpoint?.url && endpoint?.token === this.endpoint?.token)) return;
                this.endpoint = endpoint && { ...endpoint };
                this.endpointProcess = processGeneration;
                this.connectionChanged();
            }, this.built, (pid, generation) => this.debugger.activated(pid, undefined, generation));
        this.session.build();
        return Promise.resolve();
    }

    private report(message: string, problem = false): void {
        if (this.stopping) return;
        this.message = message;
        this.problem = problem;
        this.changed();
    }

    stop(): Promise<void> {
        if (this.closing) return this.closing;
        this.stopping = true;
        this.closing = Promise.all([
            this.debugger.close(), this.session?.dispose(), this.registered?.dispose(), this.starting?.catch(() => undefined),
        ]).then(() => this.output.dispose());
        return this.closing;
    }
}
