import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { McpFixture } from './mcpFixture';
import { until } from './wait';

interface AttachAttempt {
    detail: string;
    error?: string;
    breakpoint?: string;
    disconnect?: { terminateDebuggee?: boolean; success?: boolean; error?: string };
    exitCode?: number;
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
                    if (message.type === 'request' && message.command === 'setBreakpoints') {
                        const file = message.arguments?.source?.path;
                        attempt.breakpoint = file ? `VS Code requested breakpoints in ${file}` : 'VS Code requested source breakpoints';
                    }
                    if (message.type === 'request' && message.command === 'disconnect') {
                        attempt.disconnect = { terminateDebuggee: message.arguments?.terminateDebuggee };
                    }
                },
                onDidSendMessage(message) {
                    if (message.type === 'response' && message.command === 'attach') {
                        if (message.success) attached.add(session.id);
                        else attempt.error = message.message ?? 'LLDB rejected the attach request';
                    }
                    if (message.type === 'response' && message.command === 'setBreakpoints') {
                        const points = message.body?.breakpoints as { verified: boolean; message?: string }[] | undefined;
                        const detail = message.message || points?.[0]?.message;
                        if (points?.some(point => point.verified)) verified.add(session.id);
                        else if (detail) attempt.breakpoint = detail;
                    }
                    if (message.event === 'breakpoint' && message.body?.breakpoint?.verified) verified.add(session.id);
                    if (message.event === 'stopped' && message.body?.reason === 'breakpoint') hits.set(session.id, message.body.threadId);
                    if (message.type === 'response' && message.command === 'disconnect') {
                        attempt.disconnect = { ...attempt.disconnect, success: message.success, error: message.message };
                    }
                    if (message.event === 'exited') attempt.exitCode = message.body?.exitCode;
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
        disconnectStatus(session: vscode.DebugSession) {
            const attempt = attempts.get(session.id);
            return { disconnect: attempt?.disconnect, debuggeeExitCode: attempt?.exitCode };
        },
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
            try { await until(() => verified.has(session.id)); }
            catch (error) {
                const detail = attempts.get(session.id)?.breakpoint ?? 'VS Code has no source breakpoint response';
                throw new Error(`LLDB did not verify a source breakpoint for ${configuration}. ${detail}.`, { cause: error });
            }
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
