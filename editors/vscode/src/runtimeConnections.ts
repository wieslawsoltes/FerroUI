import { randomUUID } from 'node:crypto';
import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import { RuntimeClient } from './runtimeClient';
import { localMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';
import type { AndroidRuntimeTarget, AppRuntime, DeviceRuntime, IosRuntimeTarget, RuntimeKind, RuntimeTarget } from './runtimeTargets';

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
    simulator?: IosRuntimeTarget;
    android?: AndroidRuntimeTarget;
}

interface Connection { endpoint: RuntimeMcpEndpoint; generation: number }
type ExternalKind = DeviceRuntime | { kind: 'attached' };
interface Attachment extends Connection { runtime: AppRuntime; label: string; target: ExternalKind }

/** Credentials stay in the workspace extension host. Public summaries use stable session IDs. */
export class RuntimeConnections implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly subscriptions: vscode.Disposable[];
    private readonly lifetime = new AbortController();
    private readonly managed = new Map<AppRuntime, Connection>();
    private readonly external = new Map<AppRuntime, Attachment>();
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
        return managed.concat([...this.external.values()].map(({ runtime, label, generation, target }): RuntimeSummary => ({
            ...target, runtime, label, generation, available: true,
            status: target.kind === 'simulator' ? target.simulator.system : target.kind === 'android'
                ? (target.android.emulator ? 'Android emulator' : 'Android device') : 'Attached application',
            ...(target.kind === 'android' ? { pid: target.android.pid } : {}),
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
        const existing = this.attachments().find(item => item.endpoint.url === endpoint);
        const runtime: AppRuntime = existing?.runtime ?? `attached:${randomUUID()}`;
        const label = name?.trim() || existing?.label || `Attached · ${new URL(endpoint).host}`;
        this.external.set(runtime, { runtime, label, endpoint: { url: endpoint, token }, generation: ++this.nextGeneration, target: { kind: 'attached' } });
        this.activeAttachment = runtime;
        this.changed.fire();
        return this.describe(runtime)!;
    }

    detach(runtime: AppRuntime): void {
        const attachment = this.external.get(runtime);
        if (!attachment) throw new Error('Select an attached app session.');
        if (attachment.target.kind === 'simulator') throw new Error('Stop this iOS simulator app from App Sessions or with ferroui_ios_simulator_stop.');
        if (attachment.target.kind === 'android') throw new Error('Close this Android session from App Sessions or with ferroui_android_stop.');
        this.external.delete(runtime);
        if (this.activeAttachment === runtime) this.activeAttachment = this.attachments().at(-1)?.runtime;
        this.changed.fire();
    }

    addDevice(target: DeviceRuntime, label: string, endpoint: RuntimeMcpEndpoint): RuntimeSummary {
        if (!this.trusted()) throw new Error('Open a trusted workspace before connecting a device.');
        const runtime: AppRuntime = `${target.kind}:${randomUUID()}`;
        this.external.set(runtime, { runtime, label, endpoint, target, generation: ++this.nextGeneration });
        this.changed.fire();
        return this.describe(runtime)!;
    }

    removeDevice(runtime: AppRuntime): void {
        const connection = this.external.get(runtime);
        if (connection && connection.target.kind !== 'attached' && this.external.delete(runtime)) this.changed.fire();
    }

    private attachments(): Attachment[] {
        return [...this.external.values()].filter(item => item.target.kind === 'attached');
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
        return runtime && (this.managed.get(runtime) ?? this.external.get(runtime)) || undefined;
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
        const attached = this.attachments();
        if (!runtime && attached.length > 1) {
            const picked = await vscode.window.showQuickPick(attached.map(({ label, runtime }) => ({ label, runtime })),
                { title: 'Disconnect an Attached FerroUI App' });
            runtime = picked?.runtime;
        } else runtime ??= this.activeAttachment;
        if (!runtime) return;
        try { this.detach(runtime); }
        catch (error) { void vscode.window.showErrorMessage(String(error)); }
    }

    dispose(): void {
        this.lifetime.abort();
        this.external.clear();
        this.managed.clear();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}
