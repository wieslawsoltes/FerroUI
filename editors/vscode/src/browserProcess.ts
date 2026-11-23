import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import type { Readable, Writable } from 'node:stream';
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

    constructor(readonly cdp: BrowserCdp, readonly sessionId: string | undefined, private readonly disconnect: () => Promise<void>) {
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
    private constructor(readonly cdp: BrowserCdp, private readonly process: OwnedProcess) {}

    get pid(): number | undefined { return this.process.pid; }

    static async launch(signal: AbortSignal): Promise<ManagedBrowser> {
        const executable = await browserExecutable();
        signal.throwIfAborted();
        const profile = await mkdtemp(path.join(tmpdir(), 'ferroui-chromium-'));
        const cleanup = () => rm(profile, { recursive: true, force: true, maxRetries: 5 });
        try { signal.throwIfAborted(); } catch (error) { await cleanup(); throw error; }
        const child = spawn(executable, ['--headless', '--remote-debugging-pipe', `--user-data-dir=${profile}`,
            '--no-first-run', '--no-default-browser-check', '--disable-background-networking', 'about:blank'],
        { detached: true, windowsHide: true, stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });
        const process = new OwnedProcess(child, cleanup);
        const cdp = browserPipe(child.stdio[3] as Writable, child.stdio[4] as Readable);
        void process.closed.then(() => cdp.close(), () => cdp.close());
        return new ManagedBrowser(cdp, process);
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
            page = new BrowserPage(cdp, sessionId, disconnect);
            const active = AbortSignal.any([signal, page.signal]);
            await cdp.request('Page.enable', {}, active, sessionId);
            await cdp.request('Emulation.setDeviceMetricsOverride', {
                width: viewport.width, height: viewport.height, deviceScaleFactor: 1, mobile: false,
            }, active, sessionId);
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

export async function launchBrowser(app: ServedBrowserApp, signal: AbortSignal): Promise<BrowserPage> {
    const browser = await ManagedBrowser.launch(signal);
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
