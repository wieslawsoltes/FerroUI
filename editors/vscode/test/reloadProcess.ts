import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, mkdir, mkdtemp, rm, stat, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import { inCargoBuildSlot } from '../src/cargoBuildGate';
import { ReloadHistory } from '../src/reloadHistory';
import { parseFrameReceipt, type RuntimeFrameReceipt } from '../src/reloadFrames';
import { ReloadProcess } from '../src/reloadProcess';
import { fileKey } from '../src/reloadScope';
import { ReloadSession } from '../src/reloadSession';
import { until } from './wait';

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

export async function verifyReloadWatchBoundary(folder: vscode.WorkspaceFolder): Promise<void> {
    const root = await mkdtemp(join(folder.uri.fsPath, 'reload-watch-'));
    const file = join(root, 'View.xaml');
    const manifest = join(root, 'Cargo.toml');
    const history = new ReloadHistory();
    try {
        const physical = join(root, 'cargo-output');
        const alias = join(root, 'linked-output');
        await mkdir(physical);
        await symlink(physical, alias, process.platform === 'win32' ? 'junction' : 'dir');
        const targetDirectory = join(alias, 'dev', 'target');
        const before = fileKey(targetDirectory);
        assert.equal(before, fileKey(join(physical, 'dev', 'target')));
        await mkdir(targetDirectory, { recursive: true });
        assert.equal(fileKey(targetDirectory), before);

        await writeFile(file, '<Grid/>');
        const info = await stat(file);
        await until(() => Date.now() > Math.max(info.ctimeMs, info.mtimeMs, info.birthtimeMs));
        const session = new ReloadSession({ label: 'watch', folder, cargo: 'cargo', cargoDirectory: root,
            discoveryManifest: manifest, manifest, packageId: 'watch', packageName: 'watch', binary: 'watch',
            features: [], args: [], targetDirectory: join(root, 'target') },
        { session: 'watch', target: 'watch' }, { append() {}, appendLine() {} }, history, () => {}, () => {}, () => {}, () => {});
        try {
            await session.observe(vscode.Uri.file(file), true);
            assert.deepEqual(history.recent(10, 'watch'), [], 'A delayed create from before launch is part of the initial build');
            await writeFile(file, '<Border/>');
            await session.observe(vscode.Uri.file(file), true);
            assert.equal(history.recent(10, 'watch').length, 1, 'An edit after launch remains queued');
        } finally { await session.dispose(); }
    } finally {
        history.dispose();
        await rm(root, { recursive: true, force: true });
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

        const firstApp = { session: 'first', target: 'Account Desk' };
        const secondApp = { session: 'second', target: 'Routing Queue' };
        const firstProcess = history.startGeneration(firstApp.session);
        const secondProcess = history.startGeneration(secondApp.session);
        const firstSave = history.observe('Account.xaml', 'same-receipt', 1000, firstApp);
        const secondSave = history.observe('Queue.xaml', 'same-receipt', 1000, secondApp);
        history.finish(firstSave, 'applied', 34, undefined, { generation: firstProcess, frame: 1 });
        history.finish(secondSave, 'applied', 54, undefined, { generation: secondProcess, frame: 1 });
        history.startGeneration(firstApp.session);
        assert.equal(firstSave.frame?.status, 'processExited');
        assert.equal(secondSave.frame?.status, 'pending');
        history.observeFrame(secondProcess, { ...receipt, batch: 25 });
        assert.equal(secondSave.frame?.batch, 25);
        assert.deepEqual(history.recent(50, secondApp.session), [secondSave]);
    } finally { history.dispose(); }
}

export async function verifyCargoBuildSlots(): Promise<void> {
    const signal = new AbortController().signal;
    const cancelled = new AbortController();
    const events: string[] = [];
    let release!: () => void;
    const finish = new Promise<void>(resolve => { release = resolve; });
    const first = inCargoBuildSlot('shared-target', signal, async () => {
        events.push('first build');
        await finish;
        events.push('first executable copied');
    });
    const queued = inCargoBuildSlot('shared-target', cancelled.signal, async () => { events.push('cancelled build'); });
    const later = inCargoBuildSlot('shared-target', signal, async () => { events.push('second build'); });
    try {
        cancelled.abort();
        await assert.rejects(queued, { name: 'AbortError' });
        await inCargoBuildSlot('independent-target', signal, async () => { events.push('independent build'); });
        assert.deepEqual(events, ['first build', 'independent build']);
    } finally { release(); await Promise.all([first, later]); }
    assert.deepEqual(events, ['first build', 'independent build', 'first executable copied', 'second build']);
}
