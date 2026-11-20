import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, mkdtemp, rm, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { ReloadHistory } from '../src/reloadHistory';
import { parseFrameReceipt, type RuntimeFrameReceipt } from '../src/reloadFrames';
import { ReloadProcess } from '../src/reloadProcess';

export async function verifyProcessShutdown(): Promise<void> {
    const directory = await mkdtemp(join(tmpdir(), 'ferroui-reload-stop-'));
    const copy = join(directory, 'launch-copy');
    await writeFile(copy, 'temporary launch artifact');
    const child = spawn(process.execPath, ['-e', `
        process.on('SIGTERM', () => {});
        process.stdout.write('ready');
        setInterval(() => {}, 1000);
    `], { detached: process.platform !== 'win32', windowsHide: true,
        stdio: ['ignore', 'pipe', 'ignore'], env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' } });
    const processHandle = new ReloadProcess(child, async () => {
        await delay(25);
        await unlink(copy);
    });
    try {
        await Promise.race([once(child.stdout, 'data'), processHandle.closed.then(() => { throw new Error('The test process exited before startup'); })]);
        await processHandle.stop();
        assert.ok(child.exitCode !== null || child.signalCode !== null);
        await assert.rejects(access(copy), { code: 'ENOENT' });
    } finally {
        await processHandle.stop();
        await rm(directory, { recursive: true, force: true });
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

        const previousGeneration = history.startGeneration();
        const retired = history.observe('View.rs', 'first', 1000);
        history.finish(retired, 'applied', 27, undefined, { generation: previousGeneration, frame: 1 });
        const currentGeneration = history.startGeneration();
        assert.equal(retired.frame?.status, 'processExited');
        const receipt: RuntimeFrameReceipt = { frame: 1, batch: 21, status: 'rendered', unixMs: 1040, afterAcknowledgementMs: 10 };
        history.observeFrame(previousGeneration, { ...receipt, batch: 3 });
        history.observeFrame(currentGeneration, receipt);
        const xaml = history.observe('View.xaml', 'first', 1010);
        const rust = history.observe('View.rs', 'second', 1020);
        const startup = { generation: currentGeneration, frame: 1, acknowledgedAt: 1035 };
        history.finish(xaml, 'rebuilt', 31, undefined, startup);
        history.finish(rust, 'rebuilt', 31, undefined, startup);
        assert.deepEqual(xaml.frame, { receipt: 1, batch: 21, status: 'rendered', elapsedMs: 30, afterAcknowledgementMs: 10 });
        assert.deepEqual(rust.frame, { ...xaml.frame, elapsedMs: 20 });
        assert.equal(xaml.acknowledgedMs, 25);
        assert.equal(rust.acknowledgedMs, 15);
        assert.equal(retired.frame?.batch, undefined);
        const rejected = history.observe('View.xaml', 'invalid', 1030);
        history.finish(rejected, 'rejected', 31, 'Invalid XAML');
        const idle = history.observe('View.rs', 'comment', 1040);
        history.finish(idle, 'applied', 31, undefined, { generation: currentGeneration, frame: 2 });
        history.observeFrame(currentGeneration, { ...receipt, frame: 2, batch: 22, status: 'noSceneUpdate' });
        assert.equal(idle.frame?.elapsedMs, undefined);
        assert.equal(idle.frame?.status, 'noSceneUpdate');
        assert.equal(rejected.frame, undefined);
        const lateObservation = history.observe('View.rs', 'observed after render', 1060);
        history.finish(lateObservation, 'applied', 31, undefined, { generation: currentGeneration, frame: 3, acknowledgedAt: 1045 });
        history.observeFrame(currentGeneration, { ...receipt, frame: 3 });
        assert.equal(lateObservation.acknowledgedMs, undefined);
        assert.equal(lateObservation.frame?.elapsedMs, undefined);
        assert.equal(lateObservation.frame?.status, 'rendered');
        assert.deepEqual(parseFrameReceipt(JSON.stringify(receipt)), receipt);
        assert.equal(parseFrameReceipt(JSON.stringify({ ...receipt, frame: -1 })), undefined);
    } finally { history.dispose(); }
}
