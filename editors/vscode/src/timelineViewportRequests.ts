import { setTimeout as delay } from 'node:timers/promises';
import type { TimelineViewportChange, TimelineViewportReply, TimelineViewportRequest, TimelineViewportState } from './timelineViewportProtocol';

/** Requests the ruler's actual state from the visible webview, including gestures made there. */
export class TimelineViewportRequests<Frame> {
    private next = 0;
    private readonly listeners = new Map<number, (reply: TimelineViewportReply) => void>();

    constructor(private readonly post: (request: TimelineViewportRequest<Frame>) => Thenable<boolean> | undefined) {}

    receive(reply: TimelineViewportReply): void {
        this.listeners.get(reply.request)?.(reply);
    }

    async read(frame: Frame, current: () => boolean, signal: AbortSignal) {
        try {
            return await this.request(frame, current, signal);
        } catch {
            signal.throwIfAborted();
            return undefined;
        }
    }

    async request(frame: Frame, current: () => boolean, signal: AbortSignal, change?: TimelineViewportChange) {
        const timeout = AbortSignal.any([signal, AbortSignal.timeout(change ? 5_000 : 750)]);
        const request = ++this.next;
        const message: TimelineViewportRequest<Frame> = { type: 'timelineViewportRequest', request, frame, change };
        const check = () => {
            timeout.throwIfAborted();
            if (!current()) throw new Error('Read the current Timeline revision again.');
        };
        let resolveReply!: (viewport?: TimelineViewportState) => void;
        let rejectReply!: (error: unknown) => void;
        const reply = new Promise<TimelineViewportState | undefined>((resolve, reject) => {
            resolveReply = resolve;
            rejectReply = reject;
        });
        // Start listening before posting. A fast webview can reply before postMessage resolves.
        this.listeners.set(request, result => {
            if ('error' in result) rejectReply(new Error(result.error));
            else resolveReply(result.viewport);
        });
        const cancel = () => rejectReply(signal.aborted ? signal.reason : new Error('The Timeline ruler did not respond.'));
        timeout.addEventListener('abort', cancel, { once: true });
        // The reply can reject while the webview is becoming available.
        void reply.catch(() => {});
        try {
            check();
            let posted = this.post(message);
            while (!posted) {
                if (!change) return undefined;
                await delay(30, undefined, { signal: timeout });
                check();
                posted = this.post(message);
            }
            void posted.then(sent => {
                if (!sent) rejectReply(new Error('Open the Timeline to show its current ruler.'));
            }, rejectReply);
            const viewport = await reply;
            check();
            return viewport;
        } finally {
            timeout.removeEventListener('abort', cancel);
            this.listeners.delete(request);
        }
    }
}
