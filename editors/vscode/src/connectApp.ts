import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import type { RuntimeSummary } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';

export async function connectApp<T>(title: string, choose: () => T | Promise<T | undefined>,
    launch: (request: T, signal: AbortSignal) => Promise<RuntimeSummary>, open: (runtime: AppRuntime) => void): Promise<void> {
    try {
        const request = await choose();
        if (!request) return;
        const runtime = await vscode.window.withProgress({ title, location: vscode.ProgressLocation.Notification, cancellable: true },
            async (_, token) => {
                const controller = new AbortController();
                const subscription = token.onCancellationRequested(() => controller.abort());
                if (token.isCancellationRequested) controller.abort();
                try { return await launch(request, controller.signal); }
                catch (error) { if (!controller.signal.aborted) throw error; }
                finally { subscription.dispose(); }
            });
        if (runtime) open(runtime.runtime);
    } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
}
