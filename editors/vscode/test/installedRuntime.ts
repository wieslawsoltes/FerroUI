import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { KeyboardFocus, KeyboardInput } from '../src/previewKeyboardProtocol';
import { revision } from '../src/reloadProcess';
import { RuntimeClient } from '../src/runtimeClient';
import { localMcpEndpoint, type RuntimeMcpEndpoint } from '../src/runtimeMcp';
import type { RuntimeNode, RuntimeProperty, RuntimeSource } from '../src/runtimeProtocol';
import { installedRuntimeDebug } from './installedRuntimeDebug';
import { until } from './wait';

export async function verifyInstalledRuntime(workspace: vscode.WorkspaceFolder): Promise<void> {
    const receipts = process.env.FERROUI_TEST_RUNTIME_RECEIPTS;
    if (!receipts) return;
    const fixture = vscode.Uri.joinPath(workspace.uri, 'installed-runtime');
    const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(fixture, 'main.rs'));
    const xaml = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(fixture, 'Status.xaml'));
    const debugName = process.env.FERROUI_TEST_RUNTIME_DEBUG;
    let debug = debugName ? installedRuntimeDebug(rust, debugName) : undefined;
    let attachFailure: unknown;
    let tabs: vscode.Disposable | undefined;

    const clients: RuntimeClient[] = [];
    async function inspector(pid: number) {
        const endpoint = JSON.parse(await readFile(path.join(receipts!, `${pid}-inspector.json`), 'utf8')) as RuntimeMcpEndpoint;
        assert.ok(localMcpEndpoint(endpoint.url) && /^[a-f0-9]{64}$/.test(endpoint.token));
        const client = new RuntimeClient({ ...endpoint, expectedProcess: { pid } });
        clients.push(client);
        return client;
    }
    try {
        await vscode.commands.executeCommand('ferroui.showAppSessions');
        await vscode.commands.executeCommand(debug ? 'ferroui.hotReload.runAndDebug' : 'ferroui.hotReload.start');
        let pid = await acceptedProcess();
        if (debug) {
            try { await debug.accept(pid); }
            catch (error) {
                attachFailure = error;
                console.error('Initial debugger acceptance failed. Running the native reload checks in a fresh app session.', error);
                await debug.close();
                debug = undefined;
                await vscode.commands.executeCommand('ferroui.hotReload.stopAll');
                await vscode.commands.executeCommand('ferroui.hotReload.start');
                pid = await acceptedProcess();
            }
        }
        const app = await inspector(pid);
        await expectText(app, 'RsxStatus', 'Initial RSX status');
        await expectText(app, 'XamlStatus', 'Initial XAML status');
        const initialFrame = await screenshot(app);
        await vscode.commands.executeCommand('ferroui.openLiveApp');
        await until(() => Boolean(liveTab()));
        await verifyWindowKeyboard(app);
        const tabChanges: string[] = [];
        tabs = vscode.window.tabGroups.onDidChangeTabs(({ opened, closed }) => {
            if (opened.some(isLiveTab)) tabChanges.push('opened');
            if (closed.some(isLiveTab)) tabChanges.push('closed');
        });
        await approve(app);
        await expectText(app, 'SavedCount', 'Reviews saved: 1');

        const markup = xaml.getText();
        await save(xaml, markup.replace('</ContentControl>', '</Border>'));
        await until(async () => (await readFile(path.join(receipts, `${pid}-xaml.txt`), 'utf8').catch(() => '')) === 'Failed');
        assert.equal(await runningPid(), pid);
        await expectText(app, 'XamlStatus', 'Initial XAML status');
        await save(xaml, markup.replace('Initial XAML status', 'Review delayed'));
        await save(xaml, markup.replace('Initial XAML status', 'Approved XAML status'));
        await expectText(app, 'XamlStatus', 'Approved XAML status');
        assert.equal(await runningPid(), pid);

        await save(rust, rust.getText().replace('Initial RSX status', 'Approved RSX status'));
        await expectText(app, 'RsxStatus', 'Approved RSX status');
        assert.equal(await runningPid(), pid);
        assert.equal(app.processIdentity?.pid, pid);
        assert.notEqual(await screenshot(app), initialFrame);

        const liveRust = rust.getText();
        await save(rust, liveRust + '\ncompile_error!("Rejected installed release build");\n');
        await until(async () => Number(await readFile(path.join(receipts, 'failed-build'), 'utf8').catch(() => '')) > 0, 90_000);
        assert.equal(await runningPid(), pid);
        await expectText(app, 'BuildStatus', 'Original build');
        debug?.attached();

        await save(rust, liveRust.replace('Original build', 'Rejected startup'));
        await until(async () => {
            const rejected = Number(await readFile(path.join(receipts, 'rejected-startup'), 'utf8').catch(() => ''));
            return rejected > 0 && !processExists(rejected);
        }, 90_000);
        assert.equal(await runningPid(), pid);
        await expectText(app, 'BuildStatus', 'Original build');
        await approve(app);
        await expectText(app, 'SavedCount', 'Reviews saved: 2');
        await save(rust, liveRust.replace('Original build', 'Accepted replacement'));
        const replacement = await acceptedProcess(pid);
        await debug?.accept(replacement);
        const updated = await inspector(replacement);
        await expectText(updated, 'BuildStatus', 'Accepted replacement');
        await expectText(updated, 'RsxStatus', 'Approved RSX status');
        await expectText(updated, 'XamlStatus', 'Approved XAML status');
        await expectText(updated, 'SavedCount', 'Reviews saved: 2');
        await screenshot(updated);
        const [node] = await updated.call<RuntimeNode[]>('find_elements', { name: 'RsxStatus' });
        const source = await updated.call<RuntimeSource>('get_source', { node_id: node.node_id });
        assert.equal(source.language, 'rust');
        assert.equal(source.revision, revision(Buffer.from(rust.getText())), 'The replacement inspector maps the saved RSX revision');
        await until(() => !processExists(pid));
        if (debug) {
            await approve(updated);
            const disconnect = await debug.detach();
            await expectDetachedRuntime(updated, replacement, disconnect);
            assert.equal(await runningPid(), replacement);
            console.log('Installed LLDB verified Rust breakpoints, retained failed candidates, followed the new PID and detached from the live app.');
        }
        assert.ok(liveTab());
        assert.deepEqual(tabChanges, [], 'Live App stays open through the accepted replacement');
        console.log('Installed Live App passed in-place XAML and RSX saves, rejected edits, failed compilation and startup, and state restoration after rebuild.');
        if (attachFailure !== undefined) throw attachFailure;
    } finally {
        tabs?.dispose();
        await debug?.close();
        clients.forEach(client => client.dispose());
        await vscode.commands.executeCommand('ferroui.hotReload.stopAll');
        const tab = liveTab();
        if (tab) await vscode.window.tabGroups.close(tab);
    }

    async function approve(app: RuntimeClient) {
        const [button] = await app.call<RuntimeNode[]>('find_elements', { name: 'ReviewRecord' });
        const click = () => app.call('click', { node_id: button.node_id });
        if (debug) await debug.click(click);
        else await click();
    }
}

