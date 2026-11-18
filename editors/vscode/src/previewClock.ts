import { setTimeout as delay } from 'node:timers/promises';

/** Advance accepted native frames against a monotonic clock without queuing renders. */
export async function drivePreviewClock(running: () => boolean, step: (seconds: number) => Promise<boolean>): Promise<void> {
    let last = performance.now();
    while (running()) {
        await delay(16);
        if (!running()) return;
        const now = performance.now();
        const seconds = (now - last) / 1000;
        last = now;
        if (!await step(seconds)) return;
    }
}
