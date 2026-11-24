import * as path from 'node:path';
import { setTimeout as delayFor } from 'node:timers/promises';
import type { CargoTarget } from './cargoTarget';
import { errorMessage } from './protocol';
import type { ReloadHistory, ReloadRecord } from './reloadHistory';
import type { ProcessOutput } from './reloadLog';
import { revision } from './reloadProcess';
import { ReloadScope } from './reloadScope';
import type { AppRuntime } from './runtimeTargets';
import { fileKey } from './workspacePaths';

export interface MobileBundle { dispose(): Promise<void> }
export interface MobileProcess { pid: number; close(terminate: boolean): Promise<void> }

interface Deployment<Bundle, Process> {
    platform: string;
    label: string;
    reloadOnSave: boolean;
    build(signal: AbortSignal, starting: (target: CargoTarget) => void): Promise<Bundle>;
    install(bundle: Bundle, signal: AbortSignal): Promise<Process>;
    owns(process: Process, signal: AbortSignal): Promise<boolean>;
    running(signal: AbortSignal): Promise<boolean>;
    beforeInstall(): Promise<void>;
    publish(status: string, process?: Process | null): void;
    resumeDebugger?(process: Process): void;
}

/** Keeps the accepted installation available while staging a source-built replacement. */
export class MobileDevelopment<Bundle extends MobileBundle, Process extends MobileProcess> {
    current?: Process;
    deploying = false;
    protected artifact?: Bundle;
    private operation?: Promise<void>;
    private timer?: ReturnType<typeof setTimeout>;
    private requested = 0;
    private scope?: ReloadScope;
    private workspaceInputs = new Set<string>();
    private readonly stamps = new Map<string, string>();
    private readonly changes = new Map<string, { version: number; record: ReloadRecord }>();
    private generation?: number;
    private failure?: string;

    constructor(private readonly deployment: Deployment<Bundle, Process>, readonly runtime: AppRuntime,
        private readonly history: ReloadHistory, private readonly output: ProcessOutput, private readonly signal: AbortSignal) {}

    private get platform(): string { return this.deployment.platform; }

    rebuild(delay = 0): void {
        this.signal.throwIfAborted();
        this.requested++;
        this.schedule(delay);
    }

    observe(file: string, text: () => string): void {
        if (this.signal.aborted || !this.deployment.reloadOnSave) return;
        const key = fileKey(file);
        if (!this.workspaceInputs.has(key) && (!/\.(?:rs|xaml)$|(?:^|[\\/])Cargo\.toml$/i.test(file) || !this.scope?.observes(key))) return;
        const stamp = revision(Buffer.from(text(), 'utf8'));
        if (this.stamps.get(key) === stamp) return;
        this.stamps.set(key, stamp);
        const prior = this.changes.get(key);
        if (prior) this.history.finish(prior.record, 'superseded');
        this.changes.set(key, { version: ++this.requested,
            record: this.history.observe(key, stamp, Date.now(), { session: this.runtime, target: this.deployment.label }) });
        this.schedule(250);
    }

    async ready(): Promise<void> {
        if (!this.current || this.deploying) await this.operation;
        this.signal.throwIfAborted();
        if (!this.current) throw new Error(this.failure ?? `The ${this.platform} app is waiting for its first frame.`);
    }

    private schedule(delay: number): void {
        clearTimeout(this.timer);
        if (this.operation || this.signal.aborted) return;
        this.status(`${this.platform} build queued`);
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
            this.status(`${this.platform} build queued`);
            await delayFor(250, undefined, { signal: this.signal }).catch(() => {});
        }
    }

    private async replace(version: number): Promise<void> {
        let candidate: Bundle | undefined;
        let installed: Process | undefined;
        let attemptedDeploy = false;
        this.failure = undefined;
        try {
            candidate = await this.deployment.build(this.signal, cargo => {
                this.scope = new ReloadScope(cargo);
                const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
                this.workspaceInputs = new Set(['Cargo.lock', '.cargo/config', '.cargo/config.toml'].map(file => fileKey(path.join(workspace, file))));
                this.status(this.current ? `Building ${this.platform} update` : `Building ${this.platform} app`);
                this.output.appendLine(`Building ${this.deployment.label}`);
            });
            if (version !== this.requested) return;
            const previous = this.current;
            if (previous && !await this.deployment.owns(previous, this.signal)) {
                await this.exited(previous);
                throw new Error(`The ${this.platform} app process changed. Rebuild to start a fresh session process.`);
            }
            if (!previous && this.artifact && await this.deployment.running(this.signal)) {
                throw new Error(`That ${this.platform} app now has an independently running process. Start a new app session to deploy it.`);
            }
            if (version !== this.requested) return;
            this.deploying = true;
            await this.deployment.beforeInstall();
            this.signal.throwIfAborted();
            this.status(`Deploying ${this.platform} app`);
            attemptedDeploy = true;
            installed = await this.launch(candidate);
            this.signal.throwIfAborted();
            const preceding = this.artifact;
            this.artifact = candidate;
            this.accept(installed, preceding ? `${this.platform} app redeployed` : `${this.platform} app running`);
            const presented = Date.now();
            const generation = this.generation!;
            this.history.observeFrame(generation, { frame: 1, batch: 1, status: 'rendered', unixMs: presented, afterAcknowledgementMs: 0 });
            this.finish(version, record => this.history.finish(record, 'rebuilt', installed!.pid, undefined,
                { generation, frame: 1, acknowledgedAt: presented }));
            installed = undefined;
            await previous?.close(false).catch(problem => this.output.appendLine(errorMessage(problem)));
            await preceding?.dispose().catch(problem => this.output.appendLine(errorMessage(problem)));
        } catch (error) {
            if (!this.signal.aborted) {
                this.failure = errorMessage(error);
                this.output.appendLine(this.failure);
                this.finish(version, record => this.history.finish(record, 'buildFailed', this.current?.pid, this.failure));
                if (attemptedDeploy) await this.recover();
                else this.status(`${this.platform} build failed`);
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
            if (previous && await this.deployment.owns(previous, this.signal)) {
                this.deploying = false;
                this.status(`${this.platform} deployment failed`);
                this.deployment.resumeDebugger?.(previous);
                return;
            }
            // Only restore a package whose prior app process belonged to this session.
            if (previous) await this.exited(previous);
            if (previous && this.artifact && !await this.deployment.running(this.signal)) {
                this.status(`Restoring previous ${this.platform} build`);
                const restored = await this.launch(this.artifact);
                if (this.signal.aborted) { await restored.close(true); return; }
                this.accept(restored, `${this.platform} deployment failed · restored previous build`);
                return;
            }
        } catch (error) { this.output.appendLine(errorMessage(error)); }
        if (!this.signal.aborted) this.status(`${this.platform} deployment failed`);
    }

    async exited(process: Process): Promise<void> {
        if (this.current !== process) return;
        this.current = undefined;
        this.retireFrame();
        this.deployment.publish(`${this.platform} app exited`, null);
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

    private launch(artifact: Bundle): Promise<Process> {
        return this.deployment.install(artifact, AbortSignal.any([this.signal, AbortSignal.timeout(55_000)]));
    }

    private status(message: string): void { this.deployment.publish(message); }

    private accept(process: Process, message: string): void {
        this.deploying = false;
        this.current = process;
        this.generation = this.history.startGeneration(this.runtime);
        this.deployment.publish(message, process);
        this.deployment.resumeDebugger?.(process);
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
