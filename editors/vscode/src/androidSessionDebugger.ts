import type * as vscode from 'vscode';
import type { AndroidBridge, AndroidReceipt } from './androidBridge';
import { AndroidDebugger } from './androidDebugger';
import { errorMessage } from './protocol';

/** Keep the selected launch configuration across managed installs. Each PID gets its own server. */
export class AndroidSessionDebugger {
    private launch?: { folder: vscode.WorkspaceFolder; configuration: vscode.DebugConfiguration };
    private attachment?: AndroidDebugger;
    private disconnecting?: Promise<void>;

    constructor(private readonly bridge: AndroidBridge, private readonly packageId: string,
        private readonly output: Pick<vscode.OutputChannel, 'appendLine'>) {}

    async attach(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, receipt: AndroidReceipt,
        symbols: readonly string[], signal: AbortSignal): Promise<void> {
        signal.throwIfAborted();
        if (this.launch || this.disconnecting) throw new Error('Stop the current Android debugger before attaching again.');
        this.launch = { folder, configuration };
        await this.start(receipt, symbols, signal);
    }

    suspend(): Promise<void> {
        const attachment = this.attachment;
        if (!attachment) return this.disconnecting ?? Promise.resolve();
        this.attachment = undefined;
        return this.disconnecting = attachment.close().finally(() => { this.disconnecting = undefined; });
    }

    resume(receipt: AndroidReceipt, symbols: readonly string[], signal: AbortSignal): void {
        void this.start(receipt, symbols, signal).catch(error => this.report(error));
    }

    close(): Promise<void> {
        this.launch = undefined;
        return this.suspend();
    }

    private async start(receipt: AndroidReceipt, symbols: readonly string[], signal: AbortSignal): Promise<void> {
        const launch = this.launch;
        if (!launch || this.attachment || signal.aborted) return;
        const attachment = new AndroidDebugger(this.bridge, this.packageId, receipt, this.output, () => {
            if (this.attachment !== attachment) return;
            this.attachment = undefined;
            this.launch = undefined;
        }, symbols);
        this.attachment = attachment;
        try { await attachment.attach(launch.folder, launch.configuration, signal); }
        catch (error) {
            await attachment.close().catch(problem => this.report(problem));
            throw error;
        }
    }

    private report(error: unknown): void { this.output.appendLine(`FerroUI Android debugger: ${errorMessage(error)}`); }
}
