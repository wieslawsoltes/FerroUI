import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import type { Readable, Writable } from 'node:stream';
import { setTimeout as delay } from 'node:timers/promises';
import type { BrowserCdp } from './browserCdp';
import { browserPipe, browserSocket } from './browserCdpTransport';
import { browserExecutable } from './browserExecutable';
import { loopbackPage, type BrowserApp, type ServedBrowserApp } from './browserTargets';
import { OwnedProcess } from './ownedProcess';

export class BrowserPage {
    private readonly lifetime = new AbortController();
    readonly signal = this.lifetime.signal;
    readonly closed: Promise<void>;
    private closing?: Promise<void>;
    private readonly removeListener: () => void;
    private readonly disconnected = () => { void this.close(); };

    constructor(readonly cdp: BrowserCdp, readonly sessionId: string | undefined, private readonly disconnect: () => Promise<void>,
        readonly debugPort?: number) {
        this.closed = new Promise(resolve => this.signal.addEventListener('abort', () => resolve(), { once: true }));
        this.removeListener = cdp.onEvent(event => {
            const detached = this.sessionId !== undefined && event.method === 'Target.detachedFromTarget'
                && (event.params as { sessionId?: string })?.sessionId === this.sessionId;
            const ended = event.sessionId === this.sessionId && ['Inspector.detached', 'Inspector.targetCrashed'].includes(event.method);
            if (detached || ended) this.disconnected();
        });
        cdp.signal.addEventListener('abort', this.disconnected, { once: true });
        if (cdp.signal.aborted) this.disconnected();
    }

    close(): Promise<void> {
        if (!this.closing) {
            this.closing = Promise.resolve().then(this.disconnect);
            this.cdp.signal.removeEventListener('abort', this.disconnected);
            this.removeListener();
            this.lifetime.abort(new Error('The Chromium app document has closed.'));
        }
        return this.closing;
    }
}

export class ManagedBrowser {
    private constructor(readonly cdp: BrowserCdp, private readonly process: OwnedProcess, readonly debugPort?: number) {}

    get pid(): number | undefined { return this.process.pid; }

    static async launch(signal: AbortSignal, debug = false): Promise<ManagedBrowser> {
        const executable = await browserExecutable();
        signal.throwIfAborted();
        const profile = await mkdtemp(path.join(tmpdir(), 'ferroui-chromium-'));
        const cleanup = () => rm(profile, { recursive: true, force: true, maxRetries: 5 });
        try { signal.throwIfAborted(); } catch (error) { await cleanup(); throw error; }
        const transport = debug ? ['--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0'] : ['--remote-debugging-pipe'];
        const child = spawn(executable, ['--headless', ...transport, `--user-data-dir=${profile}`,
            '--no-first-run', '--no-default-browser-check', '--disable-background-networking', 'about:blank'],
        { detached: true, windowsHide: true, stdio: debug ? 'ignore' : ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });
        const process = new OwnedProcess(child, cleanup);
        if (debug) return this.connectDebugger(process, profile, signal);
        const cdp = browserPipe(child.stdio[3] as Writable, child.stdio[4] as Readable);
        void process.closed.then(() => cdp.close(), () => cdp.close());
        return new ManagedBrowser(cdp, process);
    }

    private static async connectDebugger(process: OwnedProcess, profile: string, signal: AbortSignal): Promise<ManagedBrowser> {
        const closed = new AbortController();
        void process.closed.then(() => closed.abort(new Error('Chromium stopped before its debugger was ready.')), error => closed.abort(error));
        try {
            const active = AbortSignal.any([signal, closed.signal, AbortSignal.timeout(12_000)]);
            const { port, route } = await browserDebugEndpoint(profile, active);
            const cdp = await browserSocket(`ws://127.0.0.1:${port}${route}`, active);
            void process.closed.then(() => cdp.close(), () => cdp.close());
            return new ManagedBrowser(cdp, process, port);
        } catch (error) {
            await process.stop().catch(() => {});
            throw error;
        }
    }

