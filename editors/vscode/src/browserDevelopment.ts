import { readFile, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { Uri } from 'vscode';
import { buildBrowserSite, resolveBrowserBuild, type BrowserBuildTarget, type BuiltBrowserSite } from './browserBuild';
import type { BrowserDebugger } from './browserDebug';
import { BrowserInspection } from './browserInspection';
import { DebuggerPause } from './debuggerPause';
import { ManagedBrowser, type BrowserPage } from './browserProcess';
import { BrowserSites, type PublishedBrowserSite } from './browserSites';
import type { DevelopmentBrowserApp } from './browserTargets';
import { errorMessage } from './protocol';
import type { ReloadHistory, ReloadRecord } from './reloadHistory';
import type { ReloadLog } from './reloadLog';
import { revision, type ProjectMetadata } from './reloadProcess';
import { ReloadScope } from './reloadScope';
import type { RuntimeConnections } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime } from './runtimeTargets';
import { fileKey, isInside } from './workspacePaths';

interface ActivePage {
    page: BrowserPage;
    dispose(): Promise<void>;
    generation: number;
}

const saveDelay = 250;
const sourceFile = /(?:\.(?:rs|xaml)$|(?:^|[\\/])(?:Cargo\.(?:toml|lock)|\.cargo[\\/]config(?:\.toml)?)$)/i;

/** Owns the compiler, site snapshots and Chromium profile for one configured development app. */
export class BrowserDevelopment {
    readonly runtime: AppRuntime;
    private readonly discoveryManifest: string;
    private readonly lifetime = new AbortController();
    private readonly changes = new Map<string, ReloadRecord>();
    private readonly stamps = new Map<string, string>();
    private readonly reads = new Map<string, Promise<void>>();
    private requested = 0;
    private started = 0;
    private timer?: ReturnType<typeof setTimeout>;
    private running?: Promise<void>;
    private closing?: Promise<void>;
    private validation?: AbortController;
    private target?: BrowserBuildTarget;
    private scope?: ReloadScope;
    private sites?: BrowserSites;
    private browser?: ManagedBrowser;
    private live?: ActivePage;

    constructor(readonly app: DevelopmentBrowserApp, private readonly connections: RuntimeConnections, private readonly history: ReloadHistory,
        readonly output: ReloadLog, private readonly publishMetadata: (metadata: ProjectMetadata) => void,
        readonly appDebugger?: BrowserDebugger) {
        this.discoveryManifest = fileKey(path.resolve(Uri.parse(app.folder, true).fsPath, app.build.manifestPath));
        this.runtime = connections.beginBrowser({
            folder: app.folder, profile: app.id, launched: true, development: true,
            ...(appDebugger ? { debuggable: true } : {}),
        }, app.label, appDebugger && (generation => appDebugger.statusFor(generation))).runtime;
    }

    rebuild(delay = 0): void {
        if (this.lifetime.signal.aborted) return;
        this.requested++;
        this.validation?.abort();
        if (!this.running) this.status('Browser build queued');
        this.schedule(delay);
    }

    observe(uri: Uri): void {
        if (uri.scheme !== 'file' || this.lifetime.signal.aborted) return;
        const file = fileKey(uri.fsPath);
        const target = this.target;
        const source = sourceFile.test(file);
        const insidePage = target && isInside(target.hostPage, file);
        const workspaceInput = file === this.discoveryManifest || target?.workspaceInputs.includes(file);
        if (!(insidePage || workspaceInput || source && this.scope?.observes(file))) return;
        const observed = Date.now();
        const pending: Promise<void> = this.observeRevision(file, Boolean(source || workspaceInput), observed, () => this.reads.get(file) === pending);
        this.reads.set(file, pending);
        void pending.finally(() => { if (this.reads.get(file) === pending) this.reads.delete(file); });
    }

