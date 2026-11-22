import { randomUUID } from 'node:crypto';
import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import { RuntimeClient } from './runtimeClient';
import { localMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';
import { type AppRuntime, type RuntimeKind, type RuntimeTarget } from './runtimeTargets';

export type { AppRuntime, RuntimeTarget } from './runtimeTargets';

export interface RuntimeSummary {
    runtime: AppRuntime;
    kind: RuntimeKind;
    label: string;
    status: string;
    generation: number | null;
    available: boolean;
    pid?: number;
    app_session?: string;
}

interface Connection { endpoint: RuntimeMcpEndpoint; generation: number }
interface Attachment extends Connection { runtime: AppRuntime; label: string }

/** Credentials stay in the workspace extension host. Public summaries use stable session IDs. */
export class RuntimeConnections implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly subscriptions: vscode.Disposable[];
    private readonly lifetime = new AbortController();
    private readonly managed = new Map<AppRuntime, Connection>();
    private readonly attached = new Map<AppRuntime, Attachment>();
    private activeAttachment?: AppRuntime;
    private nextGeneration = Date.now() * 1024;

    constructor(private readonly reload: HotReload) {
        this.subscriptions = [
            reload.onDidChangeRuntime(() => this.syncManaged()),
            vscode.commands.registerCommand('ferroui.mcp.attach', () => this.attachInteractive()),
            vscode.commands.registerCommand('ferroui.mcp.detach', (runtime?: AppRuntime) => this.detachInteractive(runtime)),
        ];
    }

    targets(): RuntimeSummary[] {
        if (!this.trusted()) return [];
        const managed = this.reload.runs().map((run): RuntimeSummary => ({
            runtime: run.runtime, kind: 'managed', label: run.label, status: run.state.status,
            app_session: run.id, pid: run.pid, generation: this.revision(run.runtime) ?? null, available: this.managed.has(run.runtime),
        }));
        return managed.concat([...this.attached.values()].map(attachment => ({
            runtime: attachment.runtime, kind: 'attached', label: attachment.label, status: 'Attached application',
            generation: attachment.generation, available: true,
        })));
    }

    defaultRuntime(): AppRuntime | undefined {
        const targets = this.targets();
        const active = this.resolve('managed');
        return targets.find(target => target.runtime === active && target.available)?.runtime
            ?? targets.find(target => target.available)?.runtime ?? targets.find(target => target.runtime === active)?.runtime ?? targets[0]?.runtime;
    }

    describe(target: RuntimeTarget): RuntimeSummary | undefined {
        const runtime = this.resolve(target);
        return this.targets().find(summary => summary.runtime === runtime);
    }

    endpoints(): { target: RuntimeSummary; endpoint: RuntimeMcpEndpoint }[] {
        return this.targets().flatMap(target => {
            const endpoint = this.get(target.runtime);
            return endpoint ? [{ target, endpoint }] : [];
        });
    }

    revision(target: RuntimeTarget): number | undefined { return this.connection(target)?.generation; }
    get(target: RuntimeTarget): RuntimeMcpEndpoint | undefined { return this.connection(target)?.endpoint; }

    session(target: RuntimeTarget) {
        const runtime = this.resolve(target);
        const connection = runtime && this.connection(runtime);
        if (!runtime || !connection) {
            if (target === 'managed') throw new Error('Start a managed FerroUI application with MCP inspection enabled.');
            if (target === 'attached') throw new Error('Attach an authenticated FerroUI application first.');
            throw new Error('This app inspector is unavailable. Read ferroui_app_sessions for the current sessions.');
        }
        const { endpoint, generation } = connection;
        return { runtime, endpoint, generation, current: () => this.connection(runtime) === connection };
    }

    async attach(url: string, token: string, name?: string, signal?: AbortSignal): Promise<RuntimeSummary> {
        if (!this.trusted()) throw new Error('Open a trusted workspace before attaching to a FerroUI app.');
        if (!localMcpEndpoint(url) || !/^[a-fA-F0-9]{64}$/.test(token)) throw new Error('Supply the loopback MCP address and its bearer token.');
        const endpoint = new URL(url).toString();
        const cancellation = signal ? AbortSignal.any([signal, this.lifetime.signal]) : this.lifetime.signal;
        cancellation.throwIfAborted();
        const probe = new RuntimeClient({ url: endpoint, token });
        const abort = () => probe.dispose();
        cancellation.addEventListener('abort', abort, { once: true });
        // Authenticate and read the catalog. Studio can expose tools without native window inspection.
        try { await probe.supports('list_windows'); }
        catch {
            cancellation.throwIfAborted();
            throw new Error('Could not authenticate to the running FerroUI app. Check its endpoint and bearer token.');
        } finally {
            cancellation.removeEventListener('abort', abort);
            probe.dispose();
        }
        cancellation.throwIfAborted();
        if (!this.trusted()) throw new Error('The trusted workspace closed while connecting.');
        const existing = [...this.attached.values()].find(item => item.endpoint.url === endpoint);
        const runtime: AppRuntime = existing?.runtime ?? `attached:${randomUUID()}`;
        const label = name?.trim() || existing?.label || `Attached · ${new URL(endpoint).host}`;
        this.attached.set(runtime, { runtime, label, endpoint: { url: endpoint, token }, generation: ++this.nextGeneration });
        this.activeAttachment = runtime;
        this.changed.fire();
        return this.describe(runtime)!;
    }

    detach(runtime: AppRuntime): void {
        if (!this.attached.delete(runtime)) throw new Error('Select an attached app session.');
        if (this.activeAttachment === runtime) this.activeAttachment = [...this.attached.keys()].at(-1);
        this.changed.fire();
    }

    private resolve(target: RuntimeTarget): AppRuntime | undefined {
        if (target === 'managed') {
            const active = this.reload.active;
            return active ? `managed:${active}` : undefined;
        }
        if (target === 'attached') return this.activeAttachment;
        return target;
    }

    private connection(target: RuntimeTarget): Connection | undefined {
        if (!this.trusted()) return;
        const runtime = this.resolve(target);
        return runtime && (this.managed.get(runtime) ?? this.attached.get(runtime)) || undefined;
    }

    private syncManaged(): void {
        if (this.lifetime.signal.aborted) return;
        const runs = this.reload.runs();
        const present = new Set(runs.map(run => run.runtime));
        for (const key of this.managed.keys()) if (!present.has(key)) this.managed.delete(key);
        for (const run of runs) {
            if (!run.endpoint) this.managed.delete(run.runtime);
            else if (this.managed.get(run.runtime)?.endpoint !== run.endpoint) {
                this.managed.set(run.runtime, { endpoint: run.endpoint, generation: ++this.nextGeneration });
            }
        }
        this.changed.fire();
    }

    private trusted(): boolean {
        return !this.lifetime.signal.aborted && vscode.workspace.isTrusted && Boolean(vscode.workspace.workspaceFolders?.length);
    }

    private async attachInteractive(): Promise<void> {
        const url = await vscode.window.showInputBox({ prompt: 'FerroUI MCP endpoint from the running application or Designer Studio', ignoreFocusOut: true,
            placeHolder: 'http://127.0.0.1:7491/mcp', validateInput: value => localMcpEndpoint(value) ? undefined : 'Use a loopback HTTP address ending in /mcp.' });
        if (!url) return;
        const token = await vscode.window.showInputBox({ prompt: 'Bearer token displayed by the running FerroUI MCP server', password: true, ignoreFocusOut: true,
            validateInput: value => /^[a-fA-F0-9]{64}$/.test(value) ? undefined : 'Enter the 64-character FerroUI bearer token.' });
        if (!token) return;
        try { await this.attach(url, token); }
        catch (error) { void vscode.window.showErrorMessage(String(error)); }
    }

    private async detachInteractive(runtime?: AppRuntime): Promise<void> {
        if (!runtime && this.attached.size > 1) {
            const picked = await vscode.window.showQuickPick([...this.attached.values()].map(({ label, runtime }) => ({ label, runtime })),
                { title: 'Disconnect an Attached FerroUI App' });
            runtime = picked?.runtime;
        } else runtime ??= this.activeAttachment;
        if (!runtime) return;
        try { this.detach(runtime); }
        catch (error) { void vscode.window.showErrorMessage(String(error)); }
    }

    dispose(): void {
        this.lifetime.abort();
        this.attached.clear();
        this.managed.clear();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}
