import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import type { McpFixture } from './mcpFixture';
import { trackNativeDebugger, waitForDebuggerAttachment } from './nativeDebugFixture';
import { until } from './wait';

export async function verifyIosSimulatorDebug(mcp: McpFixture, app: RuntimeSummary, document: vscode.TextDocument, button: number) {
    assert.ok(vscode.extensions.getExtension('llvm-vs-code-extensions.lldb-dap'), 'Install LLVM LLDB DAP in the test extensions directory.');
    const { runtime, generation } = app;
    const callback = '*reviewed += 1';
    const offset = document.getText().indexOf(callback);
    assert.ok(offset >= 0, 'Find the Contact Review callback.');
    const position = document.positionAt(offset);
    const point = new vscode.SourceBreakpoint(new vscode.Location(document.uri, position));
    const tracker = trackNativeDebugger('Attach to Contact Review');
    const acceptSession = async (pid: number, previous?: string) => {
        const session = await tracker.accept(previous);
        assert.equal(session.configuration.pid, pid, 'LLDB attaches to the process verified by FerroUI.');
        return session;
    };
    const readCount = async () => mcp.content<{ result: { text: string }[] }>(await mcp.invoke('ferroui_runtime_query', {
        runtime, expected_generation: generation, tool: 'find_elements', arguments: { name: 'ReviewedCount' },
    })).result[0]?.text;
    try {
        assert.equal(app.debuggerAttached, false);
        vscode.debug.addBreakpoints([point]);
        const debug = mcp.content<{ pid: number; generation: number; attached: boolean }>(await mcp.invoke('ferroui_ios_simulator_debug', {
            runtime, expected_generation: generation, debug_configuration: 'Attach to Contact Review',
        }));
        assert.ok(debug.attached && debug.pid > 0);
        assert.equal(debug.generation, generation);
        const session = await acceptSession(debug.pid);
        await waitForDebuggerAttachment(mcp, app, true);
        const click = mcp.invoke('ferroui_runtime_call', { runtime, expected_generation: generation,
            tool: 'click', arguments: { node_id: button } }).then(response => { mcp.content(response); }).catch((error: unknown) => error);
        const threadId = await tracker.breakpoint(session);
        try {
            const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId });
            assert.match(frame.name, /review_next_contact/);
            assert.equal(fileKey(frame.source.path), fileKey(document.uri.fsPath));
            assert.equal(frame.line, position.line + 1);
        } finally { await session.customRequest('continue', { threadId }); }
        assert.equal(await click, undefined);
        await until(async () => await readCount() === '2 contacts reviewed');
        await vscode.debug.stopDebugging(session);
        await until(() => !tracker.sessions.has(session.id));
        await waitForDebuggerAttachment(mcp, app, false);
        assert.equal(await readCount(), '2 contacts reviewed', 'Detaching preserves the original inspector generation.');

        await vscode.commands.executeCommand('ferroui.appSessions.debug', runtime);
        const reopened = await acceptSession(debug.pid, session.id);
        await waitForDebuggerAttachment(mcp, app, true);
        await vscode.debug.stopDebugging(reopened);
        await until(() => !tracker.sessions.has(reopened.id));
        await waitForDebuggerAttachment(mcp, app, false);
        assert.ok(app.simulator);
        const requested = mcp.content<RuntimeSummary>(await mcp.invoke('ferroui_ios_simulator_run', {
            device: app.simulator.device, profile: app.simulator.profile, debug: true,
        }));
        assert.equal(requested.runtime, runtime);
        assert.equal(requested.generation, generation);
        await acceptSession(debug.pid, reopened.id);
        await waitForDebuggerAttachment(mcp, app, true);
        console.log('iOS simulator Rust breakpoint, exact debugger attachment state, and same-process VS Code and MCP Run and Debug passed');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        await tracker.close();
    }
}