    async openPage(url: string, viewport: Pick<BrowserApp, 'width' | 'height'>, signal: AbortSignal, ownsBrowser = false): Promise<BrowserPage> {
        const cdp = this.cdp;
        signal.throwIfAborted();
        // Each development page has its own window so Chromium keeps rendering the
        // accepted page while a candidate starts. Receive the ID even on superseding saves.
        const { targetId } = await cdp.request<{ targetId: string }>('Target.createTarget', {
            url: 'about:blank', newWindow: !ownsBrowser,
        }, cdp.signal);
        const disconnect = () => ownsBrowser ? this.close() : this.closeTarget(targetId);
        let page: BrowserPage | undefined;
        try {
            const { sessionId } = await cdp.request<{ sessionId: string }>('Target.attachToTarget', { targetId, flatten: true }, signal);
            page = new BrowserPage(cdp, sessionId, disconnect, this.debugPort);
            const active = AbortSignal.any([signal, page.signal]);
            await cdp.request('Page.enable', {}, active, sessionId);
            await cdp.request('Emulation.setDeviceMetricsOverride', {
                width: viewport.width, height: viewport.height, deviceScaleFactor: 1, mobile: false,
            }, active, sessionId);
            // Chromium can hide a headless window during candidate replacement.
            // Keep both development pages producing frames during a checked handoff.
            if (!ownsBrowser) await cdp.request('Emulation.setFocusEmulationEnabled', { enabled: true }, active, sessionId);
            const result = await cdp.request<{ errorText?: string }>('Page.navigate', { url }, active, sessionId);
            if (result.errorText) throw new Error('Chromium could not open the configured app URL.');
            return page;
        } catch (error) {
            await (page?.close() ?? disconnect());
            signal.throwIfAborted();
            throw error;
        }
    }

    close(): Promise<void> {
        this.cdp.close();
        return this.process.stop();
    }

    private async closeTarget(targetId: string): Promise<void> {
        if (this.cdp.signal.aborted) return;
        await this.cdp.request('Target.closeTarget', { targetId }, AbortSignal.timeout(3_000)).catch(() => {});
    }
}

async function browserDebugEndpoint(profile: string, signal: AbortSignal): Promise<{ port: number; route: string }> {
    const file = path.join(profile, 'DevToolsActivePort');
    for (;;) {
        signal.throwIfAborted();
        const contents = await readFile(file, 'utf8').catch((error: NodeJS.ErrnoException) => {
            if (error.code === 'ENOENT') return '';
            throw error;
        });
        const [port, route] = contents.trim().split(/\r?\n/);
        if (/^\d{1,5}$/.test(port) && Number(port) > 0 && Number(port) < 65536
            && /^\/devtools\/browser\/[A-Za-z0-9-]+$/.test(route ?? '')) return { port: Number(port), route };
        await delay(50, undefined, { signal });
    }
}

export async function launchBrowser(app: ServedBrowserApp, signal: AbortSignal, debug = false): Promise<BrowserPage> {
    const browser = await ManagedBrowser.launch(signal, debug);
    try {
        return await browser.openPage(app.url, app, signal, true);
    } catch (error) {
        await browser.close();
        throw error;
    }
}

export async function attachBrowser(app: ServedBrowserApp, signal: AbortSignal): Promise<BrowserPage> {
    if (!app.debugPort) throw new Error('Set a local Chromium debug port in the app profile before attaching.');
    const targets = await browserPages(app.debugPort, signal);
    const pages = targets.filter(page => page.type === 'page' && loopbackPage(page.url)?.href === app.url);
    if (pages.length !== 1) throw new Error('Open one Chromium tab at the profile’s exact app URL.');
    const id = pages[0].id;
    if (typeof id !== 'string' || !/^[A-Za-z0-9_-]{1,128}$/.test(id)) throw new Error('Chromium returned an invalid page identity.');
    const cdp = await browserSocket(`ws://127.0.0.1:${app.debugPort}/devtools/page/${id}`, signal);
    try {
        await cdp.request('Page.enable', {}, signal);
        return new BrowserPage(cdp, undefined, async () => { cdp.close(); });
    } catch (error) { cdp.close(); throw error; }
}

async function browserPages(port: number, signal: AbortSignal): Promise<{ id?: unknown; type?: unknown; url?: unknown }[]> {
    try {
        const response = await fetch(`http://127.0.0.1:${port}/json/list`, {
            redirect: 'error', signal: AbortSignal.any([signal, AbortSignal.timeout(4_000)]),
        });
        if (!response.ok || !response.body) throw new Error('Chromium did not list browser pages');
        const reader = response.body.getReader();
        let size = 0;
        const parts: Uint8Array[] = [];
        try {
            for (;;) {
                const chunk = await reader.read();
                if (chunk.done) break;
                size += chunk.value.byteLength;
                if (size > 1024 * 1024) throw new Error('The Chromium page list is too large');
                parts.push(chunk.value);
            }
        } finally { await reader.cancel().catch(() => {}); }
        const pages = JSON.parse(Buffer.concat(parts, size).toString('utf8'));
        if (Array.isArray(pages)) return pages.filter(page => page && typeof page === 'object');
    } catch { signal.throwIfAborted(); }
    throw new Error('Start an isolated Chrome or Chromium profile with a local debug port on the workspace host.');
}
