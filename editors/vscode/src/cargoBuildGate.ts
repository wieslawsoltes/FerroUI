import { fileKey } from './workspacePaths';

const queues = new Map<string, Promise<void>>();

/** Keep the build and its executable copy in the same slot. Cargo releases its own filesystem
 * lock before the extension has copied the artifact for a particular set of features. */
export async function inCargoBuildSlot<T>(targetDirectory: string, signal: AbortSignal, build: () => Promise<T>): Promise<T> {
    const key = fileKey(targetDirectory);
    const previous = queues.get(key) ?? Promise.resolve();
    let release!: () => void;
    const turn = new Promise<void>(resolve => { release = resolve; });
    const tail = previous.then(() => turn);
    queues.set(key, tail);
    try {
        await waitForTurn(previous, signal);
        signal.throwIfAborted();
        return await build();
    } finally {
        release();
        void tail.then(() => { if (queues.get(key) === tail) queues.delete(key); });
    }
}

async function waitForTurn(previous: Promise<void>, signal: AbortSignal): Promise<void> {
    signal.throwIfAborted();
    let abort!: () => void;
    const cancelled = new Promise<never>((_, reject) => { abort = () => reject(signal.reason); });
    signal.addEventListener('abort', abort, { once: true });
    try { await Promise.race([previous, cancelled]); }
    finally { signal.removeEventListener('abort', abort); }
}
