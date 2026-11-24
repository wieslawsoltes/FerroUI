import * as path from 'node:path';
import { setTimeout as delayFor } from 'node:timers/promises';
import type { AndroidBridge } from './androidBridge';
import { buildAndroidApp, type AndroidArtifact } from './androidBuild';
import { openAndroidProcess, type AndroidProcess } from './androidProcess';
import type { AndroidApp } from './androidTargets';
import { errorMessage } from './protocol';
import type { ReloadHistory, ReloadRecord } from './reloadHistory';
import type { ProcessOutput } from './reloadLog';
import { revision } from './reloadProcess';
import { ReloadScope } from './reloadScope';
import type { RuntimeConnections } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';
import { fileKey } from './workspacePaths';

/** Owns the staged APK and the last accepted installation for one device and package. */
export class AndroidDevelopment {
    current?: AndroidProcess;
    deploying = false;
    private artifact?: AndroidArtifact;
    private operation?: Promise<void>;
    private timer?: ReturnType<typeof setTimeout>;
    private requested = 0;
    private scope?: ReloadScope;
    private workspaceInputs = new Set<string>();
    private readonly stamps = new Map<string, string>();
    private readonly changes = new Map<string, { version: number; record: ReloadRecord }>();
    private generation?: number;
    private failure?: string;

    constructor(private readonly app: AndroidApp, private readonly bridge: AndroidBridge, private readonly connections: RuntimeConnections,
        readonly runtime: AppRuntime, private readonly history: ReloadHistory, private readonly output: ProcessOutput, private readonly signal: AbortSignal,
        private readonly beforeDeploy: () => Promise<void>) {}

    get symbols(): string[] {
        const built = this.artifact?.symbolFiles ?? [];
        const names = new Set(built.map(file => path.basename(file)));
        return [...built, ...this.app.symbolFiles.filter(file => !names.has(path.basename(file)))];
    }

    rebuild(delay = 0): void {
        this.signal.throwIfAborted();
        this.requested++;
        this.schedule(delay);
    }

    observe(file: string, text: () => string): void {
        if (this.signal.aborted || !this.app.build?.reloadOnSave) return;
        const key = fileKey(file);
        if (!this.workspaceInputs.has(key) && (!/\.(?:rs|xaml)$|(?:^|[\\/])Cargo\.toml$/i.test(file) || !this.scope?.observes(key))) return;
        const stamp = revision(Buffer.from(text(), 'utf8'));
        if (this.stamps.get(key) === stamp) return;
        this.stamps.set(key, stamp);
        const prior = this.changes.get(key);
        if (prior) this.history.finish(prior.record, 'superseded');
        this.changes.set(key, { version: ++this.requested,
            record: this.history.observe(key, stamp, Date.now(), { session: this.runtime, target: this.app.label }) });
        this.schedule(250);
    }

    async ready(): Promise<void> {
        await this.operation;
        this.signal.throwIfAborted();
        if (!this.current) throw new Error(this.failure ?? 'The Android app is waiting for its first frame.');
    }

    private schedule(delay: number): void {
        clearTimeout(this.timer);
        if (this.operation || this.signal.aborted) return;
        this.status('Android build queued');
        if (delay) this.timer = setTimeout(() => this.start(), delay);
        else this.start();
    }

    private start(): void {
        if (this.signal.aborted) return;
        this.operation = this.drain().finally(() => { this.operation = undefined; });
    }

    private async drain(): Promise<void> {
        while (!this.signal.aborted) {
            const version = this.requested;
            await this.replace(version);
            if (version === this.requested) return;
            this.status('Android build queued');
            await delayFor(250, undefined, { signal: this.signal }).catch(() => {});
        }
    }

