import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { fileKey } from '../src/workspacePaths';
import { waitFor } from './browserTestSupport';
import { openEditorMcp } from './mcpFixture';
import { trackNativeDebugger } from './nativeDebugFixture';

const apps = {
    android: {
        platform: 'Android', source: 'lib.rs', profile: 'service-review', debugger: 'Attach to Service Review', command: 'ferroui.android.debug',
        statement: '*count += 1;', callback: 'complete_review', button: 'ReviewCase',
        heading: 'Service review', reviewed: 'Reviewed cases: 1',
    },
    ios_simulator: {
        platform: 'iOS simulator', source: 'main.rs', profile: 'contact-review', debugger: 'Attach to Contact Review', command: 'ferroui.iosSimulator.debug',
        statement: '*reviewed += 1;', callback: 'review_next_contact', button: 'ReviewContact',
        heading: 'Contact review', reviewed: '1 contacts reviewed',
    },
} as const;

export async function verifyNativeMobileDevelopment(platform: keyof typeof apps, device: string): Promise<void> {
    const fixture = apps[platform];
    const tool = (action: 'run' | 'debug' | 'output' | 'stop') => `ferroui_${platform}_${action}`;
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder, fixture.source));
    const xaml = vscode.Uri.joinPath(folder, 'Message.xaml');
    const mark = rust.getText().indexOf(fixture.statement);
    assert.ok(mark >= 0, `Find the ${fixture.callback} callback.`);
    const point = new vscode.SourceBreakpoint(new vscode.Location(rust.uri, rust.positionAt(mark)));
    const tracker = trackNativeDebugger(fixture.debugger);
    const { content, invoke, close } = await openEditorMcp(40_000);
    let runtime: string | undefined;
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    const current = async () => (await sessions()).find(item => item.runtime === runtime);
    const output = async () => content<{ output: string }>(await invoke(tool('output'), { runtime })).output;
    const accept = (prior?: number) => waitFor(async () => {
        const session = await current();
        if (session?.status === `${fixture.platform} build failed` || session?.status === `${fixture.platform} deployment failed`) throw Error(await output());
        if (session?.available && session.generation && session.generation !== prior) return session;
    }, `a source-built ${fixture.platform} frame`, 15 * 60_000);
    const identity = (session: RuntimeSummary) => ({ runtime, expected_generation: session.generation });
    const find = async (session: RuntimeSummary, match: { name: string } | { text: string }) =>
        content<{ result: RuntimeNode[] }>(await invoke('ferroui_runtime_query', {
            ...identity(session), tool: 'find_elements', arguments: match,
        })).result[0];
    const save = async (uri: vscode.Uri, before: string, after: string) => {
        const document = await vscode.workspace.openTextDocument(uri);
        const offset = document.getText().indexOf(before);
        assert.ok(offset >= 0);
        const edit = new vscode.WorkspaceEdit();
        edit.replace(document.uri, new vscode.Range(document.positionAt(offset), document.positionAt(offset + before.length)), after);
        assert.ok(await vscode.workspace.applyEdit(edit) && await document.save());
    };
    const acceptDebugger = async (app: RuntimeSummary, prior?: vscode.DebugSession) => {
        const debuggerSession = await tracker.accept(prior?.id);
        if (prior) await waitFor(() => !tracker.sessions.has(prior.id) ? true : undefined, 'the retired debugger to detach');
        if (app.simulator) assert.equal(debuggerSession.configuration.pid, app.pid);
        return debuggerSession;
    };
    const callback = async (app: RuntimeSummary, debug: vscode.DebugSession) => {
        const button = await find(app, { name: fixture.button });
        assert.ok(button);
        const click = invoke('ferroui_runtime_call', { ...identity(app), tool: 'click', arguments: { node_id: button.node_id } })
            .then(response => { content(response); }).catch((error: unknown) => error);
        const threadId = await tracker.breakpoint(debug);
        try {
            const { stackFrames: [frame] } = await debug.customRequest('stackTrace', { threadId, startFrame: 0, levels: 1 });
            assert.ok(frame.name.includes(fixture.callback), frame.name);
            assert.equal(fileKey(frame.source.path), fileKey(rust.uri.fsPath));
            assert.equal(frame.line, point.location.range.start.line + 1);
        } finally { await debug.customRequest('continue', { threadId }); }
        assert.equal(await click, undefined);
        await waitFor(async () => (await find(app, { name: 'ReviewedCount' }))?.text === fixture.reviewed ? true : undefined,
            'the native callback to finish');
    };
    try {
        await vscode.workspace.getConfiguration('ferroui.mcp').update('allowWrites', true, vscode.ConfigurationTarget.Global);
        await vscode.window.showTextDocument(rust);
        const request = { device, profile: fixture.profile, folder: folder.toString() };
        const runDebug = { ...request, debug: true, debug_configuration: fixture.debugger };
        const invalid = await invoke(tool('run'), { ...runDebug, debug_configuration: 'Missing attach configuration' });
        assert.ok(invalid.isError);
        assert.match(invalid.content[0]?.text ?? '', /Choose one attach configuration/);
        assert.deepEqual(await sessions(), [], 'An invalid debugger selection must leave the app stopped.');
        vscode.debug.addBreakpoints([point]);
        const pending = content<RuntimeSummary>(await invoke(tool('run'), runDebug));
        runtime = pending.runtime;
        assert.ok((pending.android ?? pending.simulator)?.development && !pending.available);
        assert.equal(content<RuntimeSummary>(await invoke(tool('run'), runDebug)).runtime, runtime);
        content(await invoke('ferroui_runtime_open', { runtime }));
        const first = await accept();
        assert.ok(first.pid && first.generation);
        if (first.android) assert.ok(first.android.debuggable);
        const firstDebugger = await acceptDebugger(first);
        assert.ok(await find(first, { text: 'XAML inbox' }));
        await callback(first, firstDebugger);
        content(await invoke(tool('debug'), { ...identity(first), action: 'detach' }));
        await waitFor(() => !tracker.sessions.has(firstDebugger.id) ? true : undefined, 'the first debugger to detach');
        await vscode.commands.executeCommand(fixture.command, request);
        const workbenchDebugger = await acceptDebugger(first, firstDebugger);
        assert.equal((await current())?.generation, first.generation);

        const updatedHeading = `${fixture.heading} updated`;
        await save(rust.uri, `text: "${fixture.heading}"`, `text: "${updatedHeading}"`);
        const second = await accept(first.generation);
        assert.notEqual(second.pid, first.pid);
        assert.ok((await invoke('ferroui_runtime_query', { ...identity(first), tool: 'list_windows' })).isError);
        const secondDebugger = await acceptDebugger(second, workbenchDebugger);
        assert.ok(await find(second, { text: updatedHeading }));
        await callback(second, secondDebugger);

        await save(xaml, 'XAML inbox', 'XAML inbox updated');
        const third = await accept(second.generation ?? undefined);
        assert.notEqual(third.pid, second.pid);
        const thirdDebugger = await acceptDebugger(third, secondDebugger);
        assert.ok(await find(third, { text: 'XAML inbox updated' }));
        await callback(third, thirdDebugger);

        await save(rust.uri, fixture.statement, 'compile_error!("managed mobile source fixture");');
        await waitFor(async () => (await current())?.status === `${fixture.platform} build failed` ? true : undefined, 'a rejected Rust save', 120_000);
        assert.equal((await current())?.generation, third.generation);
        assert.ok(tracker.sessions.has(thirdDebugger.id));
        assert.match(await output(), /managed mobile source fixture/);
        content(await invoke(tool('debug'), { ...identity(third), action: 'detach' }));
        assert.equal((await current())?.pid, third.pid);
        assert.ok(await find(third, { text: 'XAML inbox updated' }));
        content(await invoke(tool('stop'), { runtime }));
        assert.equal(await current(), undefined);
        runtime = undefined;
        console.log(`Managed ${fixture.platform} Run and Debug via MCP and workbench, native Rust and XAML saves, debugger handoff, failed build retention, and detach passed`);
    } finally {
        if (runtime) await invoke(tool('stop'), { runtime }).catch(() => {});
        vscode.debug.removeBreakpoints([point]);
        await tracker.close();
        close();
    }
}
