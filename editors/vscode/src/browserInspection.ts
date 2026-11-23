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
const presentedBridge = `(() => { const app = (${bridge}); return app?.version === 1 && app.ready() ? app : null; })()`;
const connected = `this === (${bridge}) && this.version === 1`;
const dispatch = `function (request) { return (${connected}) ? this.dispatch(request) : null; }`;
const frameBounds = `function () { return (${connected}) ? this.bounds() : null; }`;

/** One channel per Chromium document. Reopening a tab creates a new runtime generation. */
export class BrowserInspection implements RuntimeInspectionChannel {
    private objectId?: string;
    private root?: number;
    private navigation = 0;

    private constructor(private readonly page: BrowserPage) {
        this.page.cdp.onEvent(event => this.onEvent(event));
    }

    static async open(page: BrowserPage, url: string, signal: AbortSignal): Promise<BrowserInspection> {
        const inspection = new BrowserInspection(page);
        const deadline = AbortSignal.any([signal, AbortSignal.timeout(15_000)]);
        try {
            while (!deadline.aborted) {
                const revision = inspection.navigation;
                // The target can advertise its destination while the main frame is still empty or blank.
                const location = await inspection.location(deadline);
                if (location === url) {
                    const result = await inspection.request<Evaluation>('Runtime.evaluate', { expression: presentedBridge, silent: true }, deadline);
                    if (!result.exceptionDetails && result.result?.objectId && inspection.navigation === revision) {
                        if (await inspection.location(deadline) === url && inspection.navigation === revision) {
                            inspection.objectId = result.result.objectId;
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
            if (!deadline.aborted) throw error;
        }
        throw new Error('The browser app did not provide a Live App inspector. Serve a debug build with the inspector installed.');
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
        const response = await this.page.cdp.request<T>(method, args, signal, this.page.sessionId);
        signal.throwIfAborted();
        if (this.objectId && this.navigation !== revision) throw new Error('The Chromium app document has changed.');
        return response;
    }

    private onEvent(event: BrowserEvent): void {
        if (event.method === 'Target.detachedFromTarget' && this.page.sessionId
            && (event.params as { sessionId?: string })?.sessionId === this.page.sessionId) this.page.cdp.close();
        if (event.sessionId !== this.page.sessionId) return;
        if (event.method === 'Inspector.detached' || event.method === 'Inspector.targetCrashed') this.page.cdp.close();
        if (event.method === 'Page.frameNavigated' && !(event.params as { frame?: { parentId?: string } })?.frame?.parentId) {
            this.navigation++;
            if (this.objectId) this.page.cdp.close();
        }
    }
}

function png(data: string | undefined): RuntimeToolResult {
    if (typeof data !== 'string' || !data.startsWith('iVBORw0KGgo')) throw new Error('Chromium did not return a PNG frame.');
    return { content: [{ type: 'image', mimeType: 'image/png', data }] };
}
