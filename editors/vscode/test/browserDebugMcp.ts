import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { BrowserSites } from '../src/browserSites';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import { runtimeApp, verifyBrowserAttach } from './browserMcp';
import { openEditorMcp } from './mcpFixture';
import { until } from './wait';

export async function verifyBrowserDebug(): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const script = vscode.Uri.joinPath(workspace, 'wwwroot', 'probe.js');
    const failMarker = vscode.Uri.joinPath(workspace, 'fail-build');
    const editor = await vscode.window.showTextDocument(script);
    const point = new vscode.SourceBreakpoint(new vscode.Location(script, new vscode.Position(2, 4)));
    const hits: { session: vscode.DebugSession; thread: number }[] = [];
    const terminated = new Set<string>();
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('*', {
        createDebugAdapterTracker(session) {
            return { onDidSendMessage(message) {
                if (message.event !== 'stopped' || message.body?.reason !== 'breakpoint') return;
                const owner = session.parentSession ?? session;
                // The running page can hit its interval breakpoint again before its replacement is accepted.
                if (!owner.configuration.ferrouiManagedInstance || hits.some(hit => (hit.session.parentSession ?? hit.session).id === owner.id)) return;
                hits.push({ session, thread: message.body.threadId });
            } };
        },
    });
    const ending = vscode.debug.onDidTerminateDebugSession(session => terminated.add(session.id));
    vscode.debug.addBreakpoints([point]);
    const mcp = await openEditorMcp(38_000);
    const { invoke, content } = mcp;
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = config.inspect<boolean>('allowWrites')?.globalValue;
    const browser = vscode.workspace.getConfiguration('ferroui.browser');
    const profiles = browser.inspect<unknown[]>('apps')?.globalValue ?? [];
    const sites = new BrowserSites();
    let runtime: string | undefined;
    let servedRuntime: string | undefined;
    const readOutput = async (runtime: string, lines = 60) => {
        const result = content<{ runtime: string; output: string }>(await invoke('ferroui_browser_output', { runtime, lines }));
        assert.equal(result.runtime, runtime);
        return result.output;
    };
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    const current = async () => (await sessions()).find(session => session.runtime === runtime);
    const accepted = async (previous?: number | null) => {
        let result: RuntimeSummary | undefined;
        await until(async () => {
            result = await current();
            return Boolean(result?.available && result.generation !== previous);
        });
        return result!;
    };
    const breakpoint = async (index: number, expected: RuntimeSummary) => {
        await until(() => Boolean(hits[index]));
        const { session, thread } = hits[index];
        const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId: thread });
        assert.equal(frame.line, 3);
        assert.equal(fileKey(frame.source.path), fileKey(script.fsPath));
        const evaluation = await session.customRequest('evaluate', { frameId: frame.id,
            expression: `location.href === ${JSON.stringify(expected.browser?.url)}`, context: 'watch' });
        assert.equal(evaluation.result, 'true');
        return session.parentSession ?? session;
    };
    try {
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const queued = content<RuntimeSummary>(await invoke('ferroui_browser_run', { profile: 'debug-dispatch', folder: workspace.toString(), debug: true }));
        runtime = queued.runtime;
        assert.equal(queued.browser?.debuggable, true);
        assert.equal(queued.debuggerAttached, false);
        const initial = await accepted();
        const old = await breakpoint(0, initial);
        await until(async () => (await current())?.debuggerAttached === true);
        const stopped = await current();
        assert.equal(stopped?.browser?.paused, true, 'The accepted Chromium page reports its debugger pause.');
        assert.equal(stopped?.debuggerPaused, true, 'The session exposes the same pause state as native apps.');
        const blocked = await invoke('ferroui_runtime_query', { runtime, expected_generation: initial.generation, tool: 'list_windows' });
        assert.ok(blocked.isError);
        assert.match(blocked.content[0]?.text ?? '', /paused in the debugger/);
        const duplicate = content<RuntimeSummary>(await invoke('ferroui_browser_run', { profile: 'debug-dispatch', folder: workspace.toString(), debug: true }));
        assert.equal(duplicate.generation, initial.generation);
        assert.ok(!terminated.has(old.id));

        await sites.listen();
        const { url } = await sites.publish(process.env.FERROUI_TEST_BROWSER_DEBUG_SITE!, false);
        const servedProfile = { id: 'served-review', label: 'Review Desk', url };
        await browser.update('apps', [...profiles, servedProfile], vscode.ConfigurationTarget.Global);
        const served = content<RuntimeSummary>(await invoke('ferroui_browser_run', { profile: servedProfile.id, folder: workspace.toString() }));
        servedRuntime = served.runtime;
        assert.notEqual(served.runtime, runtime);
        assert.equal(served.debuggerAttached, undefined);
        assert.equal(await readOutput(served.runtime, 1), 'Review Desk · inspector ready');
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        try { assert.match(await readOutput(runtime), /Building Dispatch Desk/); }
        finally { await config.update('allowWrites', true, vscode.ConfigurationTarget.Global); }

        await vscode.workspace.fs.writeFile(failMarker, Buffer.from('controlled'));
        content(await invoke('ferroui_browser_rebuild', { runtime }));
        await until(async () => (await current())?.status === 'Browser update failed');
        assert.equal((await current())?.generation, initial.generation);
        assert.equal((await current())?.debuggerAttached, true);
        assert.ok(!terminated.has(old.id));
        const diagnostics = await readOutput(runtime);
        assert.match(diagnostics, /Controlled build failure for the debugger fixture/);
        assert.doesNotMatch(diagnostics, /Review Desk/);
        const servedOutput = await readOutput(served.runtime);
        assert.match(servedOutput, /Review Desk · Starting Chromium/);
        assert.doesNotMatch(servedOutput, /Building Dispatch Desk|Controlled build failure/);
        await vscode.commands.executeCommand('ferroui.appSessions.showOutput', runtime);
        await vscode.commands.executeCommand('ferroui.appSessions.showOutput', served.runtime);
        await vscode.workspace.fs.delete(failMarker);

        const debuggerGeneration = stopped?.debuggerGeneration;
        assert.ok(debuggerGeneration);
        const continueDebugger = (expected_debugger_generation: number) => invoke('ferroui_continue_debugger', { runtime, expected_debugger_generation });
        assert.match((await continueDebugger(debuggerGeneration - 1)).content[0]?.text ?? '', /current debugger generation/);
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        try { assert.ok((await continueDebugger(debuggerGeneration)).isError); }
        finally { await config.update('allowWrites', true, vscode.ConfigurationTarget.Global); }
        assert.equal((await current())?.debuggerPaused, true);
        const continued = content<RuntimeSummary>(await continueDebugger(debuggerGeneration));
        assert.equal(continued.runtime, initial.runtime);
        assert.equal(continued.generation, initial.generation);
        assert.equal(continued.debuggerPaused, false);
        assert.equal(continued.debuggerAttached, true);

        const line = editor.document.lineAt(2);
        assert.ok(await editor.edit(edit => edit.replace(line.range, line.text.replace('+= 1', '+= 2'))));
        assert.ok(await editor.document.save());
        const replacement = await accepted(initial.generation);
        assert.equal(replacement.runtime, initial.runtime);
        assert.equal(replacement.pid, initial.pid);
        const next = await breakpoint(1, replacement);
        assert.notEqual(next.id, old.id);
        await until(() => terminated.has(old.id));
        assert.equal(next.configuration.port, old.configuration.port);
        assert.match((await invoke('ferroui_browser_debug', { runtime, expected_generation: initial.generation }))
            .content[0]?.text ?? '', /accepted browser generation/);
        const detach = (generation: RuntimeSummary['generation']) => invoke('ferroui_browser_debug', {
            runtime, expected_generation: generation, action: 'detach',
        });
        assert.match((await detach(initial.generation)).content[0]?.text ?? '', /accepted browser generation/);
        assert.ok(!terminated.has(next.id));
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        try {
            assert.ok((await detach(replacement.generation)).isError);
            assert.ok(!terminated.has(next.id));
            assert.equal((await current())?.debuggerAttached, true);
        } finally { await config.update('allowWrites', true, vscode.ConfigurationTarget.Global); }

        const detached = content<RuntimeSummary>(await detach(replacement.generation));
        assert.equal(detached.debuggerAttached, false);
        await until(() => terminated.has(next.id));
        assert.equal((await current())?.generation, replacement.generation);
        assert.equal((await current())?.debuggerAttached, false);
        const resumed = await current();
        assert.equal(resumed?.debuggerPaused, false);
        assert.ok(!resumed?.browser?.paused);
        const live = runtimeApp(mcp, replacement);
        await live.click((await live.find('DispatchOrder')).node_id);
        assert.equal((await live.find('DispatchedCount')).text, 'Dispatched orders: 1');
        content(await invoke('ferroui_browser_rebuild', { runtime }));
        const undebugged = await accepted(replacement.generation);
        assert.equal(undebugged.runtime, replacement.runtime);
        assert.equal(undebugged.pid, replacement.pid);
        assert.equal(undebugged.debuggerAttached, false);
        await runtimeApp(mcp, undebugged).find('DispatchedCount');
        assert.equal(hits.length, 2, 'The retired debugger does not follow the replacement.');
        content(await invoke('ferroui_browser_debug', { runtime, expected_generation: undebugged.generation }));
        const reattached = await breakpoint(2, undebugged);
        assert.notEqual(reattached.id, next.id);
        await until(async () => (await current())?.debuggerAttached === true);
        await vscode.commands.executeCommand('ferroui.appSessions.detachDebug', runtime);
        await until(() => terminated.has(reattached.id));
        assert.equal((await current())?.generation, undebugged.generation);
        assert.equal((await current())?.debuggerAttached, false);
        content(await invoke('ferroui_browser_debug', { runtime, expected_generation: undebugged.generation }));
        const stoppedFromEditor = await breakpoint(3, undebugged);
        await until(async () => (await current())?.debuggerAttached === true);
        await vscode.debug.stopDebugging(stoppedFromEditor);
        await until(async () => terminated.has(stoppedFromEditor.id) && (await current())?.debuggerAttached === false);
        assert.equal((await current())?.generation, undebugged.generation);
        content(await invoke('ferroui_browser_debug', { runtime, expected_generation: undebugged.generation }));
        const finalDebugger = await breakpoint(4, undebugged);
        content(await invoke('ferroui_browser_stop', { runtime }));
        await until(() => terminated.has(finalDebugger.id));
        assert.equal(await current(), undefined);
        assert.ok((await invoke('ferroui_browser_output', { runtime })).isError);
        await assert.rejects(fetch(`http://127.0.0.1:${finalDebugger.configuration.port}/json/version`, { signal: AbortSignal.timeout(2_000) }));
        runtime = undefined;
        assert.match(await readOutput(served.runtime), /Review Desk · inspector ready/);
        assert.equal((await runtimeApp(mcp, served).find('Heading')).text, 'Dispatch desk');
        content(await invoke('ferroui_browser_stop', { runtime: served.runtime }));
        assert.ok((await invoke('ferroui_browser_output', { runtime: served.runtime })).isError);
        servedRuntime = undefined;
        await verifyBrowserAttach(url, { profile: servedProfile.id, folder: workspace.toString() }, servedProfile.label, mcp, sessions);
        console.log('Chromium verified exact debugger attachment state, VS Code Stop Debugging, debugger handoff, session output and independent cleanup');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        tracker.dispose();
        ending.dispose();
        try {
            if (runtime) await invoke('ferroui_browser_stop', { runtime });
            if (servedRuntime) await invoke('ferroui_browser_stop', { runtime: servedRuntime });
        } finally {
            await sites.close();
            await browser.update('apps', profiles, vscode.ConfigurationTarget.Global);
            await config.update('allowWrites', previous, vscode.ConfigurationTarget.Global);
            mcp.close();
        }
    }
}
