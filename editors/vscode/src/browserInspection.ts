import { setTimeout as delay } from 'node:timers/promises';
import type { BrowserEvent } from './browserCdp';
import type { BrowserPage } from './browserProcess';
import type { Bounds } from './protocol';
import type { RuntimeInspectionChannel, RuntimeToolDefinition, RuntimeToolResult } from './runtimeClient';
import type { RuntimeNode } from './runtimeProtocol';

interface FrameTree { frameTree: { frame: { url: string; urlFragment?: string } } }
interface Evaluation { result?: { objectId?: string; value?: unknown }; exceptionDetails?: object }
interface InspectionReply { ok: boolean; value?: unknown; error?: string }

const bridge = 'Object.getOwnPropertyDescriptor(globalThis, "__ferrouiLiveApp")?.value ?? null';
const presentedBridge = `(() => {
    const app = (${bridge});
    if (app?.version !== 1) return 'inspector';
    if (app.ready()) return app;
    return document.hidden ? 'hidden' : 'frame';
})()`;
const connected = `this === (${bridge}) && this.version === 1`;
const dispatch = `function (request) { return (${connected}) ? this.dispatch(request) : null; }`;
const frameBounds = `function () { return (${connected}) ? this.bounds() : null; }`;
const startupErrors = {
    navigation: 'Timed out loading the configured Chromium app URL.',
    inspector: 'Timed out waiting for the Live App inspector. Serve a debug build with the inspector enabled.',
    frame: 'Timed out waiting for the Live App inspector’s first frame.',
    hidden: 'Timed out waiting for the browser inspector’s first frame while its tab was hidden.',
};

/** One channel per Chromium document. Reopening a tab creates a new runtime generation. */
export class BrowserInspection implements RuntimeInspectionChannel {
    private objectId?: string;
    private root?: number;
    private navigation = 0;

    private constructor(private readonly page: BrowserPage) {
        const unsubscribe = this.page.cdp.onEvent(event => this.onEvent(event));
        this.page.signal.addEventListener('abort', unsubscribe, { once: true });
    }

    static async open(page: BrowserPage, url: string, signal: AbortSignal, report?: (message: string) => void): Promise<BrowserInspection> {
        const inspection = new BrowserInspection(page);
        const deadline = AbortSignal.any([signal, page.signal, AbortSignal.timeout(15_000)]);
        let pending: keyof typeof startupErrors = 'navigation';
        const unsubscribe = report && page.cdp.onEvent(event => {
            if (event.sessionId !== page.sessionId || event.method !== 'Runtime.exceptionThrown') return;
            const details = (event.params as { exceptionDetails?: { exception?: { description?: string }; text?: string } })?.exceptionDetails;
            const message = details?.exception?.description ?? details?.text;
            if (typeof message === 'string') report(message.slice(0, 4_000));
        });
        try {
            if (report) await inspection.request('Runtime.enable', {}, deadline);
            while (!deadline.aborted) {
                const revision = inspection.navigation;
                // The target can advertise its destination while the main frame is still empty or blank.
                const location = await inspection.location(deadline);
                if (location === url) {
                    const result = await inspection.request<Evaluation>('Runtime.evaluate', { expression: presentedBridge, silent: true }, deadline);
                    if (!result.exceptionDetails && inspection.navigation === revision) {
                        const state = result.result?.value;
                        pending = state === 'frame' || state === 'hidden' ? state : 'inspector';
                        const objectId = result.result?.objectId;
                        if (objectId && await inspection.location(deadline) === url && inspection.navigation === revision) {
                            inspection.objectId = objectId;
                            return inspection;
                        }
                    }
                } else if (location && location !== 'about:blank') {
                    throw new Error('The Chromium page left the configured browser app URL. Update the profile to match its final address.');
                }
                await delay(125, undefined, { signal: deadline });
            }
        } catch (error) {
            signal.throwIfAborted();
            page.signal.throwIfAborted();
            if (!deadline.aborted) throw error;
        } finally { unsubscribe?.(); }
        throw new Error(startupErrors[pending]);
    }

    async tools(signal: AbortSignal): Promise<RuntimeToolDefinition[]> {
        const catalog = await this.inspect('tools', undefined, undefined, signal);
        if (!Array.isArray(catalog)) throw new Error('The browser app did not return an inspector tool catalog.');
        return catalog as RuntimeToolDefinition[];
    }

