import { randomUUID } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import * as vscode from 'vscode';
import { DebuggerPause, detachedDebugger, noStoppedDebugger, type DebuggerStatus } from './debuggerPause';
import { trackDebuggerPause } from './debuggerPauseTracker';

export const managedProcessVariable = '${command:ferroui.hotReload.processId}';

interface Attachment { id: string; folder: vscode.WorkspaceFolder; configuration: vscode.DebugConfiguration; generation?: number }
interface DebuggerOptions {
    attachError?: string;
    onDetach?(): void;
    onDidChange?(): void;
    programForGeneration?(generation: number): string | undefined;
}

export class ReloadDebugger implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private attachment?: Attachment;
    private active?: vscode.DebugSession;
    private readonly pause = new DebuggerPause();
    private transition = Promise.resolve();
    private readonly instance = randomUUID();
    private closing?: Promise<void>;

    constructor(private readonly output: Pick<vscode.OutputChannel, 'appendLine'>,
        private readonly options: DebuggerOptions = {}) {
        const stopWatching = this.pause.onDidChange(() => this.options.onDidChange?.());
        this.subscriptions = [
            { dispose: stopWatching },
            vscode.debug.registerDebugAdapterTrackerFactory('*', { createDebugAdapterTracker: session => {
                const generation = this.attachment?.generation;
                if (generation === undefined || !this.owns(session, generation)) return;
                return trackDebuggerPause(this.pause, () => this.owns(session, generation), session);
            } }),
            vscode.debug.onDidStartDebugSession(session => {
                if (session.configuration.ferrouiManagedInstance !== this.instance) return;
                if (session.parentSession?.configuration.ferrouiManagedInstance === this.instance) return;
                const attachment = this.attachment;
                if (attachment && session.configuration.ferrouiManagedSession === attachment.id) this.setActive(session);
                else void this.stop(session);
            }),
            vscode.debug.onDidTerminateDebugSession(session => {
                if (this.active?.id === session.id) {
                    this.attachment = undefined;
                    this.clearActive();
                    this.options.onDetach?.();
                }
            }),
        ];
    }

    async attach(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, target: number, generation = target): Promise<void> {
        this.prepare(folder, configuration);
        await this.activate(target, undefined, generation);
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

    follows(generation: number): boolean { return this.attachment?.generation === generation; }

    get attached(): boolean { return this.active !== undefined; }

    get status(): DebuggerStatus { return { attached: this.attached, paused: this.attached && this.pause.paused }; }

    statusFor(generation: number | undefined): DebuggerStatus {
        return generation !== undefined && this.follows(generation) ? this.status : detachedDebugger;
    }

    continueStopped(signal?: AbortSignal): Promise<void> {
        if (!this.status.paused) return Promise.reject(new Error(noStoppedDebugger));
        return this.pause.continueStopped(signal);
    }

    private owns(session: vscode.DebugSession, generation: number): boolean {
        const attachment = this.attachment;
        if (attachment?.generation !== generation) return false;
        for (let owner: vscode.DebugSession | undefined = session; owner; owner = owner.parentSession) {
            if (owner.configuration.ferrouiManagedInstance === this.instance
                && owner.configuration.ferrouiManagedSession === attachment.id) return true;
        }
        return false;
    }

    activated(target: number, configuration?: vscode.DebugConfiguration, generation = target): void {
        void this.activate(target, configuration, generation).catch(error => this.output.appendLine(`FerroUI debugger: ${String(error)}`));
    }

    private async activate(target: number, configuration: vscode.DebugConfiguration | undefined, generation: number): Promise<void> {
        const attached = this.attachment;
        if (!attached || this.follows(generation)) return;
        const old = this.active;
        const replacement = { ...attached, id: randomUUID(), generation, configuration: configuration ?? attached.configuration };
        this.attachment = replacement;
        this.clearActive();
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
        const program = attachment.generation === undefined ? undefined : this.options.programForGeneration?.(attachment.generation);
        if (program) config.program = program;
        config.ferrouiManagedInstance = this.instance;
        config.ferrouiManagedSession = attachment.id;
        try {
            if (!await vscode.debug.startDebugging(attachment.folder, config)) {
                throw new Error(this.options.attachError ?? 'The configured Rust debugger did not attach.');
            }
        } catch (error) {
            if (this.attachment === attachment) {
                const active = this.active;
                this.attachment = undefined;
                this.clearActive();
                if (active) await this.stop(active);
            }
            throw error;
        }
    }

    detach(): Promise<void> {
        const active = this.active;
        this.attachment = undefined;
        this.clearActive();
        if (active) this.transition = this.transition.then(() => this.stop(active));
        return this.transition;
    }

    detachForReplacement(): Promise<void> {
        const attachment = this.attachment;
        this.detach();
        if (attachment) this.attachment = { ...attachment, id: randomUUID(), generation: undefined };
        return this.transition;
    }

    private setActive(session: vscode.DebugSession): void {
        if (this.active?.id === session.id) return;
        this.active = session;
        this.options.onDidChange?.();
    }

    private clearActive(): void {
        const attached = this.attached;
        this.active = undefined;
        if (this.pause.paused) this.pause.clear();
        else if (attached) this.options.onDidChange?.();
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
