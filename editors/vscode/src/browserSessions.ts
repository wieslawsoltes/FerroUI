import * as vscode from 'vscode';
import { BrowserInspection } from './browserInspection';
import { attachBrowser, launchBrowser, type BrowserPage } from './browserProcess';
import { browserProfiles, type BrowserApp, type BrowserMode, type BrowserRequest } from './browserTargets';
import type { RuntimeConnections, RuntimeSummary } from './runtimeConnections';
import { verifyRuntime } from './runtimeReady';
import type { AppRuntime } from './runtimeTargets';

interface Session {
    app: BrowserApp;
    page: Promise<BrowserPage>;
    stop: AbortController;
    runtime?: AppRuntime;
    closing?: Promise<void>;
}

export class BrowserSessions implements vscode.Disposable {
    private readonly lifetime = new AbortController();
    private readonly sessions = new Map<string, Session>();
    private readonly folders = vscode.workspace.onDidChangeWorkspaceFolders(({ removed }) => {
        const closed = new Set(removed.map(folder => folder.uri.toString()));
        for (const [key, session] of this.sessions) {
            if (closed.has(session.app.folder)) this.retire(key, session);
        }
    });

    constructor(private readonly connections: RuntimeConnections) {}

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
        const key = mode === 'attach' ? `page:${app.debugPort}:${app.url}` : `run:${app.folder}:${app.id}`;
        const current = this.sessions.get(key);
        if (current) {
            const ready = current.runtime && this.connections.describe(current.runtime);
            if (ready && sameProfile(current.app, app)) return ready;
            throw new Error('Close the current browser session before opening this profile.');
        }
        const stop = new AbortController();
        const timeout = AbortSignal.timeout(30_000);
        const active = AbortSignal.any([signal, this.lifetime.signal, stop.signal, timeout]);
        const page = mode === 'run' ? launchBrowser(app, active) : attachBrowser(app, active);
        const session: Session = { app, page, stop };
        this.sessions.set(key, session);
        try {
            const browser = await page;
            void browser.cdp.closed.then(() => this.retire(key, session));
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
        const current = [...this.sessions].find(([, session]) => session.runtime === runtime);
        if (!current) throw new Error('Choose a current browser app session.');
        await this.close(...current);
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

    async shutdown(): Promise<void> {
        this.lifetime.abort();
        this.folders.dispose();
        await Promise.allSettled([...this.sessions].map(([key, session]) => this.close(key, session)));
    }

    dispose(): void { void this.shutdown(); }
}

function sameProfile(previous: BrowserApp, next: BrowserApp): boolean {
    return previous.folder === next.folder && previous.id === next.id && previous.url === next.url
        && previous.width === next.width && previous.height === next.height && previous.debugPort === next.debugPort;
}
