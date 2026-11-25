export const continueAppMessage = 'The app is paused in the debugger. Continue before using the live inspector.';
const completionDeadline = 28_000;

export interface PauseState {
    readonly paused: boolean;
    onDidChange(listener: (paused: boolean) => void): () => void;
}

export interface DebuggerStatus { attached: boolean; paused: boolean }
export const detachedDebugger: Readonly<DebuggerStatus> = { attached: false, paused: false };

/** Aggregates stops from the adapters that belong to one debugger attachment. */
export class DebuggerPause implements PauseState {
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

    private notify(): void {
        for (const listener of this.listeners) listener(this.paused);
    }
}

/** Let an in-flight inspector action finish after Continue within a bounded overall deadline. */
export async function withInspectorDeadline<T>(pause: PauseState, signal: AbortSignal, idleTimeout: number,
    send: (active: AbortSignal) => Promise<T>): Promise<T> {
    signal.throwIfAborted();
    if (pause.paused) throw new Error(continueAppMessage);
    const timeout = new AbortController();
    let paused = false;
    let pausedDuringRequest = false;
    const expired = () => timeout.abort(new Error(pausedDuringRequest
        ? 'The inspector action did not complete before its deadline. Continue and check the app before repeating it.'
        : 'The app inspector request timed out.'));
    const overall = setTimeout(expired, completionDeadline);
    let timer = setTimeout(expired, idleTimeout);
    const dispose = pause.onDidChange(next => {
        if (next === paused) return;
        paused = next;
        clearTimeout(timer);
        if (paused) pausedDuringRequest = true;
        else timer = setTimeout(expired, idleTimeout);
    });
    try { return await send(AbortSignal.any([signal, timeout.signal])); }
    finally {
        dispose();
        clearTimeout(timer);
        clearTimeout(overall);
    }
}
