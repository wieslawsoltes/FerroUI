import { open, readFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { inCargoBuildSlot } from './cargoBuildGate';
import type { CargoTarget } from './cargoTarget';
import { buildCargo, launchArtifact, ReloadProcess, revision, type ProjectMetadata, type RuntimeEvent } from './reloadProcess';
import type { ReloadHistory, ReloadIdentity, ReloadRecord } from './reloadHistory';
import type { RuntimeFrameReceipt } from './reloadFrames';
import type { ProcessOutput } from './reloadLog';
import { ReloadScope } from './reloadScope';
import { fileKey } from './workspacePaths';
import type { RuntimeMcpEndpoint } from './runtimeMcp';

const reloadWindow = 1800;

interface SourceChange {
    file: string;
    fingerprint: string;
    xamlFingerprint?: string;
    observed: number;
    language?: RuntimeEvent['language'];
    record: ReloadRecord;
}

type LiveEvent = RuntimeEvent & { generation: number; pid?: number };

export class ReloadSession implements vscode.Disposable {
    private readonly lifetime = new AbortController();
    private readonly watchingSince = Date.now();
    private readonly changes = new Map<string, SourceChange>();
    private readonly observed = new Map<string, string>();
    private readonly waitingReads = new Map<string, object>();
    private readonly results = new Map<string, LiveEvent>();
    private readonly acceptedXaml = new Map<string, string>();
    private readonly languages = new Set<RuntimeEvent['language']>();
    private readonly scope: ReloadScope;
    private live?: ReloadProcess;
    private liveGeneration?: number;
    private compiler?: ReloadProcess;
    private candidate?: ReloadProcess;
    private replacement?: Promise<void>;
    private timer?: ReturnType<typeof setTimeout>;
    private building = false;
    private rebuild = false;
    private serial = 0;
    private disposed = false;

    constructor(readonly target: CargoTarget, private readonly identity: ReloadIdentity,
        private readonly output: ProcessOutput, private readonly history: ReloadHistory,
        private readonly status: (message: string, problem?: boolean) => void,
        private readonly mcpChanged: (endpoint?: RuntimeMcpEndpoint, processGeneration?: number) => void,
        private readonly built: (metadata: ProjectMetadata) => void,
        private readonly activated: (pid: number) => void) {
        this.scope = new ReloadScope(target);
    }

    get pid(): number | undefined { return this.live?.pid; }

    xamlRevision(file: string): string | undefined {
        return this.live ? this.acceptedXaml.get(fileKey(file)) : undefined;
    }

    build(): void {
        this.rebuild = true;
        this.schedule(0);
    }

    ensureStarted(): void {
        if (!this.disposed && !this.live && !this.building && !this.timer) this.build();
    }

    async observe(uri: vscode.Uri, fromWatcher = false): Promise<void> {
        if (this.disposed || uri.scheme !== 'file') return;
        const file = uri.fsPath;
        const key = fileKey(file);
        if (!this.scope.observes(key)) return;
        const extension = path.extname(file).toLowerCase();
        if (extension !== '.rs' && extension !== '.xaml' && path.basename(file) !== 'Cargo.toml') return;
        const marker = {};
        this.waitingReads.set(key, marker);
        let fingerprint: string;
        let xamlFingerprint: string | undefined;
        try {
            const bytes = fromWatcher ? await this.watchedBytes(file) : await readFile(file);
            if (bytes === undefined) return;
            fingerprint = revision(bytes);
            // The protocol acknowledges the file bytes. XAML source positions and the
            // loaded VS Code document exclude the leading UTF-8 signature.
            if (extension === '.xaml' && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
                xamlFingerprint = revision(bytes.subarray(3));
            }
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
        const previous = this.changes.get(key);
        if (previous) this.history.finish(previous.record, 'superseded', this.pid);
        const observed = Date.now();
        const record = this.history.observe(file, fingerprint, observed, this.identity);
        this.changes.set(key, { file, fingerprint, xamlFingerprint, language, observed, record });
        this.serial++;
        this.schedule(120);
    }

    private async watchedBytes(file: string): Promise<Buffer | undefined> {
        const handle = await open(file, 'r');
        try {
            const info = await handle.stat();
            const changed = Math.max(info.ctimeMs, info.mtimeMs, info.birthtimeMs);
            // Native watchers can deliver a batch after launch. The initial
            // Cargo build already sees files written before the session starts.
            if (changed > 0 && changed < this.watchingSince) return;
            return await handle.readFile();
        } finally { await handle.close(); }
    }

    private accept(event: RuntimeEvent): void {
        if (event.status === 'ready') {
            this.languages.add(event.language);
            return;
        }
        if (this.liveGeneration === undefined || !event.file || !path.isAbsolute(event.file)) return;
        const key = fileKey(event.file);
        if (!this.scope.observeRuntime(key)) return;
        this.results.set(key, { ...event, generation: this.liveGeneration, pid: this.pid });
        if (this.changes.has(key)) this.schedule(120);
        // The running app may see a structural edit before VS Code's watcher reports it.
        else void this.observe(vscode.Uri.file(event.file));
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
                        this.history.finish(change.record, 'rejected', result.pid, result.message, result);
                        failed = true;
                        this.status(`${result.language.toUpperCase()} edit rejected`, true);
                    } else {
                        this.history.finish(change.record, 'applied', result.pid, result.message, result);
                        if (result.language === 'xaml') this.acceptedXaml.set(key, change.xamlFingerprint ?? change.fingerprint);
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
            const replacement = this.replace();
            this.replacement = replacement;
            try { await replacement; }
            finally { if (this.replacement === replacement) this.replacement = undefined; }
        } else if (wait) {
            this.status('Waiting for the running app');
            this.schedule(wait);
        } else if (applied && !failed) {
            this.status(applied);
        }
    }

    private async replace(): Promise<void> {
        let serial = this.serial;
        let previous = this.live;
        this.building = true;
        this.status(previous ? 'Update queued' : 'Build queued');
        try {
            const candidateEvents: RuntimeEvent[] = [];
            const candidateFrames: RuntimeFrameReceipt[] = [];
            let endpoint: RuntimeMcpEndpoint | undefined;
            let live = false;
            const candidate = await inCargoBuildSlot(this.scope.targetDirectory, this.lifetime.signal, async () => {
                serial = this.serial;
                previous = this.live;
                this.rebuild = false;
                this.output.appendLine(`Building ${this.target.label}${previous ? ` · keeping PID ${previous.pid} running` : ''}`);
                this.status(previous ? 'Building update' : 'Building application');
                const build = buildCargo(this.target, this.output);
                this.compiler = build.process;
                const artifact = await build.artifact;
                this.compiler = undefined;
                if (this.superseded(serial)) return;
                this.built(build.metadata());
                const launch = await launchArtifact(artifact, this.target, this.output, event => {
                    if (this.disposed) return;
                    if (!live) candidateEvents.push(event);
                    else if (this.live === launch.process) this.accept(event);
                }, value => {
                    if (this.disposed) return;
                    if (!live) endpoint = value;
                    else if (this.live === launch.process) this.mcpChanged(value, this.liveGeneration);
                }, receipt => {
                    if (this.disposed) return;
                    if (!live) {
                        if (candidateFrames.length === 256) candidateFrames.shift();
                        candidateFrames.push(receipt);
                    } else if (this.live === launch.process && this.liveGeneration !== undefined) {
                        this.history.observeFrame(this.liveGeneration, receipt);
                    }
                });
                this.candidate = launch.process;
                return { ...launch, manifests: build.localManifests() };
            });
            if (!candidate || this.superseded(serial)) return;
            await awaitStartup(candidate.process, candidate.ready);
            if (this.superseded(serial)) return;
            this.live = candidate.process;
            this.scope.built(candidate.manifests);
            const generation = this.history.startGeneration(this.identity.session);
            this.liveGeneration = generation;
            this.candidate = undefined;
            const ready = candidateEvents.find(event => event.status === 'ready');
            for (const receipt of candidateFrames) this.history.observeFrame(generation, receipt);
            for (const change of this.changes.values()) {
                this.history.finish(change.record, 'rebuilt', this.pid, undefined, {
                    generation, frame: ready?.frame, acknowledgedAt: ready?.acknowledgedAt,
                });
            }
            this.changes.clear();
            this.results.clear();
            this.acceptedXaml.clear();
            this.languages.clear();
            live = true;
            this.mcpChanged(endpoint, generation);
            for (const event of candidateEvents) this.accept(event);
            if (this.live.pid) this.activated(this.live.pid);
            const retired = previous?.stop();
            this.output.appendLine(`Running ${this.target.label} · PID ${this.live.pid}${previous ? ' · rebuilt and restarted' : ''}`);
            this.status(previous ? 'Rebuilt and restarted' : 'Watching saves');
            const ended = (message: string, failed: boolean) => {
                if (this.disposed || this.live !== candidate.process) return;
                this.live = undefined;
                this.liveGeneration = undefined;
                this.history.endGeneration(generation);
                this.mcpChanged();
                this.languages.clear();
                this.output.appendLine(message);
                this.status('Application stopped', failed);
            };
            void this.live.closed.then(code => ended(`Application exited${code === null ? '' : ` (${code})`}.`, Boolean(code)),
                error => ended(String(error), true));
            await retired;
        } catch (error) {
            if (this.disposed || this.serial !== serial) return;
            for (const change of this.changes.values()) this.history.finish(change.record, 'buildFailed', this.pid, String(error));
            this.changes.clear();
            this.output.appendLine(`${String(error)}${this.live ? ' The previous application is still running.' : ''}`);
            this.status(this.live ? 'Update failed · app still running' : 'Launch failed', true);
        } finally {
            this.compiler = undefined;
            const stopped = this.candidate?.stop();
            this.candidate = undefined;
            await stopped;
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

    async dispose(): Promise<void> {
        this.disposed = true;
        this.lifetime.abort();
        if (this.liveGeneration !== undefined) this.history.endGeneration(this.liveGeneration);
        for (const change of this.changes.values()) this.history.finish(change.record, 'stopped', this.pid);
        this.mcpChanged();
        clearTimeout(this.timer);
        await Promise.all([this.compiler?.stop(), this.candidate?.stop(), this.live?.stop(), this.replacement]);
    }
}

async function awaitStartup(child: ReloadProcess, acknowledged: Promise<void>): Promise<void> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const exited = child.closed.then(code => { throw new Error(`Candidate exited before it opened a window (${code ?? 'terminated'}).`); });
    try {
        const opened = acknowledged.then(() => new Promise<void>(resolve => setTimeout(resolve, 100)));
        await Promise.race([opened, exited, new Promise<never>((_, reject) => {
            timer = setTimeout(() => reject(new Error('No FerroUI runtime acknowledgement. Enable hot reload in the selected Cargo binary.')), 15_000);
        })]);
        // The process may exit while its executable copy is still being released.
        if (child.hasExited) throw new Error('Candidate exited during startup.');
    } finally {
        clearTimeout(timer);
    }
}
