import { setTimeout as delay } from 'node:timers/promises';

export async function until(check: () => boolean | Promise<boolean>): Promise<void> {
    const deadline = Date.now() + 20_000;
    while (Date.now() < deadline) {
        if (await check()) return;
        await delay(150);
    }
    throw new Error('Timed out waiting for a VS Code extension result');
}
