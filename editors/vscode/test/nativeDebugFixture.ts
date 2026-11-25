import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { McpFixture } from './mcpFixture';
import { until } from './wait';

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
    const terminated = vscode.debug.onDidTerminateDebugSession(session => { sessions.delete(session.id); });
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('lldb-dap', {
        createDebugAdapterTracker(session) {
            if (!session.configuration.ferrouiManagedInstance || session.configuration.name !== configuration) return;
            sessions.set(session.id, session);
            return { onDidSendMessage(message) {
                if (message.type === 'response' && message.command === 'attach' && message.success) attached.add(session.id);
                if ((message.command === 'setBreakpoints' && message.body?.breakpoints?.some((point: { verified: boolean }) => point.verified))
                    || (message.event === 'breakpoint' && message.body?.breakpoint?.verified)) verified.add(session.id);
                if (message.event === 'stopped' && message.body?.reason === 'breakpoint') hits.set(session.id, message.body.threadId);
            } };
        },
    });
    return {
        sessions,
        async accept(previous?: string) {
            const find = () => [...sessions.values()].find(session => session.id !== previous && attached.has(session.id));
            await until(() => Boolean(find()));
            const session = find();
            assert.ok(session, 'The configured native debugger started.');
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
