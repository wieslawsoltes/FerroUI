import type * as vscode from 'vscode';
import type { AndroidBridge, AndroidReceipt } from './androidBridge';
import { startAndroidDebugServer, type AndroidDebugServer } from './androidDebugServer';
import { androidDebugSymbols } from './androidDebugSymbols';
import { ReloadDebugger } from './reloadDebugger';

/** Own the device server, forwarding rule and editor debugger for one attachment. */
export class AndroidDebugger {
    private readonly stop = new AbortController();
    private readonly debugger: ReloadDebugger;
    private server?: AndroidDebugServer;
    private starting?: Promise<void>;
    private closing?: Promise<void>;

    constructor(private readonly bridge: AndroidBridge, private readonly packageId: string, private readonly receipt: AndroidReceipt,
        private readonly output: Pick<vscode.OutputChannel, 'appendLine'>, private readonly onClose: () => void, private readonly symbols: readonly string[] = []) {
        this.debugger = new ReloadDebugger(output, 'LLDB DAP could not attach to the Android app.', () => this.retire());
    }

    attach(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, signal: AbortSignal): Promise<void> {
        return this.starting = this.start(folder, configuration, AbortSignal.any([signal, this.stop.signal, AbortSignal.timeout(35_000)]));
    }

    private async start(folder: vscode.WorkspaceFolder, configuration: vscode.DebugConfiguration, signal: AbortSignal) {
        const report = (message: string) => this.output.appendLine(message);
        const symbols = await androidDebugSymbols(this.bridge, this.packageId, this.receipt, this.symbols, signal);
        // Seed LLDB with the app library so the host debugger picks the Android ELF architecture.
        const program = configuration.program ?? symbols[0]?.file;
        const load: string[] = [];
        for (const { file, slide } of symbols) {
            const quoted = JSON.stringify(file);
            if (file !== program) load.push(`target modules add ${quoted}`);
            load.push(`target modules load --file ${quoted} --slide ${slide}`);
        }
        const config = load.length
            ? { ...configuration, program, postRunCommands: [...load, ...(configuration.postRunCommands ?? [])] }
            : configuration;
        const server = this.server = await startAndroidDebugServer(this.bridge, this.packageId, this.receipt, report, signal);
        const ended = () => this.retire();
        void server.closed.then(ended, ended);
        signal.throwIfAborted();
        const cancelAttach = () => { void server.close().catch(error => report(String(error))); };
        signal.addEventListener('abort', cancelAttach, { once: true });
        try {
            await this.debugger.attach(folder, { ...config, 'gdb-remote-host': '127.0.0.1', 'gdb-remote-port': server.port }, this.receipt.pid);
            signal.throwIfAborted();
        } finally { signal.removeEventListener('abort', cancelAttach); }
    }

    private retire(): void {
        void this.close().catch(error => this.output.appendLine(String(error)));
    }

    close(): Promise<void> {
        if (this.closing) return this.closing;
        this.stop.abort(new Error('The Android debugger session ended.'));
        return this.closing = (async () => {
            try { await this.debugger.close(); }
            finally {
                await this.starting?.catch(() => {});
                try { await this.server?.close(); }
                finally { this.onClose(); }
            }
        })();
    }
}
