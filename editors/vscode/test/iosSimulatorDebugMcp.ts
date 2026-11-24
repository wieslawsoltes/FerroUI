import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import type { McpFixture } from './mcpFixture';
import { until } from './wait';

export async function verifyIosSimulatorDebug(mcp: McpFixture, app: RuntimeSummary, document: vscode.TextDocument, button: number) {
    assert.ok(vscode.extensions.getExtension('llvm-vs-code-extensions.lldb-dap'), 'Install LLVM LLDB DAP in the test extensions directory.');
    const { runtime, generation } = app;
    const callback = '*reviewed += 1';
    const offset = document.getText().indexOf(callback);
    assert.ok(offset >= 0, 'Find the Contact Review callback.');
    const position = document.positionAt(offset);
    const point = new vscode.SourceBreakpoint(new vscode.Location(document.uri, position));
    const sessions = new Map<string, vscode.DebugSession>();
    const started = new Set<string>();
    const verified = new Set<string>();
    const hits = new Map<string, number>();
    const terminated = vscode.debug.onDidTerminateDebugSession(session => { sessions.delete(session.id); });
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('lldb-dap', {
        createDebugAdapterTracker(session) {
            if (!session.configuration.ferrouiManagedInstance || session.configuration.name !== 'Attach to Contact Review') return;
            sessions.set(session.id, session);
            return { onDidSendMessage(message) {
                if (message.type === 'response' && message.command === 'attach' && message.success) started.add(session.id);
                if ((message.command === 'setBreakpoints' && message.body?.breakpoints?.some((point: { verified: boolean }) => point.verified))
                    || (message.event === 'breakpoint' && message.body?.breakpoint?.verified)) verified.add(session.id);
                if (message.event === 'stopped' && message.body?.reason === 'breakpoint') hits.set(session.id, message.body.threadId);
            } };
        },
    });
    const acceptSession = async (pid: number, previous?: string) => {
        const find = () => [...sessions.values()].find(session => session.id !== previous && started.has(session.id));
        await until(() => Boolean(find()));
        const session = find()!;
        assert.equal(session.configuration.pid, pid, 'LLDB attaches to the process verified by FerroUI.');
        await until(() => verified.has(session.id));
        return session;
    };
    const readCount = async () => mcp.content<{ result: { text: string }[] }>(await mcp.invoke('ferroui_runtime_query', {
        runtime, expected_generation: generation, tool: 'find_elements', arguments: { name: 'ReviewedCount' },
    })).result[0]?.text;
    try {
        vscode.debug.addBreakpoints([point]);
        const debug = mcp.content<{ pid: number; generation: number; attached: boolean }>(await mcp.invoke('ferroui_ios_simulator_debug', {
            runtime, expected_generation: generation, debug_configuration: 'Attach to Contact Review',
        }));
        assert.ok(debug.attached && debug.pid > 0);
        assert.equal(debug.generation, generation);
        const session = await acceptSession(debug.pid);
        const click = mcp.invoke('ferroui_runtime_call', { runtime, expected_generation: generation,
            tool: 'click', arguments: { node_id: button } }).then(response => { mcp.content(response); }).catch((error: unknown) => error);
        await until(() => hits.has(session.id));
        const threadId = hits.get(session.id)!;
        try {
            const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId });
            assert.match(frame.name, /review_next_contact/);
            assert.equal(fileKey(frame.source.path), fileKey(document.uri.fsPath));
            assert.equal(frame.line, position.line + 1);
        } finally { await session.customRequest('continue', { threadId }); }
        assert.equal(await click, undefined);
        await until(async () => await readCount() === '2 contacts reviewed');
        await vscode.debug.stopDebugging(session);
        await until(() => !sessions.has(session.id));
        assert.equal(await readCount(), '2 contacts reviewed', 'Detaching preserves the original inspector generation.');

        await vscode.commands.executeCommand('ferroui.appSessions.debug', runtime);
        await acceptSession(debug.pid, session.id);
        console.log('iOS simulator Rust breakpoint, debugger detach, and same-process VS Code reattach passed');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        await Promise.allSettled([...sessions.values()].map(session => vscode.debug.stopDebugging(session)));
        tracker.dispose();
        terminated.dispose();
    }
}