    async invoke(name: string, args: object, signal: AbortSignal): Promise<RuntimeToolResult> {
        const parameters = args as Record<string, unknown>;
        if (name === 'screenshot') return this.screenshot(parameters, signal);
        const value = name === 'wait_for' ? await this.waitFor(parameters, signal) : await this.inspect('call', name, args, signal);
        if (name === 'list_windows' && Array.isArray(value)) this.root = (value as RuntimeNode[])[0]?.node_id;
        return { content: [{ type: 'text', text: JSON.stringify(value) }] };
    }

    private async screenshot(args: Record<string, unknown>, signal: AbortSignal): Promise<RuntimeToolResult> {
        const scale = args.scale ?? 1;
        if (typeof scale !== 'number' || !Number.isFinite(scale) || scale < 0.1 || scale > 4
            || args.node_id !== undefined && !Number.isSafeInteger(args.node_id)
            || Object.keys(args).some(key => key !== 'scale' && key !== 'node_id')) throw new Error('Invalid browser screenshot arguments.');
        if (this.root === undefined) {
            const windows = await this.inspect('call', 'list_windows', {}, signal) as RuntimeNode[];
            this.root = windows[0]?.node_id;
        }
        if (this.root === undefined) throw new Error('The browser app has no inspectable window.');
        if (args.node_id !== undefined && args.node_id !== this.root) {
            const frame = await this.inspect('call', 'screenshot', args, signal) as { data?: string };
            return png(frame.data);
        }
        const bounds = (await this.evaluate(frameBounds, [], signal)) as Bounds | null;
        if (!bounds || ![bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) || bounds.x < 0 || bounds.y < 0
            || bounds.width <= 0 || bounds.height <= 0 || Math.ceil(bounds.width * scale) * Math.ceil(bounds.height * scale) > 16_000_000) {
            throw new Error('The browser app frame is empty or exceeds 16 megapixels.');
        }
        const frame = await this.request<{ data?: string }>('Page.captureScreenshot', {
            format: 'png', fromSurface: true, captureBeyondViewport: true, clip: { ...bounds, scale },
        }, signal);
        return png(frame.data);
    }

    private async waitFor(args: Record<string, unknown>, signal: AbortSignal): Promise<unknown> {
        const deadline = Date.now() + Math.min(30_000, typeof args.timeout_ms === 'number' ? args.timeout_ms : 5_000);
        for (;;) {
            const result = await this.inspect('call', 'wait_for', args, signal) as { matched: boolean };
            if (result.matched || Date.now() >= deadline) return result;
            await delay(100, undefined, { signal });
        }
    }

    private async inspect(method: string, tool: string | undefined, args: object | undefined, signal: AbortSignal): Promise<unknown> {
        const request = JSON.stringify({ method, tool, arguments: args });
        const value = await this.evaluate(dispatch, [{ value: request }], signal);
        if (typeof value !== 'string') throw new Error('The current Chromium document has no FerroUI inspector.');
        const reply = JSON.parse(value) as InspectionReply;
        if (!reply?.ok) throw new Error(reply?.error ?? 'The browser could not run the inspector tool.');
        return reply.value;
    }

    private async evaluate(functionDeclaration: string, args: { value: string }[], signal: AbortSignal): Promise<unknown> {
        const response = await this.request<Evaluation>('Runtime.callFunctionOn', {
            objectId: this.objectId, functionDeclaration, arguments: args, returnByValue: true, silent: true,
        }, signal);
        if (response.exceptionDetails) throw new Error('The browser could not run the FerroUI inspector.');
        return response.result?.value;
    }

    private async location(signal: AbortSignal): Promise<string> {
        const { frameTree: { frame } } = await this.request<FrameTree>('Page.getFrameTree', {}, signal);
        return frame.url + (frame.urlFragment ?? '');
    }

    private async request<T>(method: string, args: object, signal: AbortSignal): Promise<T> {
        const revision = this.navigation;
        const active = AbortSignal.any([signal, this.page.signal]);
        const response = await this.page.cdp.request<T>(method, args, active, this.page.sessionId);
        active.throwIfAborted();
        if (this.objectId && this.navigation !== revision) throw new Error('The Chromium app document has changed.');
        return response;
    }

    private onEvent(event: BrowserEvent): void {
        if (event.sessionId !== this.page.sessionId) return;
        if (event.method === 'Page.frameNavigated' && !(event.params as { frame?: { parentId?: string } })?.frame?.parentId) {
            this.navigation++;
            if (this.objectId) void this.page.close();
        }
    }
}

function png(data: string | undefined): RuntimeToolResult {
    if (typeof data !== 'string' || !data.startsWith('iVBORw0KGgo')) throw new Error('Chromium did not return a PNG frame.');
    return { content: [{ type: 'image', mimeType: 'image/png', data }] };
}
