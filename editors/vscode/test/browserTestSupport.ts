import { setTimeout as delay } from 'node:timers/promises';

export function browserBuildTimeout(): number {
    const timeout = Number(process.env.FERROUI_TEST_BROWSER_BUILD_TIMEOUT_MS);
    return Number.isFinite(timeout) && timeout > 0 ? timeout : 90_000;
}

export async function waitFor<T>(check: () => T | undefined | Promise<T | undefined>, title: string, timeout = 30_000): Promise<T> {
    const deadline = Date.now() + timeout;
    for (;;) {
        const value = await check();
        if (value !== undefined) return value;
        if (Date.now() >= deadline) throw new Error(`Timed out waiting for ${title}.`);
        await delay(180);
    }
}
