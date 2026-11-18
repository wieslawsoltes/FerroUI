import { realpathSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { CargoTarget } from './cargoTarget';
import { isInside } from './workspacePaths';
import { buildCargo, launchArtifact, ReloadProcess, revision, type ProjectMetadata, type RuntimeEvent } from './reloadProcess';
import type { ProcessOutput } from './reloadLog';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

const watchPattern = '**/{*.rs,*.xaml,Cargo.toml}';
const reloadWindow = 1800;

interface SourceChange { file: string; fingerprint: string; observed: number; language?: RuntimeEvent['language'] }

export class ReloadSession implements vscode.Disposable {
    private readonly resources: vscode.Disposable[] = [];
    private readonly changes = new Map<string, SourceChange>();
    private readonly observed = new Map<string, string>();
    private readonly waitingReads = new Map<string, object>();
    private readonly results = new Map<string, RuntimeEvent>();
    private readonly acceptedXaml = new Map<string, string>();
    private readonly languages = new Set<RuntimeEvent['language']>();
    private readonly root: string;
    private readonly targetDirectory: string;
    private live?: ReloadProcess;
    private compiler?: ReloadProcess;
    private candidate?: ReloadProcess;
    private timer?: ReturnType<typeof setTimeout>;
    private building = false;
    private rebuild = false;
    private serial = 0;
    private disposed = false;

    constructor(readonly target: CargoTarget, private readonly output: ProcessOutput,
        private readonly status: (message: string, problem?: boolean) => void,
        private readonly mcpChanged: (endpoint?: RuntimeMcpEndpoint) => void,
        private readonly built: (metadata: ProjectMetadata) => void,
        private readonly activated: (pid: number) => void) {
        this.root = fileKey(target.folder.uri.fsPath);
        this.targetDirectory = fileKey(target.targetDirectory);
        const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(target.folder, watchPattern));
        const observe = (uri: vscode.Uri) => { void this.observe(uri); };
        this.resources.push(watcher, watcher.onDidChange(observe), watcher.onDidCreate(observe), watcher.onDidDelete(observe),
            vscode.workspace.onDidSaveTextDocument(document => observe(document.uri)));
    }

    get pid(): number | undefined { return this.live?.pid; }

    containsXaml(file: string, source: string): boolean {
        return this.live !== undefined && this.acceptedXaml.get(fileKey(file)) === revision(Buffer.from(source));
    }

    build(): void {
        this.rebuild = true;
        this.schedule(0);
    }

    private async observe(uri: vscode.Uri): Promise<void> {
        if (this.disposed || uri.scheme !== 'file') return;
        const file = uri.fsPath;
        const key = fileKey(file);
        if (!isInside(this.root, key) || isInside(this.targetDirectory, key)
            || path.relative(this.root, key).split(path.sep).some(part => ['.git', '.vscode', 'target', 'node_modules'].includes(part))) return;
        const extension = path.extname(file).toLowerCase();
        if (extension !== '.rs' && extension !== '.xaml' && path.basename(file) !== 'Cargo.toml') return;
        const marker = {};
        this.waitingReads.set(key, marker);
        let fingerprint: string;
        try {
            fingerprint = revision(await readFile(file));
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
                this.output.appendLine(`Could not read ${path.basename(file)}. ${String(error)}`);
                return;
            }
            fingerprint = 'deleted';
        } finally {
            if (this.waitingReads.get(key) !== marker) return;
            this.waitingReads.delete(key);
        }
        if (this.disposed || this.observed.get(key) === fingerprint) return;
        this.observed.set(key, fingerprint);
        const language = fingerprint === 'deleted' ? undefined : extension === '.xaml' ? 'xaml' : extension === '.rs' ? 'rsx' : undefined;
        this.changes.set(key, { file, fingerprint, language, observed: Date.now() });
        this.serial++;
        this.schedule(120);
    }

    private accept(event: RuntimeEvent): void {
        if (event.status === 'ready') {
            this.languages.add(event.language);
            return;
        }
        if (!event.file || !path.isAbsolute(event.file)) return;
        const key = fileKey(event.file);
        if (!isInside(this.root, key)) return;
        this.results.set(key, event);
        if (this.changes.has(key)) this.schedule(120);
        // The running app may see a structural edit before VS Code's watcher reports it.
        else if (event.status === 'rebuild') void this.observe(vscode.Uri.file(event.file));
    }

    private schedule(delay: number): void {
        if (this.disposed) return;
        clearTimeout(this.timer);
        this.timer = setTimeout(() => { this.timer = undefined; void this.flush(); }, delay);
    }

    private async flush(): Promise<void> {
        if (this.disposed || this.building) return;
        let wait = 0;
        let applied: string | undefined;
        let failed = false;
        for (const [key, change] of this.changes) {
            const result = this.results.get(key);
            if (result?.revision === change.fingerprint && result.language === change.language) {
                this.results.delete(key);
                this.output.appendLine(`${result.language.toUpperCase()} ${result.status} · ${vscode.workspace.asRelativePath(change.file)}${result.message ? ` · ${result.message}` : ''}`);
                if (result.status === 'rebuild') this.rebuild = true;
                else {
                    this.changes.delete(key);
                    if (result.status === 'failed') {
                        failed = true;
                        this.status(`${result.language.toUpperCase()} edit rejected`, true);
                    } else {
                        if (result.language === 'xaml') this.acceptedXaml.set(key, change.fingerprint);
                        applied = `${result.language.toUpperCase()} reloaded`;
                    }
                }
            } else if (!this.live || !change.language || !this.languages.has(change.language)) this.rebuild = true;
            else {
                const remaining = reloadWindow - (Date.now() - change.observed);
                if (remaining <= 0) this.rebuild = true;
                else wait = Math.max(wait, remaining);
            }
        }
        if (this.rebuild) {
            this.rebuild = false;
            await this.replace();
        } else if (wait) {
            this.status('Waiting for the running app');
            this.schedule(wait);
        } else if (applied && !failed) {
            this.status(applied);
        }
    }

    private async replace(): Promise<void> {
        const serial = this.serial;
        const previous = this.live;
        this.building = true;
        this.output.appendLine(`Building ${this.target.label}${previous ? ` · keeping PID ${previous.pid} running` : ''}`);
        this.status(previous ? 'Building update' : 'Building application');
        try {
            const build = buildCargo(this.target, this.output);
            this.compiler = build.process;
            const artifact = await build.artifact;
            this.compiler = undefined;
            if (this.superseded(serial)) return;
            this.built(build.metadata());
            const candidateEvents: RuntimeEvent[] = [];
            let endpoint: RuntimeMcpEndpoint | undefined;
            let live = false;
            const candidate = await launchArtifact(artifact, this.target, this.output, event => {
                if (this.disposed) return;
                if (!live) candidateEvents.push(event);
                else if (this.live === candidate.process) this.accept(event);
            }, value => {
                if (this.disposed) return;
                if (!live) endpoint = value;
                else if (this.live === candidate.process) this.mcpChanged(value);
            });
            this.candidate = candidate.process;
            if (this.superseded(serial)) return;
            await awaitStartup(candidate.process, candidate.ready);
            if (this.superseded(serial)) return;
            this.live = candidate.process;
            this.candidate = undefined;
            this.changes.clear();
            this.results.clear();
            this.acceptedXaml.clear();
            this.languages.clear();
            live = true;
            this.mcpChanged(endpoint);
            for (const event of candidateEvents) this.accept(event);
            if (this.live.pid) this.activated(this.live.pid);
            previous?.stop();
            this.output.appendLine(`Running ${this.target.label} · PID ${this.live.pid}${previous ? ' · rebuilt and restarted' : ''}`);
            this.status(previous ? 'Rebuilt and restarted' : 'Watching saves');
            void this.live.closed.then(code => {
                if (this.disposed || this.live !== candidate.process) return;
                this.live = undefined;
                this.mcpChanged();
                this.languages.clear();
                this.output.appendLine(`Application exited${code === null ? '' : ` (${code})`}.`);
                this.status('Application stopped', Boolean(code));
            }, error => this.output.appendLine(String(error)));
        } catch (error) {
            if (this.disposed || this.serial !== serial) return;
            this.changes.clear();
            this.output.appendLine(`${String(error)}${this.live ? ' The previous application is still running.' : ''}`);
            this.status(this.live ? 'Update failed · app still running' : 'Launch failed', true);
        } finally {
            this.compiler = undefined;
            this.candidate?.stop();
            this.candidate = undefined;
            this.building = false;
            if (!this.disposed && (this.serial !== serial || this.rebuild)) {
                if (!this.live) this.rebuild = true;
                this.schedule(120);
            }
        }
    }

    private superseded(serial: number): boolean {
        if (this.disposed) return true;
        if (this.serial === serial) return false;
        this.output.appendLine('A newer save superseded the candidate.');
        return true;
    }

    dispose(): void {
        this.disposed = true;
        this.mcpChanged();
        clearTimeout(this.timer);
        this.resources.forEach(resource => resource.dispose());
        this.compiler?.stop();
        this.candidate?.stop();
        this.live?.stop();
    }
}

function fileKey(file: string): string {
    let real: string;
    try { real = realpathSync.native(file); } catch { real = path.resolve(file); }
    return process.platform === 'win32' ? real.toLowerCase() : real;
}

async function awaitStartup(child: ReloadProcess, acknowledged: Promise<void>): Promise<void> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const exited = child.closed.then(code => { throw new Error(`Candidate exited before it opened a window (${code ?? 'terminated'}).`); });
    try {
        const opened = acknowledged.then(() => new Promise<void>(resolve => setTimeout(resolve, 100)));
        await Promise.race([opened, exited, new Promise<never>((_, reject) => {
            timer = setTimeout(() => reject(new Error('No FerroUI runtime acknowledgement. Enable hot reload in the selected Cargo binary.')), 15_000);
        })]);
    } finally {
        clearTimeout(timer);
    }
}
