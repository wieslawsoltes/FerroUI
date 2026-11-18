import { randomUUID } from 'node:crypto';
import * as vscode from 'vscode';

interface Attachment { id: string; folder: vscode.WorkspaceFolder; configuration: vscode.DebugConfiguration; pid: number }

export class ReloadDebugger implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private attachment?: Attachment;
    private active?: vscode.DebugSession;
    private transition = Promise.resolve();

    constructor(private readonly output: vscode.OutputChannel) {
        this.subscriptions = [
            vscode.debug.onDidStartDebugSession(session => {
                if (session.configuration.ferrouiManagedSession === this.attachment?.id) this.active = session;
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
        this.detach();
        const attachment = { id: randomUUID(), folder, configuration, pid };
        this.attachment = attachment;
        await this.start(attachment);
    }

    activated(pid: number): void {
        const attached = this.attachment;
        if (!attached || attached.pid === pid) return;
        const old = this.active;
        this.active = undefined;
        const replacement = { ...attached, id: randomUUID(), pid };
        this.attachment = replacement;
        this.transition = this.transition.then(async () => {
            if (old) await vscode.debug.stopDebugging(old);
            if (this.attachment === replacement) await this.start(replacement);
        }).catch(error => this.output.appendLine(`FerroUI debugger: ${String(error)}`));
    }

    private async start(attachment: Attachment): Promise<void> {
        const config = { ...attachment.configuration, ferrouiManagedSession: attachment.id };
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
        if (active) void Promise.resolve(vscode.debug.stopDebugging(active)).catch(error => this.output.appendLine(String(error)));
    }

    dispose(): void {
        this.detach();
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}
