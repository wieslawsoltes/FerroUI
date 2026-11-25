import { setTimeout as delay } from 'node:timers/promises';

export async function until(check: () => boolean | Promise<boolean>, timeout = 20_000): Promise<void> {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
        if (await check()) return;
        await delay(150);
    }
    throw new Error('Timed out waiting for a VS Code extension result');
}
