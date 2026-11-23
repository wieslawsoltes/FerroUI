import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import type { Readable, Writable } from 'node:stream';
import type { BrowserCdp } from './browserCdp';
import { browserPipe, browserSocket } from './browserCdpTransport';
import { browserExecutable } from './browserExecutable';
import { loopbackPage, type BrowserApp } from './browserTargets';
import { OwnedProcess } from './ownedProcess';

export interface BrowserPage {
    cdp: BrowserCdp;
    sessionId?: string;
    close(): Promise<void>;
}

export async function launchBrowser(app: BrowserApp, signal: AbortSignal): Promise<BrowserPage> {
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
    const close = async () => { cdp.close(); await process.stop(); };
    void process.closed.then(() => cdp.close(), () => cdp.close());
    try {
        const { targetId } = await cdp.request<{ targetId: string }>('Target.createTarget', { url: 'about:blank' }, signal);
        const { sessionId } = await cdp.request<{ sessionId: string }>('Target.attachToTarget', { targetId, flatten: true }, signal);
        await cdp.request('Page.enable', {}, signal, sessionId);
        await cdp.request('Emulation.setDeviceMetricsOverride', {
            width: app.width, height: app.height, deviceScaleFactor: 1, mobile: false,
        }, signal, sessionId);
        const result = await cdp.request<{ errorText?: string }>('Page.navigate', { url: app.url }, signal, sessionId);
        if (result.errorText) throw new Error('Chromium could not open the configured app URL.');
        return { cdp, sessionId, close };
    } catch (error) {
        await close();
        signal.throwIfAborted();
        throw error;
    }
}

export async function attachBrowser(app: BrowserApp, signal: AbortSignal): Promise<BrowserPage> {
    if (!app.debugPort) throw new Error('Set a local Chromium debug port in the app profile before attaching.');
    const targets = await browserPages(app.debugPort, signal);
    const pages = targets.filter(page => page.type === 'page' && loopbackPage(page.url)?.href === app.url);
    if (pages.length !== 1) throw new Error('Open one Chromium tab at the profile’s exact app URL.');
    const id = pages[0].id;
    if (typeof id !== 'string' || !/^[A-Za-z0-9_-]{1,128}$/.test(id)) throw new Error('Chromium returned an invalid page identity.');
    const cdp = await browserSocket(`ws://127.0.0.1:${app.debugPort}/devtools/page/${id}`, signal);
    try {
        await cdp.request('Page.enable', {}, signal);
        return { cdp, async close() { cdp.close(); } };
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
