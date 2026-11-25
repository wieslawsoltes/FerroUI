import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import { fileKey } from '../src/workspacePaths';
import { trackNativeDebugger } from './nativeDebugFixture';
import { until } from './wait';

export function installedRuntimeDebug(source: vscode.TextDocument, configuration: string) {
    const line = source.getText().split('\n').findIndex(text => text.includes('std::fs::write(state, count.to_string())'));
    assert.ok(line >= 0, 'Find the Release Review callback');
    const point = new vscode.SourceBreakpoint(new vscode.Location(source.uri, new vscode.Position(line, 0)));
    const tracker = trackNativeDebugger(configuration);
    let session: vscode.DebugSession | undefined;
    vscode.debug.addBreakpoints([point]);

    const attached = () => {
        assert.ok(session && tracker.sessions.has(session.id), 'The accepted process still has its native debugger');
        return session;
    };
    return {
        async accept(pid: number) {
            const previous = session;
            const replacement = await tracker.accept(previous?.id);
            assert.equal(replacement.configuration.pid, pid, 'LLDB follows the accepted app PID');
            if (previous) await until(() => !tracker.sessions.has(previous.id));
            session = replacement;
        },
        attached,
        async click(action: () => Promise<unknown>) {
            const current = attached();
            const input = action().then(() => undefined, (error: unknown) => error);
            const threadId = await tracker.breakpoint(current);
            try {
                const { stackFrames: [frame] } = await current.customRequest('stackTrace', { threadId });
                assert.match(frame.name, /\bsave_review\b/);
                assert.equal(fileKey(await realpath(frame.source.path)), fileKey(await realpath(source.uri.fsPath)));
                assert.equal(frame.line, line + 1);
            } finally { await current.customRequest('continue', { threadId }); }
            const error = await input;
            if (error !== undefined) throw error;
        },
        async detach() {
            const current = attached();
            await vscode.debug.stopDebugging(current);
            await until(() => !tracker.sessions.has(current.id));
            return tracker.disconnectStatus(current);
        },
        async close() {
            vscode.debug.removeBreakpoints([point]);
            await tracker.close();
        },
    };
}
