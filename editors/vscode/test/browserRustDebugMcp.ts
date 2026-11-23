import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import { runtimeApp } from './browserMcp';
import { browserBuildTimeout, waitFor } from './browserTestSupport';
import { openEditorMcp } from './mcpFixture';

interface BreakpointReport {
    verified: boolean;
    line?: number;
    source?: { path?: string };
}

function trackRustBreakpoint(location: vscode.Location) {
    const bound = new Map<string, vscode.DebugSession>();
    const hits = new Map<string, { session: vscode.DebugSession; threadId: number }>();
    const file = fileKey(location.uri.fsPath);
    const line = location.range.start.line + 1;
    const matches = (breakpoint?: BreakpointReport) => breakpoint?.verified
        && (breakpoint.line === undefined || breakpoint.line === line)
        && (!breakpoint.source?.path || fileKey(breakpoint.source.path) === file);
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('*', {
        createDebugAdapterTracker(session) {
            const root = session.parentSession ?? session;
            const url = root.configuration.url;
            if (!root.configuration.ferrouiManagedInstance || typeof url !== 'string') return;
            return { onDidSendMessage(message) {
                if (message.event === 'stopped' && message.body?.reason === 'breakpoint') {
                    hits.set(url, { session, threadId: message.body.threadId });
                }
                if ((message.event === 'breakpoint' && matches(message.body?.breakpoint))
                    || (message.command === 'setBreakpoints' && message.body?.breakpoints?.some(matches))) {
                    bound.set(url, root);
                }
            } };
        },
    });
    return { bound, hits, tracker };
}

function sourceLine(document: vscode.TextDocument, text: string): vscode.TextLine {
    const offset = document.getText().indexOf(text);
    assert.ok(offset >= 0, `Find ${text} in the browser fixture.`);
    return document.lineAt(document.positionAt(offset).line);
}

export async function verifyBrowserRustDebug(): Promise<void> {
    assert.ok(vscode.extensions.getExtension('ms-vscode.wasm-dwarf-debugging'), 'Install WebAssembly DWARF Debugging in the test extensions directory.');
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const editor = await vscode.window.showTextDocument(vscode.Uri.joinPath(workspace, 'main.rs'));
    const source = editor.document;
    const operation = sourceLine(source, '*count = count.saturating_add(1);');
    const position = new vscode.Position(operation.lineNumber, operation.firstNonWhitespaceCharacterIndex);
    const point = new vscode.SourceBreakpoint(new vscode.Location(source.uri, position));
    const mcp = await openEditorMcp(38_000);
    const { invoke, content } = mcp;
    const settings = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = settings.inspect<boolean>('allowWrites')?.globalValue;
    const { bound, hits, tracker } = trackRustBreakpoint(point.location);
    let runtime: string | undefined;
    const ready = (prior?: RuntimeSummary) => waitFor(async () => {
        const sessions = content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
        const current = sessions.find(session => session.runtime === runtime);
        if (current?.status === 'Browser launch failed' || current?.status === 'Browser update failed') {
            throw new Error(content<{ output: string }>(await invoke('ferroui_browser_output', { runtime, lines: 80 })).output);
        }
        if (current?.available && current.generation !== prior?.generation) return current;
    }, 'the accepted Chromium page', prior ? browserBuildTimeout() : Math.max(360_000, browserBuildTimeout()));

    const dispatch = async (page: RuntimeSummary, count: number) => {
        const url = page.browser?.url;
        assert.ok(url);
        const debug = await waitFor(() => bound.get(url), 'a verified breakpoint on the accepted Chromium page', 90_000);
        const app = runtimeApp(mcp, page);
        const button = await app.find('DispatchOrder');
        assert.ok(button);
        const click = app.click(button.node_id).then(() => undefined, (error: unknown) => error);
        const { session, threadId } = await waitFor(() => hits.get(url), 'the dispatch callback breakpoint');
        try {
            const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId });
            assert.match(frame.name, /mark_dispatched/);
            assert.equal(fileKey(frame.source.path), fileKey(source.uri.fsPath));
            assert.equal(frame.line, operation.lineNumber + 1);
        } finally {
            await session.customRequest('continue', { threadId });
        }
        assert.equal(await click, undefined);
        const result = await app.result<{ matched: boolean }>('wait_for', { name: 'DispatchedCount', text: `Dispatched orders: ${count}` });
        assert.ok(result.matched, 'Continuing the debugger completes the app action.');
        return debug;
    };

    try {
        vscode.debug.addBreakpoints([point]);
        await settings.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const started = content<RuntimeSummary>(await invoke('ferroui_browser_run', {
            profile: 'dispatch-check', folder: workspace.toString(), debug: true,
        }));
        runtime = started.runtime;
        assert.equal(started.browser?.debuggable, true);
        content(await invoke('ferroui_runtime_open', { runtime }));
        const first = await ready();
        const originalDebug = await dispatch(first, 1);

        const heading = sourceLine(source, 'text: "Dispatch desk"');
        assert.ok(await editor.edit(edit => edit.replace(heading.range, heading.text.replace('Dispatch desk', 'Routes ready'))));
        assert.ok(await source.save());
        const replacement = await ready(first);
        assert.equal(replacement.runtime, first.runtime);
        assert.equal(replacement.pid, first.pid);
        assert.notEqual(replacement.browser?.url, first.browser?.url);
        const app = runtimeApp(mcp, replacement);
        assert.equal((await app.find('Heading')).text, 'Routes ready');
        assert.equal((await app.find('DispatchedCount')).text, 'Dispatched orders: 1');
        assert.notEqual((await dispatch(replacement, 2)).id, originalDebug.id);
        assert.ok((await invoke('ferroui_browser_debug', { runtime, expected_generation: first.generation })).isError);
        console.log('Managed Chromium Rust source breakpoints, native dispatch, saved app replacement, and debugger reattachment passed');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        tracker.dispose();
        try { if (runtime) content(await invoke('ferroui_browser_stop', { runtime })); }
        finally { await settings.update('allowWrites', previous, vscode.ConfigurationTarget.Global); mcp.close(); }
    }
}