function liveTab() {
    return vscode.window.tabGroups.all.flatMap(group => group.tabs).find(isLiveTab);
}

function isLiveTab(tab: vscode.Tab): boolean { return tab.label === 'FerroUI Live App'; }

function processExists(pid: number): boolean {
    try { process.kill(pid, 0); return true; }
    catch (error) {
        if ((error as NodeJS.ErrnoException).code === 'ESRCH') return false;
        throw error;
    }
}

async function runningPid(): Promise<number | undefined> {
    const pid = Number(await vscode.commands.executeCommand('ferroui.hotReload.processId').then(value => value, () => undefined));
    if (Number.isSafeInteger(pid) && pid > 0) return pid;
}

async function acceptedProcess(previous?: number): Promise<number> {
    let pid: number | undefined;
    await until(async () => {
        pid = await runningPid();
        return pid !== undefined && pid !== previous;
    }, 90_000);
    return pid!;
}

async function save(document: vscode.TextDocument, text: string): Promise<void> {
    assert.notEqual(document.getText(), text);
    const edit = new vscode.WorkspaceEdit();
    edit.replace(document.uri, new vscode.Range(new vscode.Position(0, 0), document.positionAt(document.getText().length)), text);
    assert.ok(await vscode.workspace.applyEdit(edit) && await document.save());
}

async function expectText(client: RuntimeClient, name: string, text: string): Promise<void> {
    await until(async () => (await client.call<RuntimeNode[]>('find_elements', { name }))[0]?.text === text);
}

async function expectDetachedRuntime(client: RuntimeClient, pid: number, debuggerStatus: object): Promise<void> {
    let transportError: unknown;
    try {
        await until(async () => {
            assert.ok(processExists(pid), 'Debugger disconnect keeps the accepted native process alive');
            try {
                const [counter] = await client.call<RuntimeNode[]>('find_elements', { name: 'SavedCount' });
                return counter?.text === 'Reviews saved: 3';
            } catch (error) {
                // Connection recovery after resuming the target can take another poll.
                if (!(error instanceof TypeError) || error.message !== 'fetch failed') throw error;
                transportError = error.cause;
                return false;
            }
        });
    } catch (error) {
        const network = (transportError as { code?: string } | undefined)?.code;
        const detail = { pid, alive: processExists(pid), debugger: debuggerStatus, network };
        throw new Error(`The accepted inspector did not respond after debugger disconnect. ${JSON.stringify(detail)}`, { cause: error });
    }
}

async function verifyWindowKeyboard(client: RuntimeClient): Promise<void> {
    const [window] = await client.call<RuntimeNode[]>('list_windows');
    const [draft] = await client.call<RuntimeNode[]>('find_elements', { name: 'ReviewDraft' });
    await client.call('click', { node_id: draft.node_id });
    const input = (input: KeyboardInput) => client.call<KeyboardFocus>('window_keyboard', { node_id: window.node_id, input });
    assert.equal((await input({ kind: 'focus' })).role, 'Edit');
    await input({ kind: 'text', text: 'Release' });
    await input({ kind: 'key', key: 'Space' });
    await input({ kind: 'text', text: 'review 📝' });
    await until(async () => (await client.call<RuntimeProperty[]>('get_properties',
        { node_id: draft.node_id, properties: ['Text'] }))[0]?.value === 'Release review 📝');
    assert.match((await input({ kind: 'key', key: 'Tab' })).role, /^Button$/);
    assert.equal((await input({ kind: 'key', key: 'Shift+Tab' })).role, 'Edit');
}

async function screenshot(client: RuntimeClient): Promise<string> {
    const [window] = await client.call<RuntimeNode[]>('list_windows');
    const image = await client.call<string>('screenshot', { node_id: window.node_id });
    assert.ok(image.startsWith('iVBORw0KGgo'), 'The installed runtime presents a native PNG frame');
    return image;
}
