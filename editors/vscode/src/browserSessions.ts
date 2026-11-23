import * as vscode from 'vscode';
import { BrowserDevelopment } from './browserDevelopment';
import { BrowserInspection } from './browserInspection';
import { attachBrowser, launchBrowser, type BrowserPage } from './browserProcess';
import { browserProfiles, type BrowserApp, type BrowserMode, type BrowserRequest, type ServedBrowserApp } from './browserTargets';
import type { ReloadHistory } from './reloadHistory';
import { ReloadLog } from './reloadLog';
import type { ProjectMetadata } from './reloadProcess';
import { reloadWatchPattern } from './reloadScope';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime } from './runtimeTargets';

interface Session {
    app: ServedBrowserApp;
    page: Promise<BrowserPage>;
    stop: AbortController;
    runtime?: AppRuntime;
    closing?: Promise<void>;
}

export class BrowserSessions implements vscode.Disposable {
    private readonly lifetime = new AbortController();
    private readonly sessions = new Map<string, Session>();
    private readonly development = new Map<string, BrowserDevelopment>();
    private readonly subscriptions: vscode.Disposable[];
    private output?: vscode.OutputChannel;
    private closing?: Promise<void>;
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, session] of this.sessions) {
            if (closed.has(session.app.folder)) this.retire(key, session);
        }
        for (const [key, session] of this.development) {
            if (closed.has(session.app.folder)) void this.closeDevelopment(key, session).catch(error => console.warn(error));
        }
    });

    constructor(private readonly connections: RuntimeConnections, private readonly history: ReloadHistory,
        private readonly publish: (metadata: ProjectMetadata) => void) {
        const observe = (uri: vscode.Uri) => { for (const session of this.development.values()) session.observe(uri); };
        const watchers = [reloadWatchPattern, '**/Cargo.lock', '**/.cargo/config{,.toml}', '**/wwwroot/**'];
        this.subscriptions = [this.folders, vscode.workspace.onDidSaveTextDocument(document => observe(document.uri)),
            ...watchers.flatMap(pattern => {
                const watcher = vscode.workspace.createFileSystemWatcher(pattern);
                return [watcher, watcher.onDidChange(observe), watcher.onDidCreate(observe), watcher.onDidDelete(observe)];
            })];
    }

    targets(folder?: string) {
        this.requireWorkspace();
        return { apps: browserProfiles(folder) };
    }

    async open(request: BrowserRequest, mode: BrowserMode, signal: AbortSignal): Promise<RuntimeSummary> {
        this.requireWorkspace();
        signal.throwIfAborted();
        const apps = this.targets(request.folder).apps.filter(app => app.id === request.profile);
        if (apps.length !== 1) throw new Error('Choose an unambiguous browser app profile.');
        const [app] = apps;
        if (app.build && mode === 'attach') throw new Error('Run this managed browser profile from App Sessions.');
        const key = mode === 'attach' ? `page:${app.debugPort}:${app.url}` : `run:${app.folder}:${app.id}`;
        const current = this.sessions.get(key) ?? this.development.get(key);
        if (current) {
            const ready = current.runtime && this.connections.describe(current.runtime);
            if (ready && sameProfile(current.app, app)) return ready;
            throw new Error('Close the current browser session before opening this profile.');
        }
        if (app.build) {
            this.output ??= vscode.window.createOutputChannel('FerroUI Browser Reload');
            const session = new BrowserDevelopment(app, this.connections, this.history, new ReloadLog(this.output), this.publish);
            this.development.set(key, session);
            session.rebuild();
            return this.connections.describe(session.runtime)!;
        }
        return this.openServed(app, key, mode, signal);
    }

    private async openServed(app: ServedBrowserApp, key: string, mode: BrowserMode, signal: AbortSignal): Promise<RuntimeSummary> {
        const stop = new AbortController();
        const timeout = AbortSignal.timeout(30_000);
        const active = AbortSignal.any([signal, this.lifetime.signal, stop.signal, timeout]);
        const page = mode === 'run' ? launchBrowser(app, active) : attachBrowser(app, active);
        const session: Session = { app, page, stop };
        this.sessions.set(key, session);
        try {
            const browser = await page;
            void browser.closed.then(() => this.retire(key, session));
            const channel = await BrowserInspection.open(browser, app.url, active);
            await verifyRuntime({ channel }, active);
            active.throwIfAborted();
            const runtime = this.connections.addPlatform({ kind: 'browser', browser: {
                folder: app.folder, profile: app.id, url: app.url, launched: mode === 'run',
            } }, app.label, { channel });
            session.runtime = runtime.runtime;
            return runtime;
        } catch (error) {
            await this.close(key, session).catch(problem => console.warn(problem));
            if (timeout.aborted && !signal.aborted && !this.lifetime.signal.aborted) throw new Error('Timed out connecting the browser app.');
            throw error;
        }
    }

    async stop(runtime: AppRuntime): Promise<void> {
        const development = this.developmentFor(runtime);
        if (development) return this.closeDevelopment(...development);
        const current = [...this.sessions].find(([, session]) => session.runtime === runtime);
        if (!current) throw new Error('Choose a current browser app session.');
        await this.close(...current);
    }

    rebuild(runtime: AppRuntime): RuntimeSummary {
        const session = this.developmentFor(runtime)?.[1];
        if (!session) throw new Error('Choose a managed browser development session.');
        session.rebuild();
        return this.connections.describe(runtime)!;
    }

    recentOutput(runtime: AppRuntime, lines: number): { runtime: AppRuntime; output: string } {
        const session = this.developmentFor(runtime)?.[1];
        if (!session) throw new Error('Choose a managed browser development session.');
        return { runtime, output: session.recentOutput(lines) };
    }

    showOutput(): void { this.output?.show(true); }

    private developmentFor(runtime: AppRuntime) {
        return [...this.development].find(([, session]) => session.runtime === runtime);
    }

    private async closeDevelopment(key: string, session: BrowserDevelopment): Promise<void> {
        try { await session.close(); }
        finally { if (this.development.get(key) === session) this.development.delete(key); }
    }

    private requireWorkspace(): void {
        this.lifetime.signal.throwIfAborted();
        if (!vscode.workspace.isTrusted || !vscode.workspace.workspaceFolders?.length) {
            throw new Error('Open a trusted workspace before using a browser app profile.');
        }
    }

    private retire(key: string, session: Session): void {
        void this.close(key, session).catch(error => console.warn(error));
    }

    private close(key: string, session: Session): Promise<void> {
        if (session.closing) return session.closing;
        session.stop.abort(new Error('The browser app session has closed.'));
        if (session.runtime) this.connections.removePlatform(session.runtime);
        session.closing = session.page.catch(() => undefined).then(page => page?.close()).finally(() => {
            if (this.sessions.get(key) === session) this.sessions.delete(key);
        });
        return session.closing;
    }

    shutdown(): Promise<void> {
        if (this.closing) return this.closing;
        this.lifetime.abort();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.closing = Promise.allSettled([
            ...[...this.sessions].map(([key, session]) => this.close(key, session)),
            ...[...this.development].map(([key, session]) => this.closeDevelopment(key, session)),
        ]).then(() => this.output?.dispose());
        return this.closing;
    }

    dispose(): void { void this.shutdown(); }
}

function sameProfile(previous: BrowserApp, next: BrowserApp): boolean {
    return previous.folder === next.folder && previous.id === next.id && previous.url === next.url
        && previous.width === next.width && previous.height === next.height && previous.debugPort === next.debugPort
        && JSON.stringify(previous.build) === JSON.stringify(next.build);
}
