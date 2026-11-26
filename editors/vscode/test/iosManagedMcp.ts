import { strict as assert } from 'node:assert';
import { existsSync, readFileSync, readdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { fileKey } from '../src/workspacePaths';
import { waitFor } from './browserTestSupport';
import { openEditorMcp } from './mcpFixture';
import { trackNativeDebugger } from './nativeDebugFixture';

export async function verifyManagedSimulator(device: string): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0];
    const stage = process.env.FERROUI_IOS_MANAGED_TEST!;
    const packagerHold = join(stage, 'hold');
    const builds = join(stage, 'builds');
    const readBuilds = () => existsSync(builds) ? readFileSync(builds, 'utf8') : '';
    const artifacts = () => readdirSync(join(folder.uri.fsPath, 'target')).filter(name => name.startsWith('.ferroui-simulator-'));
    const mcp = await openEditorMcp(28_000);
    const { content, invoke } = mcp;
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const writePermission = (enabled: boolean) => config.update('allowWrites', enabled, vscode.ConfigurationTarget.Global);
    const breakpoint = process.env.FERROUI_TEST_IOS_DEBUG ? await reviewBreakpoint() : undefined;
    const tracker = breakpoint && trackNativeDebugger('Attach to Contact Review');
    let runtime: string | undefined;
    const read = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes
        .find(session => session.runtime === runtime);
    const status = (label: string) => waitFor(async () => {
        const session = await read();
        return session?.status === label ? session : undefined;
    }, label);
    const accepted = (previous?: number | null) => waitFor(async () => {
        const session = await read();
        if (session?.available && session.generation !== previous) return session;
        if (session?.status.endsWith(' failed')) {
            throw Error(content<{ output: string }>(await invoke('ferroui_ios_simulator_output', { runtime })).output);
        }
    }, 'a verified native simulator frame', 45_000);
    const save = async (file: string, next: string) => {
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder.uri, file));
        const edit = new vscode.WorkspaceEdit();
        edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), next);
        assert.ok(await vscode.workspace.applyEdit(edit) && await doc.save());
    };
    const rust = 'app/main.rs';
    try {
        const request = { profile: 'managed', device, folder: folder.uri.toString() };
        assert.match((await invoke('ferroui_ios_simulator_run', request)).content[0]?.text ?? '', /allowWrites/);
        await writePermission(true);
        const pending = content<RuntimeSummary>(await invoke('ferroui_ios_simulator_run', request));
        runtime = pending.runtime;
        assert.ok(pending.simulator?.development);
        assert.equal(pending.available, false);
        content(await invoke('ferroui_runtime_open', { runtime }));
        const first = await accepted();
        assert.ok(first.pid && first.generation);
        let debuggerSession: vscode.DebugSession | undefined;
        if (tracker && breakpoint) {
            vscode.debug.addBreakpoints([breakpoint]);
            content(await invoke('ferroui_ios_simulator_debug', { runtime, expected_generation: first.generation }));
            debuggerSession = await tracker.accept();
            assert.equal(debuggerSession.configuration.pid, first.pid);
        }
        const initialBuilds = readBuilds();
        await save('notes/lib.rs', '// unrelated Cargo package\n');
        await delay(550);
        assert.equal(readBuilds(), initialBuilds);
        await save(rust, '// fail-build\n');
        await status('iOS simulator build failed');
        assert.equal((await read())?.generation, first.generation);
        if (debuggerSession) assert.ok(tracker?.sessions.has(debuggerSession.id));
        assert.match(content<{ output: string }>(await invoke('ferroui_ios_simulator_output', { runtime })).output, /controlled simulator compiler error/);

        writeFileSync(packagerHold, '');
        await save(rust, '// superseded candidate\n');
        await waitFor(() => readBuilds().includes('superseded candidate') ? true : undefined, 'the held simulator build');
        assert.equal((await read())?.pid, first.pid);
        await save(rust, '// newest candidate\n');
        unlinkSync(packagerHold);
        const second = await accepted(first.generation);
        assert.notEqual(second.pid, first.pid);
        assert.equal(second.runtime, runtime);
        await waitFor(() => artifacts().length === 1 ? true : undefined, 'cleanup of the superseded simulator bundles');
        if (tracker && debuggerSession && breakpoint) {
            const next = await tracker.accept(debuggerSession.id);
            assert.equal(next.configuration.pid, second.pid);
            const identity = { runtime, expected_generation: second.generation };
            const { result: [button] } = content<{ result: RuntimeNode[] }>(await invoke('ferroui_runtime_query', {
                ...identity, tool: 'find_elements', arguments: { name: 'ReviewContact' },
            }));
            const click = invoke('ferroui_runtime_call', { ...identity, tool: 'click', arguments: { node_id: button.node_id } });
            const threadId = await tracker.breakpoint(next);
            try {
                const { stackFrames: [frame] } = await next.customRequest('stackTrace', { threadId, startFrame: 0, levels: 1 });
                assert.match(frame.name, /review_next_contact/);
                assert.equal(fileKey(frame.source.path), fileKey(breakpoint.location.uri.fsPath));
            } finally { await next.customRequest('continue', { threadId }); }
            content(await click);
            content(await invoke('ferroui_ios_simulator_debug', { ...identity, action: 'detach' }));
            assert.equal((await read())?.pid, second.pid);
        }

        await save('app/View.xaml', '<TextBlock Text="Saved XAML"/>\n');
        const third = await accepted(second.generation);
        assert.notEqual(third.pid, second.pid);
        assert.ok((await invoke('ferroui_runtime_query', { runtime, expected_generation: second.generation, tool: 'list_windows' })).isError);
        const { records } = content<{ records: { file: string; outcome: string; frame?: { status: string } }[] }>(
            await invoke('ferroui_reload_history', { app_session: runtime }));
        assert.ok(records.some(record => record.file.endsWith('View.xaml') && record.outcome === 'rebuilt' && record.frame?.status === 'rendered'));
        await save(rust, '// fail-startup\n');
        const restored = await status('iOS simulator deployment failed · restored previous build');
        assert.ok(restored.available && restored.generation !== third.generation && restored.pid !== third.pid);
        assert.equal(restored.runtime, runtime);

        await save(rust, '// repaired source\n');
        const recovered = await accepted(restored.generation);
        await writePermission(false);
        assert.match((await invoke('ferroui_ios_simulator_rebuild', { runtime })).content[0]?.text ?? '', /allowWrites/);
        assert.match(content<{ output: string }>(await invoke('ferroui_ios_simulator_output', { runtime })).output, /staged simulator bundle/);
        await writePermission(true);
        writeFileSync(packagerHold, '');
        content(await invoke('ferroui_ios_simulator_rebuild', { runtime }));
        await status('Building iOS simulator update');
        assert.equal((await read())?.pid, recovered.pid);
        content(await invoke('ferroui_ios_simulator_stop', { runtime }));
        assert.equal(await read(), undefined);
        runtime = undefined;
        assert.deepEqual(artifacts(), []);
        console.log('Managed iOS simulator native deployment, scoped saves, rollback, debugger handoff and cleanup passed');
    } finally {
        try {
            if (runtime) {
                await writePermission(true);
                await invoke('ferroui_ios_simulator_stop', { runtime });
            }
        } finally {
            if (breakpoint) vscode.debug.removeBreakpoints([breakpoint]);
            await tracker?.close();
            mcp.close();
        }
    }
}

async function reviewBreakpoint(): Promise<vscode.SourceBreakpoint> {
    const repo = process.env.FERROUI_TEST_IOS_SIMULATOR_SOURCE!;
    const uri = vscode.Uri.file(join(repo, 'src/iOS/FerroUI.iOS/examples/simulator_inspector.rs'));
    const document = await vscode.workspace.openTextDocument(uri);
    const offset = document.getText().indexOf('*reviewed += 1');
    assert.ok(offset >= 0, 'Find the Contact Review callback.');
    return new vscode.SourceBreakpoint(new vscode.Location(uri, document.positionAt(offset)));
}