    private async observeRevision(file: string, source: boolean, observed: number, current: () => boolean): Promise<void> {
        let stamp: string;
        try {
            const info = await stat(file);
            if (!info.isFile()) return;
            stamp = source ? revision(await readFile(file)) : `${info.mtimeMs}:${info.size}`;
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== 'ENOENT') return;
            stamp = 'removed';
        }
        if (this.lifetime.signal.aborted || !current() || this.stamps.get(file) === stamp) return;
        this.stamps.set(file, stamp);
        const previous = this.changes.get(file);
        if (previous) this.history.finish(previous, 'superseded');
        this.changes.set(file, this.history.observe(file, stamp, observed, { session: this.runtime, target: this.app.label }));
        this.rebuild(saveDelay);
    }

    private schedule(delay = saveDelay): void {
        clearTimeout(this.timer);
        if (this.running || this.lifetime.signal.aborted) return;
        this.timer = setTimeout(() => {
            this.started = this.requested;
            this.running = this.replace().finally(() => {
                this.running = undefined;
                if (this.started !== this.requested) this.schedule();
            });
        }, delay);
    }

    private async replace(): Promise<void> {
        const signal = this.lifetime.signal;
        let artifact: BuiltBrowserSite | undefined;
        let published: PublishedBrowserSite | undefined;
        let page: BrowserPage | undefined;
        let activated = false;
        try {
            this.target = await resolveBrowserBuild(this.app, signal);
            this.scope = new ReloadScope(this.target.cargo);
            artifact = await buildBrowserSite(this.target, this.app, this.output, signal, () => {
                this.started = this.requested;
                this.status(this.live ? 'Building browser update' : 'Building browser app');
                this.output.appendLine(`Building ${this.app.label}${this.live ? ' · keeping the running page' : ''}`);
            });
            if (this.obsolete()) return;
            this.scope?.built(artifact.manifests);
            this.validation = new AbortController();
            const active = AbortSignal.any([signal, this.validation.signal, AbortSignal.timeout(30_000)]);
            const browser = await this.prepareBrowser(active);
            published = await this.sites!.publish(artifact.site, this.app.build.threads);
            page = await browser.openPage(published.url, this.app, active);
            this.status('Starting browser candidate');
            const pause = this.appDebugger && new DebuggerPause();
            const channel = await BrowserInspection.open(page, published.url, active, message => this.output.appendLine(message), pause);
            await verifyRuntime({ channel }, active);
            const presented = Date.now();
            while (this.reads.size) await Promise.all(this.reads.values());
            if (this.obsolete()) return;
            page.signal.throwIfAborted();
            active.throwIfAborted();
            const previous = this.live;
            const generation = this.history.startGeneration(this.runtime);
            const live = { page, dispose: this.discard(page, published, artifact), generation };
            this.live = live;
            activated = true;
            this.connections.updateBrowser(this.runtime, previous ? 'Browser reloaded' : 'Watching browser saves',
                { url: published.url, pid: browser.pid, endpoint: { channel } });
            const acceptedGeneration = this.connections.revision(this.runtime);
            if (browser.debugPort && pause && acceptedGeneration !== undefined) {
                this.appDebugger?.accepted({ generation: acceptedGeneration, port: browser.debugPort,
                    url: published.url, hostPage: this.target.hostPage, pause,
                    onPause: paused => this.connections.setBrowserPaused(this.runtime, acceptedGeneration, paused),
                    onAttachmentChanged: () => this.connections.notifyDebuggerChanged(this.runtime),
                });
            }
            this.history.observeFrame(generation, { frame: 1, batch: 1, status: 'rendered', unixMs: presented, afterAcknowledgementMs: 0 });
            for (const record of this.changes.values()) {
                this.history.finish(record, 'rebuilt', browser.pid, undefined, { generation, frame: 1, acknowledgedAt: presented });
            }
            this.changes.clear();
            this.output.appendLine(`${this.app.label} · candidate frame accepted`);
            void page.closed.then(() => this.pageEnded(live));
            try { this.publishMetadata(artifact.metadata); }
            catch (error) { this.output.appendLine(errorMessage(error)); }
            await previous?.dispose();
        } catch (error) {
            if (!this.obsolete()) {
                const message = errorMessage(error);
                this.output.appendLine(message);
                this.status(this.live ? 'Browser update failed' : 'Browser launch failed');
                for (const record of this.changes.values()) this.history.finish(record, 'buildFailed', this.browser?.pid, message);
                this.changes.clear();
            }
        } finally {
            this.validation = undefined;
            if (!activated) await this.discard(page, published, artifact)();
        }
    }

    private async prepareBrowser(signal: AbortSignal): Promise<ManagedBrowser> {
        if (!this.sites) {
            const sites = new BrowserSites();
            await sites.listen();
            this.sites = sites;
        }
        if (this.browser?.cdp.signal.aborted) {
            await this.browser.close();
            this.browser = undefined;
        }
        return this.browser ??= await ManagedBrowser.launch(signal, Boolean(this.appDebugger));
    }

    private obsolete(): boolean { return this.lifetime.signal.aborted || this.started !== this.requested; }

    private status(message: string): void { this.connections.updateBrowser(this.runtime, message); }

    private discard(page?: BrowserPage, published?: PublishedBrowserSite, artifact?: BuiltBrowserSite): () => Promise<void> {
        return async () => {
            try { await page?.close().catch(error => this.output.appendLine(errorMessage(error))); }
            finally {
                published?.unpublish();
                await artifact?.dispose().catch(error => this.output.appendLine(errorMessage(error)));
            }
        };
    }

    private async pageEnded(active: ActivePage): Promise<void> {
        if (this.live !== active || this.lifetime.signal.aborted) return;
        this.live = undefined;
        this.appDebugger?.retire();
        this.connections.updateBrowser(this.runtime, 'Browser page closed', null);
        this.history.endGeneration(active.generation);
        await active.dispose();
    }

    close(): Promise<void> {
        if (this.closing) return this.closing;
        clearTimeout(this.timer);
        this.lifetime.abort();
        this.connections.removePlatform(this.runtime);
        this.closing = this.finish().finally(() => this.output.dispose());
        return this.closing;
    }

    private async finish(): Promise<void> {
        await this.running;
        await this.appDebugger?.close();
        for (const record of this.changes.values()) this.history.finish(record, 'stopped');
        if (this.live) {
            this.history.endGeneration(this.live.generation);
            await this.live.dispose();
        }
        await this.browser?.close();
        await this.sites?.close();
    }
}