    private async replace(version: number): Promise<void> {
        let candidate: AndroidArtifact | undefined;
        let installed: AndroidProcess | undefined;
        let attemptedDeploy = false;
        this.failure = undefined;
        try {
            candidate = await buildAndroidApp(this.app, this.bridge, this.output, this.signal, cargo => {
                this.scope = new ReloadScope(cargo);
                const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
                this.workspaceInputs = new Set(['Cargo.lock', '.cargo/config', '.cargo/config.toml'].map(file => fileKey(path.join(workspace, file))));
                this.status(this.current ? 'Building Android update' : 'Building Android app');
                this.output.appendLine(`Building ${this.app.label}`);
            });
            if (version !== this.requested) return;
            const previous = this.current;
            if (previous && !await this.bridge.matches(this.app.packageId, previous.receipt, this.signal)) {
                await this.exited(previous);
                throw new Error('The Android app process changed. Rebuild to start a fresh session process.');
            }
            if (!previous && this.artifact && await this.bridge.pid(this.app.packageId, this.signal)) {
                throw new Error('That Android package now has an independently running process. Start a new app session to deploy it.');
            }
            if (version !== this.requested) return;
            this.deploying = true;
            await this.beforeDeploy();
            this.signal.throwIfAborted();
            this.status('Deploying Android app');
            attemptedDeploy = true;
            installed = await this.launch(candidate);
            this.signal.throwIfAborted();
            const preceding = this.artifact;
            this.artifact = candidate;
            this.accept(installed, preceding ? 'Android app redeployed' : 'Android app running');
            const presented = Date.now();
            const generation = this.generation!;
            this.history.observeFrame(generation, { frame: 1, batch: 1, status: 'rendered', unixMs: presented, afterAcknowledgementMs: 0 });
            this.finish(version, record => this.history.finish(record, 'rebuilt', installed!.receipt.pid, undefined,
                { generation, frame: 1, acknowledgedAt: presented }));
            installed = undefined;
            await previous?.close(false).catch(problem => this.output.appendLine(errorMessage(problem)));
            await preceding?.dispose().catch(problem => this.output.appendLine(errorMessage(problem)));
        } catch (error) {
            if (!this.signal.aborted) {
                this.failure = errorMessage(error);
                this.output.appendLine(this.failure);
                this.finish(version, record => this.history.finish(record, 'buildFailed', this.current?.receipt.pid, this.failure));
                if (attemptedDeploy) await this.recover();
                else this.status('Android build failed');
            }
        } finally {
            this.deploying = false;
            await installed?.close(true).catch(problem => this.output.appendLine(errorMessage(problem)));
            if (this.artifact !== candidate) await candidate?.dispose().catch(problem => this.output.appendLine(errorMessage(problem)));
        }
    }

    private async recover(): Promise<void> {
        const previous = this.current;
        try {
            if (previous && await this.bridge.matches(this.app.packageId, previous.receipt, this.signal)) {
                this.status('Android deployment failed');
                return;
            }
            // Only restore a package whose prior app process belonged to this session.
            if (previous) await this.exited(previous);
            if (previous && this.artifact && !await this.bridge.pid(this.app.packageId, this.signal)) {
                this.status('Restoring previous Android build');
                const restored = await this.launch(this.artifact);
                if (this.signal.aborted) { await restored.close(true); return; }
                this.accept(restored, 'Android deployment failed · restored previous build');
                return;
            }
        } catch (error) { this.output.appendLine(errorMessage(error)); }
        if (!this.signal.aborted) this.status('Android deployment failed');
    }

    async exited(process: AndroidProcess): Promise<void> {
        if (this.current !== process) return;
        this.current = undefined;
        this.retireFrame();
        this.connections.updateAndroid(this.runtime, 'Android app exited', null);
        await process.close(false);
    }

    async close(terminate: boolean): Promise<void> {
        clearTimeout(this.timer);
        await this.operation;
        this.retireFrame();
        this.finish(Infinity, record => this.history.finish(record, 'stopped'));
        try { await this.current?.close(terminate); }
        finally { await this.artifact?.dispose(); }
    }

    private launch(artifact: AndroidArtifact): Promise<AndroidProcess> {
        return openAndroidProcess({ ...this.app, apkPath: artifact.apkPath }, this.bridge, 'run',
            AbortSignal.any([this.signal, AbortSignal.timeout(55_000)]));
    }

    private status(message: string): void { this.connections.updateAndroid(this.runtime, message); }

    private accept(process: AndroidProcess, message: string): void {
        this.current = process;
        this.generation = this.history.startGeneration(this.runtime);
        this.connections.updateAndroid(this.runtime, message, {
            pid: process.receipt.pid, debuggable: Boolean(process.receipt.debuggerPath), endpoint: process.forward.endpoint,
        });
    }

    private retireFrame(): void {
        if (this.generation !== undefined) this.history.endGeneration(this.generation);
        this.generation = undefined;
    }

    private finish(version: number, report: (record: ReloadRecord) => void): void {
        for (const [file, edit] of this.changes) {
            if (edit.version > version) continue;
            report(edit.record);
            this.changes.delete(file);
        }
    }
}
