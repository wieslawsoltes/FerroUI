import * as vscode from 'vscode';
import { inCargoBuildSlot } from './cargoBuildGate';
import { selectCargoTarget } from './cargoTarget';
import { errorMessage } from './protocol';
import { buildCargo, type ProjectMetadata, type ReloadProcess } from './reloadProcess';

/** A schema build uses the selected project's normal Cargo configuration without launching it. */
export class ProjectSchemaBuild implements vscode.Disposable {
    private readonly command = vscode.commands.registerCommand('ferroui.buildProjectSchema', () => this.build());
    private active?: ReloadProcess;
    private pending?: AbortController;
    private disposed = false;

    constructor(private readonly output: vscode.OutputChannel, private readonly publish: (metadata: ProjectMetadata) => void) {}

    private async build(): Promise<void> {
        try {
            const target = await selectCargoTarget('Select a Cargo binary to build its FerroUI XAML schema');
            if (!target || this.disposed) return;
            if (this.pending) throw new Error('A FerroUI schema build is already running.');
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification, title: `Building XAML schema for ${target.label}`, cancellable: true,
            }, async (_, token) => {
                if (this.disposed || token.isCancellationRequested) return;
                if (this.pending) throw new Error('A FerroUI schema build is already running.');
                const pending = new AbortController();
                this.pending = pending;
                const cancellation = token.onCancellationRequested(() => { pending.abort(); void this.active?.stop(); });
                try {
                    await inCargoBuildSlot(target.targetDirectory, pending.signal, async () => {
                        const build = buildCargo(target, this.output);
                        this.active = build.process;
                        await build.artifact;
                        if (!pending.signal.aborted) this.publish(build.metadata());
                    });
                } catch (error) {
                    if (!this.disposed && !token.isCancellationRequested) throw error;
                } finally {
                    cancellation.dispose();
                    this.pending = undefined;
                    this.active = undefined;
                }
            });
        } catch (error) {
            this.output.appendLine(errorMessage(error));
            void vscode.window.showErrorMessage(`FerroUI project schema build failed. ${errorMessage(error)}`, 'Show Output')
                .then(action => { if (action) this.output.show(true); });
        }
    }

    async dispose(): Promise<void> {
        this.disposed = true;
        this.command.dispose();
        this.pending?.abort();
        await this.active?.stop();
    }
}
