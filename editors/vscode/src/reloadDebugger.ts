import { randomUUID } from 'node:crypto';
import * as vscode from 'vscode';

export const managedProcessVariable = '${command:ferroui.hotReload.processId}';

interface Attachment { id: string; folder: vscode.WorkspaceFolder; configuration: vscode.DebugConfiguration; pid?: number }

export class ReloadDebugger implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private attachment?: Attachment;
    private active?: vscode.DebugSession;
    private transition = Promise.resolve();
    private readonly instance = randomUUID();
    private closing?: Promise<void>;

    constructor(private readonly output: vscode.OutputChannel) {
        this.subscriptions = [
            vscode.debug.onDidStartDebugSession(session => {
                if (session.configuration.ferrouiManagedInstance !== this.instance) return;
                if (session.configuration.ferrouiManagedSession === this.attachment?.id) this.active = session;
                else void this.stop(session);
            }),
            vscode.debug.onDidTerminateDebugSession(session => {
                if (this.active?.id === session.id) {
                    this.active = undefined;
                    this.attachment = undefined;
                }
            }),
        ];
    }

    async attach(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, pid: number): Promise<void> {
        this.prepare(folder, configuration);
        await this.activate(pid);
    }

    prepare(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration): void {
        this.detach();
        this.attachment = { id: randomUUID(), folder, configuration };
    }

    activated(pid: number): void {
        void this.activate(pid).catch(error => this.output.appendLine(`FerroUI debugger: ${String(error)}`));
    }

    private async activate(pid: number): Promise<void> {
        const attached = this.attachment;
        if (!attached || attached.pid === pid) return;
        const old = this.active;
        this.active = undefined;
        const replacement = { ...attached, id: randomUUID(), pid };
        this.attachment = replacement;
        const step = this.transition.then(async () => {
            if (old) await this.stop(old);
            if (this.attachment === replacement) await this.start(replacement, pid);
        });
        this.transition = step.catch(() => undefined);
        await step;
    }

    private async start(attachment: Attachment, pid: number): Promise<void> {
        const config = JSON.parse(JSON.stringify(attachment.configuration).replaceAll(managedProcessVariable, String(pid))) as vscode.DebugConfiguration;
        config.ferrouiManagedInstance = this.instance;
        config.ferrouiManagedSession = attachment.id;
        try {
            if (!await vscode.debug.startDebugging(attachment.folder, config)) throw new Error('The configured Rust debugger did not attach.');
        } catch (error) {
            if (this.attachment === attachment) this.attachment = undefined;
            throw error;
        }
    }

    detach(): void {
        const active = this.active;
        this.attachment = undefined;
        this.active = undefined;
        if (active) this.transition = this.transition.then(() => this.stop(active));
    }

    private async stop(session: vscode.DebugSession): Promise<void> {
        try { await vscode.debug.stopDebugging(session); }
        catch (error) { this.output.appendLine(`FerroUI debugger: ${String(error)}`); }
    }

    close(): Promise<void> {
        if (this.closing) return this.closing;
        this.detach();
        this.closing = this.transition.finally(() => this.subscriptions.forEach(subscription => subscription.dispose()));
        return this.closing;
    }

    dispose(): void { void this.close(); }
}
