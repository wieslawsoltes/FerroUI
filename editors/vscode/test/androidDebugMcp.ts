import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { fileKey } from '../src/workspacePaths';
import type { McpFixture } from './mcpFixture';
import { trackNativeDebugger, waitForDebuggerAttachment } from './nativeDebugFixture';
import { until } from './wait';

export async function verifyAndroidDebug(mcp: McpFixture, app: RuntimeSummary, document: vscode.TextDocument,
    forwards: () => Promise<string[]>) {
    assert.ok(vscode.extensions.getExtension('llvm-vs-code-extensions.lldb-dap'), 'Install LLVM LLDB DAP in the test extensions directory.');
    const { runtime, generation, pid } = app;
    assert.ok(generation && pid);
    assert.ok(app.android?.debuggable, 'Package Service Review with the NDK debugger.');
    const request = { runtime, expected_generation: generation };
    const name = 'Attach to Service Review';
    const offset = document.getText().indexOf('*count += 1');
    assert.ok(offset >= 0, 'Find the Service Review callback.');
    const position = document.positionAt(offset);
    const point = new vscode.SourceBreakpoint(new vscode.Location(document.uri, position));
    const baseline = await forwards();
    const tracker = trackNativeDebugger(name);
    const query = async (control: string) => mcp.content<{ result: RuntimeNode[] }>(await mcp.invoke('ferroui_runtime_query', {
        ...request, tool: 'find_elements', arguments: { name: control },
    })).result[0];
    const verifyForward = async (session: vscode.DebugSession) => {
        const port = session.configuration['gdb-remote-port'];
        assert.ok(Number.isInteger(port) && port > 0);
        const active = await forwards();
        assert.equal(active.length, baseline.length + 1);
        assert.ok(active.some(line => line.includes(`tcp:${port} tcp:`)), 'The accepted debugger owns one independent ADB forward.');
    };
    try {
        assert.equal(app.debuggerAttached, false);
        vscode.debug.addBreakpoints([point]);
        const debug = mcp.content<{ pid: number; generation: number; attached: boolean }>(await mcp.invoke('ferroui_android_debug', {
            ...request, debug_configuration: name,
        }));
        assert.deepEqual({ pid: debug.pid, generation: debug.generation, attached: debug.attached }, { pid, generation, attached: true });
        const session = await tracker.accept();
        await waitForDebuggerAttachment(mcp, app, true);
        await verifyForward(session);
        const button = await query('ReviewCase');
        const click = mcp.invoke('ferroui_runtime_call', { ...request, tool: 'click', arguments: { node_id: button.node_id } })
            .then(response => { mcp.content(response); }).catch((error: unknown) => error);
        const threadId = await tracker.breakpoint(session);
        try {
            const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId });
            assert.match(frame.name, /complete_review/);
            assert.equal(fileKey(frame.source.path), fileKey(document.uri.fsPath));
            assert.equal(frame.line, position.line + 1);
        } finally { await session.customRequest('continue', { threadId }); }
        assert.equal(await click, undefined);
        await until(async () => (await query('ReviewedCount')).text === 'Reviewed cases: 2');

        const detached = mcp.content<{ attached: boolean }>(await mcp.invoke('ferroui_android_debug', { ...request, action: 'detach' }));
        assert.equal(detached.attached, false);
        await waitForDebuggerAttachment(mcp, app, false);
        assert.deepEqual(await forwards(), baseline);
        assert.equal((await query('ReviewedCount')).text, 'Reviewed cases: 2');
        await vscode.commands.executeCommand('ferroui.appSessions.debug', runtime);
        const reopened = await tracker.accept(session.id);
        await waitForDebuggerAttachment(mcp, app, true);
        await verifyForward(reopened);
        await vscode.debug.stopDebugging(reopened);
        await until(async () => (await forwards()).length === baseline.length);
        await waitForDebuggerAttachment(mcp, app, false);
        assert.deepEqual(await forwards(), baseline);
        assert.equal((await query('ReviewedCount')).text, 'Reviewed cases: 2');
        const requested = mcp.content<RuntimeSummary>(await mcp.invoke('ferroui_android_run', {
            device: app.android.device, profile: app.android.profile, debug: true,
        }));
        assert.equal(requested.runtime, runtime);
        assert.equal(requested.generation, generation);
        const fromRun = await tracker.accept(reopened.id);
        await waitForDebuggerAttachment(mcp, app, true);
        await verifyForward(fromRun);
        await vscode.debug.stopDebugging(fromRun);
        await until(async () => (await forwards()).length === baseline.length);
        await waitForDebuggerAttachment(mcp, app, false);
        assert.deepEqual(await forwards(), baseline);
        console.log('Android native Rust breakpoint, exact debugger attachment state, same-process MCP and VS Code Run and Debug, and forwarding cleanup passed');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        await tracker.close();
    }
}
