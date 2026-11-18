import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { ReloadProcess } from '../src/reloadProcess';

export async function verifyProcessShutdown(): Promise<void> {
    const child = spawn(process.execPath, ['-e', `
        process.on('SIGTERM', () => {});
        process.stdout.write('ready');
        setInterval(() => {}, 1000);
    `], { detached: process.platform !== 'win32', windowsHide: true,
        stdio: ['ignore', 'pipe', 'ignore'], env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' } });
    const processHandle = new ReloadProcess(child);
    try {
        await Promise.race([once(child.stdout, 'data'), processHandle.closed.then(() => { throw new Error('The test process exited before startup'); })]);
        await processHandle.stop();
        assert.ok(child.exitCode !== null || child.signalCode !== null);
    } finally {
        await processHandle.stop();
    }
}
