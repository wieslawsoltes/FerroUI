import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import { trackNativeDebugger } from './nativeDebugFixture';
import { until } from './wait';

export function installedRuntimeDebug(source: vscode.TextDocument, configuration: string) {
    const line = source.getText().split('\n').findIndex(text => text.includes('std::fs::write(state, count.to_string())'));
    assert.ok(line >= 0, 'Find the Release Review callback');
    const point = new vscode.SourceBreakpoint(new vscode.Location(source.uri, new vscode.Position(line, 0)));
    const tracker = trackNativeDebugger(configuration);
    let session: vscode.DebugSession | undefined;
    let inspectedApp: RuntimeSummary | undefined;
    vscode.debug.addBreakpoints([point]);

    const attached = () => {
        assert.ok(session && tracker.sessions.has(session.id), 'The accepted process still has its native debugger');
        return session;
    };
    const accept = async (pid: number) => {
        const previous = session;
        const replacement = await tracker.accept(previous?.id);
        assert.equal(replacement.configuration.pid, pid, 'LLDB follows the accepted app PID');
        if (process.platform === 'win32') {
            assert.equal(replacement.configuration.debugAdapterEnv?.LLDB_USE_LLDB_SERVER, '0',
                'The Windows desktop attach uses the local LLDB process backend');
        }
        if (previous) await until(() => !tracker.sessions.has(previous.id));
        session = replacement;
    };
    const runtimeFor = (current: vscode.DebugSession) => {
        assert.ok(inspectedApp && inspectedApp.pid === current.configuration.pid, 'Use the accepted App Sessions target');
        return inspectedApp.runtime;
    };
    const attach = (current: vscode.DebugSession) => vscode.commands.executeCommand('ferroui.appSessions.debug', runtimeFor(current));
    return {
        accept,
        attached,
        async repeatAttach() {
            const current = attached();
            await attach(current);
            assert.equal(attached().id, current.id, 'Repeated attach keeps the existing LLDB session');
        },
        async reattach() {
            const current = session;
            assert.ok(current && !tracker.sessions.has(current.id), 'Reattach after the old debugger has detached');
            await attach(current);
            await accept(current.configuration.pid);
        },
        async click(action: () => Promise<unknown>) {
            const current = attached();
            const input = action().then(() => undefined, (error: unknown) => error);
            const threadId = await tracker.breakpoint(current);
            try {
                const { stackFrames: [frame] } = await current.customRequest('stackTrace', { threadId });
                assert.match(frame.name, /\bsave_review\b/);
                assert.equal(fileKey(await realpath(frame.source.path)), fileKey(await realpath(source.uri.fsPath)));
                assert.equal(frame.line, line + 1);
                const resumed = await vscode.commands.executeCommand<RuntimeSummary>('ferroui.appSessions.continueDebug');
                assert.ok(resumed, 'The FerroUI command continued the paused application');
                assert.equal(resumed.pid, current.configuration.pid);
                assert.equal(resumed.debuggerPaused, false);
                if (inspectedApp && inspectedApp.pid === resumed.pid) {
                    assert.equal(resumed.generation, inspectedApp.generation, 'Debugger reattach keeps the inspector generation');
                    assert.equal(resumed.debuggerGeneration, inspectedApp.debuggerGeneration, 'Debugger reattach keeps the process generation');
                } else if (inspectedApp) {
                    await vscode.commands.executeCommand('ferroui.appSessions.detachDebug', { session: inspectedApp });
                    assert.equal(attached().id, current.id, 'A retired App Sessions row leaves the replacement debugger attached');
                }
                inspectedApp = resumed;
            } catch (error) {
                await current.customRequest('continue', { threadId }).then(undefined, () => {});
                throw error;
            }
            const error = await input;
            if (error !== undefined) throw error;
        },
        async detach() {
            const current = attached();
            await vscode.commands.executeCommand('ferroui.appSessions.detachDebug', runtimeFor(current));
            await until(() => !tracker.sessions.has(current.id)).catch(error => {
                throw new Error(`LLDB did not detach from PID ${current.configuration.pid}. ${JSON.stringify(tracker.disconnectStatus(current))}`, { cause: error });
            });
            const status = tracker.disconnectStatus(current);
            assert.equal(status.disconnect?.terminateDebuggee, false);
            return status;
        },
        async close() {
            vscode.debug.removeBreakpoints([point]);
            await tracker.close();
        },
    };
}
