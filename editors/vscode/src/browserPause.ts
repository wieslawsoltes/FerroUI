import { browserRequestTimeout } from './browserCdp';

export const continueBrowserMessage = 'The app is paused in the debugger. Continue before using the live inspector.';
const completionDeadline = 28_000;

/** Shares one browser document's paused state between its debugger and inspector. */
export class BrowserPause {
    private readonly stopped = new Set<symbol>();
    private readonly listeners = new Set<(paused: boolean) => void>();

    get paused(): boolean { return this.stopped.size > 0; }

    set(adapter: symbol, paused: boolean): void {
        const before = this.paused;
        if (paused) this.stopped.add(adapter);
        else this.stopped.delete(adapter);
        if (before !== this.paused) this.notify();
    }

    clear(): void {
        if (!this.paused) return;
        this.stopped.clear();
        this.notify();
    }

    onDidChange(listener: (paused: boolean) => void): () => void {
        this.listeners.add(listener);
        return () => { this.listeners.delete(listener); };
    }

    async inspect<T>(signal: AbortSignal, send: (active: AbortSignal) => Promise<T>): Promise<T> {
        signal.throwIfAborted();
        if (this.paused) throw new Error(continueBrowserMessage);
        const timeout = new AbortController();
        let pausedDuringRequest = false;
        const expired = () => timeout.abort(new Error(pausedDuringRequest
            ? 'The inspector action did not complete before its deadline. Continue and check the app before repeating it.'
            : 'The browser inspector request timed out.'));
        const overall = setTimeout(expired, completionDeadline);
        let timer = setTimeout(expired, browserRequestTimeout);
        const dispose = this.onDidChange(paused => {
            clearTimeout(timer);
            if (paused) pausedDuringRequest = true;
            else timer = setTimeout(expired, browserRequestTimeout);
        });
        try { return await send(AbortSignal.any([signal, timeout.signal])); }
        finally {
            dispose();
            clearTimeout(timer);
            clearTimeout(overall);
        }
    }

    private notify(): void {
        for (const listener of this.listeners) listener(this.paused);
    }
}
