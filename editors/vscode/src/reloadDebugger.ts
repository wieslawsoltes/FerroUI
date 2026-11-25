import { randomUUID } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import * as vscode from 'vscode';

export const managedProcessVariable = '${command:ferroui.hotReload.processId}';

interface Attachment { id: string; folder: vscode.WorkspaceFolder; configuration: vscode.DebugConfiguration; target?: number }

export class ReloadDebugger implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private attachment?: Attachment;
    private active?: vscode.DebugSession;
    private transition = Promise.resolve();
    private readonly instance = randomUUID();
    private closing?: Promise<void>;

    constructor(private readonly output: Pick<vscode.OutputChannel, 'appendLine'>,
        private readonly attachError = 'The configured Rust debugger did not attach.', onDetach?: () => void) {
        this.subscriptions = [
            vscode.debug.onDidStartDebugSession(session => {
                if (session.configuration.ferrouiManagedInstance !== this.instance) return;
                if (session.parentSession?.configuration.ferrouiManagedInstance === this.instance) return;
                if (session.configuration.ferrouiManagedSession === this.attachment?.id) this.active = session;
                else void this.stop(session);
            }),
            vscode.debug.onDidTerminateDebugSession(session => {
                if (this.active?.id === session.id) {
                    this.active = undefined;
                    this.attachment = undefined;
                    onDetach?.();
                }
            }),
        ];
    }

    async attach(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, target: number): Promise<void> {
        this.prepare(folder, configuration);
        await this.activate(target);
    }

    prepare(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration): void {
        this.detach();
        this.attachment = { id: randomUUID(), folder, configuration };
    }

    prepareIfIdle(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration): void {
        if (!this.attachment) {
            this.prepare(folder, configuration);
        } else if (this.attachment.folder.uri.toString() !== folder.uri.toString()
            || !isDeepStrictEqual(this.attachment.configuration, configuration)) {
            throw new Error('Stop the current debugger before choosing another attach configuration.');
        }
    }

    follows(target: number): boolean { return this.attachment?.target === target; }

    get attached(): boolean { return this.active !== undefined; }

    owns(session: vscode.DebugSession, target: number): boolean {
        const attachment = this.attachment;
        if (attachment?.target !== target) return false;
        for (let owner: vscode.DebugSession | undefined = session; owner; owner = owner.parentSession) {
            if (owner.configuration.ferrouiManagedInstance === this.instance
                && owner.configuration.ferrouiManagedSession === attachment.id) return true;
        }
        return false;
    }

    activated(target: number, configuration?: vscode.DebugConfiguration): void {
        void this.activate(target, configuration).catch(error => this.output.appendLine(`FerroUI debugger: ${String(error)}`));
    }

    private async activate(target: number, configuration?: vscode.DebugConfiguration): Promise<void> {
        const attached = this.attachment;
        if (!attached || this.follows(target)) return;
        const old = this.active;
        this.active = undefined;
        const replacement = { ...attached, id: randomUUID(), target, configuration: configuration ?? attached.configuration };
        this.attachment = replacement;
        const step = this.transition.then(async () => {
            if (old) await this.stop(old);
            if (this.attachment === replacement) await this.start(replacement, target);
        });
        this.transition = step.catch(() => undefined);
        await step;
    }

    private async start(attachment: Attachment, target: number): Promise<void> {
        const config = JSON.parse(JSON.stringify(attachment.configuration).replaceAll(managedProcessVariable, String(target))) as vscode.DebugConfiguration;
        if (config.type === 'lldb-dap' && attachment.configuration.pid === managedProcessVariable) config.pid = target;
        config.ferrouiManagedInstance = this.instance;
        config.ferrouiManagedSession = attachment.id;
        try {
            if (!await vscode.debug.startDebugging(attachment.folder, config)) throw new Error(this.attachError);
        } catch (error) {
            if (this.attachment === attachment) this.attachment = undefined;
            throw error;
        }
    }

    detach(): Promise<void> {
        const active = this.active;
        this.attachment = undefined;
        this.active = undefined;
        if (active) this.transition = this.transition.then(() => this.stop(active));
        return this.transition;
    }

    detachForReplacement(): Promise<void> {
        const attachment = this.attachment;
        this.detach();
        if (attachment) this.attachment = { ...attachment, id: randomUUID(), target: undefined };
        return this.transition;
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
