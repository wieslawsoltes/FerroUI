import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { ReloadHistory } from '../src/reloadHistory';
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

export function verifyReloadHistory(): void {
    const history = new ReloadHistory();
    try {
        const original = history.observe('View.xaml', 'old', Date.now() - 500);
        const update = history.observe('View.xaml', 'new', Date.now());
        history.finish(original, 'superseded', 27);
        history.finish(update, 'applied', 27);
        history.finish(original, 'rebuilt', 31);
        const [last, previous] = history.recent(2);
        assert.deepEqual([last.revision, last.outcome, last.pid], ['new', 'applied', 27]);
        assert.deepEqual([previous.revision, previous.outcome, previous.pid], ['old', 'superseded', 27]);
        assert.ok(previous.elapsedMs !== undefined && previous.elapsedMs >= 500);
    } finally { history.dispose(); }
}
