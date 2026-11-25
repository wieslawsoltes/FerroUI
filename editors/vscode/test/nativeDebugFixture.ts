import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { McpFixture } from './mcpFixture';
import { until } from './wait';

interface AttachAttempt {
    detail: string;
    error?: string;
}

export async function waitForDebuggerAttachment({ invoke, content }: McpFixture, app: RuntimeSummary, attached: boolean): Promise<void> {
    await until(async () => {
        const { runtimes } = content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions'));
        const current = runtimes.find(item => item.runtime === app.runtime);
        return current?.generation === app.generation && current?.debuggerAttached === attached;
    });
}

export function trackNativeDebugger(configuration: string) {
    const sessions = new Map<string, vscode.DebugSession>();
    const attached = new Set<string>();
    const verified = new Set<string>();
    const hits = new Map<string, number>();
    const attempts = new Map<string, AttachAttempt>();
    const terminated = vscode.debug.onDidTerminateDebugSession(session => { sessions.delete(session.id); });
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('lldb-dap', {
        createDebugAdapterTracker(session) {
            if (!session.configuration.ferrouiManagedInstance || session.configuration.name !== configuration) return;
            sessions.set(session.id, session);
            const attempt: AttachAttempt = { detail: 'VS Code created the adapter' };
            attempts.set(session.id, attempt);
            return {
                onWillReceiveMessage(message) {
                    if (message.type === 'request' && message.command === 'attach') attempt.detail = 'VS Code sent the attach request';
                },
                onDidSendMessage(message) {
                    if (message.type === 'response' && message.command === 'attach') {
                        if (message.success) attached.add(session.id);
                        else attempt.error = message.message ?? 'LLDB rejected the attach request';
                    }
                    if ((message.command === 'setBreakpoints' && message.body?.breakpoints?.some((point: { verified: boolean }) => point.verified))
                        || (message.event === 'breakpoint' && message.body?.breakpoint?.verified)) verified.add(session.id);
                    if (message.event === 'stopped' && message.body?.reason === 'breakpoint') hits.set(session.id, message.body.threadId);
                },
                onError(error) { if (!attached.has(session.id)) attempt.error ??= error.message; },
                onExit(code, signal) {
                    if (!attached.has(session.id)) attempt.error ??= `Adapter exited (code ${code}, signal ${signal})`;
                },
            };
        },
    });
    return {
        sessions,
        async accept(previous?: string) {
            const find = () => [...sessions.values()].find(session => session.id !== previous && attached.has(session.id));
            const latest = () => [...attempts].findLast(([id]) => id !== previous)?.[1];
            try { await until(() => Boolean(find() || latest()?.error), 60_000); }
            catch (error) {
                const detail = latest()?.detail ?? 'VS Code did not create the configured adapter';
                throw new Error(`LLDB attach for ${configuration} timed out. ${detail}.`, { cause: error });
            }
            const session = find();
            assert.ok(session, `LLDB attach for ${configuration} failed. ${latest()?.error ?? 'The adapter exited.'}`);
            await until(() => verified.has(session.id));
            return session;
        },
        async breakpoint(session: vscode.DebugSession) {
            await until(() => hits.has(session.id));
            const thread = hits.get(session.id)!;
            hits.delete(session.id);
            return thread;
        },
        async close() {
            await Promise.allSettled([...sessions.values()].map(session => vscode.debug.stopDebugging(session)));
            tracker.dispose();
            terminated.dispose();
        },
    };
}
