import { setTimeout as delay } from 'node:timers/promises';
import type { TimelinePresentation, TimelinePresentationChange, TimelinePresentationReply, TimelinePresentationRequest } from './timelinePresentationProtocol';

/** Reports what the visible webview accepted, including changes made through its controls. */
export class TimelinePresentationRequests<Frame> {
    private next = 0;
    private readonly listeners = new Map<number, (reply: TimelinePresentationReply) => void>();

    constructor(private readonly post: (request: TimelinePresentationRequest<Frame>) => Thenable<boolean> | undefined) {}

    receive(reply: TimelinePresentationReply): void {
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

    async request(frame: Frame, current: () => boolean, signal: AbortSignal, change?: TimelinePresentationChange) {
        const timeout = AbortSignal.any([signal, AbortSignal.timeout(change ? 5_000 : 750)]);
        const request = ++this.next;
        const message: TimelinePresentationRequest<Frame> = { type: 'timelinePresentationRequest', request, frame, change };
        const check = () => {
            timeout.throwIfAborted();
            if (!current()) throw new Error('Read the current Timeline revision again.');
        };
        let resolveReply!: (presentation: TimelinePresentation) => void;
        let rejectReply!: (error: unknown) => void;
        const reply = new Promise<TimelinePresentation>((resolve, reject) => {
            resolveReply = resolve;
            rejectReply = reject;
        });
        // Start listening before posting. A fast webview can reply before postMessage resolves.
        this.listeners.set(request, result => {
            if ('error' in result) rejectReply(new Error(result.error));
            else resolveReply(result.presentation);
        });
        const cancel = () => rejectReply(signal.aborted ? signal.reason : new Error('The Timeline did not respond.'));
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
                if (!sent) rejectReply(new Error('Open the Timeline to show its current controls.'));
            }, rejectReply);
            const presentation = await reply;
            check();
            return presentation;
        } finally {
            timeout.removeEventListener('abort', cancel);
            this.listeners.delete(request);
        }
    }
}
