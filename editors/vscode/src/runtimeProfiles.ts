import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections, RuntimeTarget } from './runtimeConnections';
import type { RuntimeNode } from './runtimeProtocol';
import type { RuntimeSourceEditor } from './runtimeSource';

export interface ProfilePhase {
    phase: 'measure' | 'arrange' | 'render' | 'reconcile';
    calls: number;
    self_ms: number;
    total_ms: number;
    max_self_ms: number;
}
export interface ProfileRow {
    row_id: number;
    kind: 'control' | 'component';
    node_id: number | null;
    label: string;
    name: string | null;
    has_source: boolean;
    self_ms: number;
    phases: ProfilePhase[];
}
export interface NativeProfileReport {
    id: number;
    window_id: number;
    window_open: boolean;
    recording: boolean;
    elapsed_ms: number;
    remaining_ms: number;
    omitted_calls: number;
    rows: ProfileRow[];
}
export interface ProfileTarget { runtime: RuntimeTarget; generation: number }
export type ProfileReport = NativeProfileReport & ProfileTarget;

/** Shares the selected runtime capture between Diagnostics and editor MCP. */
export class RuntimeProfiles implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly subscription: vscode.Disposable;
    private readonly lifetime = new AbortController();
    private timer?: ReturnType<typeof setTimeout>;
    private nextRequest = 0;
    private applied = 0;
    report?: ProfileReport;
    error?: string;

    constructor(private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor) {
        this.subscription = runtimes.onDidChange(() => {
            if (this.report && !this.isCurrent(this.report)) {
                this.cancelTimer();
                this.changed.fire();
            }
        });
    }

    targets(): RuntimeTarget[] {
        return (['managed', 'attached'] as const).filter(target => this.runtimes.get(target));
    }

    isCurrent(target: ProfileTarget): boolean {
        return this.runtimes.revision(target.runtime) === target.generation && this.runtimes.get(target.runtime) !== undefined;
    }

    async windows(signal: AbortSignal, runtime = this.defaultTarget()): Promise<ProfileTarget & { windows: RuntimeNode[] }> {
        return this.use(runtime, undefined, signal, async (client, generation, active) => {
            if (!await client.supports('start_ui_profile')) throw new Error('This app’s inspector has no UI profiler.');
            return { runtime, generation, windows: await client.call<RuntimeNode[]>('list_windows', {}, active) };
        });
    }

    async start(target: ProfileTarget, window: number, duration: number, signal: AbortSignal): Promise<ProfileReport> {
        const report = await this.capture(target.runtime, target.generation, 'start_ui_profile',
            { window_id: window, duration_ms: duration }, signal);
        if (!report) throw new Error('The application returned no UI profile.');
        return report;
    }

    read(signal: AbortSignal, runtime = this.defaultTarget(), generation?: number, id?: number): Promise<ProfileReport | null> {
        return this.capture(runtime, generation, 'get_ui_profile', { profile_id: id }, signal);
    }

    async stop(target: ProfileTarget, id: number, signal: AbortSignal): Promise<ProfileReport> {
        const report = await this.capture(target.runtime, target.generation, 'stop_ui_profile', { profile_id: id }, signal);
        if (!report) throw new Error('The application returned no UI profile.');
        return report;
    }

    async sourceAt(target: ProfileTarget, id: number, row: number, signal: AbortSignal, reveal: boolean) {
        const source = await this.use(target.runtime, target.generation, signal, async (client, _, active) => {
            const report = await client.call<NativeProfileReport>('get_ui_profile', { profile_id: id }, active);
            const match = report.rows.find(entry => entry.row_id === row);
            if (!match?.has_source || match.node_id === null) throw new Error('This row has no live source to reveal.');
            return this.source.read(client, match.node_id, active);
        });
        if (!source) throw new Error('The control has no matching source in this workspace revision.');
        if (!this.isCurrent(target)) throw new Error('The application connection changed.');
        if (reveal) {
            const { document, position } = source;
            const editor = vscode.window.visibleTextEditors.find(editor => editor.document === document);
            await vscode.window.showTextDocument(document, { viewColumn: editor?.viewColumn ?? vscode.ViewColumn.Beside,
                selection: new vscode.Range(position, position) });
        }
        return this.source.describe(source, target.runtime);
    }

    private defaultTarget(): RuntimeTarget {
        return this.targets()[0] ?? 'managed';
    }

    private async capture(
        runtime: RuntimeTarget, generation: number | undefined, method: string, args: object, signal: AbortSignal,
    ): Promise<ProfileReport | null> {
        const request = ++this.nextRequest;
        const report = await this.use(runtime, generation, signal, async (client, current, active) => {
            const data = await client.call<NativeProfileReport | null>(method, args, active);
            return data && { ...data, runtime, generation: current };
        });
        if (report && !this.isCurrent(report)) throw new Error('The application connection changed.');
        const displayed = this.report;
        const restartedOrCompleted = displayed && report && report.runtime === displayed.runtime
            && report.generation === displayed.generation && (report.id < displayed.id
                || (report.id === displayed.id && report.recording && !displayed.recording));
        if (report && request >= this.applied && !restartedOrCompleted) {
            this.applied = request;
            this.report = report;
            this.error = undefined;
            this.schedule();
            this.changed.fire();
        }
        return report;
    }

    private async use<T>(runtime: RuntimeTarget, expected: number | undefined, signal: AbortSignal,
        action: (client: RuntimeClient, generation: number, active: AbortSignal) => Promise<T>): Promise<T> {
        const active = AbortSignal.any([signal, this.lifetime.signal]);
        active.throwIfAborted();
        const { endpoint, generation, current } = this.runtimes.session(runtime);
        if (expected !== undefined && expected !== generation) throw new Error('The application connection changed. Choose its current window.');
        const client = new RuntimeClient(endpoint);
        try {
            const result = await action(client, generation, active);
            active.throwIfAborted();
            if (!current()) throw new Error('The application connection changed.');
            return result;
        } finally { client.dispose(); }
    }

    private schedule(): void {
        this.cancelTimer();
        const report = this.report;
        if (!report?.recording || !this.isCurrent(report)) return;
        this.timer = setTimeout(() => {
            this.timer = undefined;
            void this.read(this.lifetime.signal, report.runtime, report.generation, report.id).catch(error => {
                if (report === this.report && !this.lifetime.signal.aborted) {
                    this.error = errorMessage(error);
                    this.changed.fire();
                }
            });
        }, Math.max(100, Math.min(750, report.remaining_ms)));
    }

    private cancelTimer(): void {
        if (this.timer) clearTimeout(this.timer);
        this.timer = undefined;
    }

    dispose(): void {
        this.lifetime.abort();
        this.cancelTimer();
        this.subscription.dispose();
        this.changed.dispose();
    }
}
