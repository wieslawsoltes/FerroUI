import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import type { RuntimeConnections } from './runtimeConnections';
import { RuntimePanel } from './runtimePanel';
import type { RuntimeSourceEditor } from './runtimeSource';
import type { AppRuntime } from './runtimeTargets';

export class RuntimePanels implements vscode.Disposable {
    private readonly panels = new Map<AppRuntime | undefined, RuntimePanel>();
    private readonly command = vscode.commands.registerCommand('ferroui.openLiveApp', (runtime?: AppRuntime) => this.open(runtime));

    constructor(private readonly context: vscode.ExtensionContext, private readonly connections: RuntimeConnections,
        private readonly reload: HotReload, private readonly source: RuntimeSourceEditor) {}

    open(runtime?: AppRuntime): void {
        if (runtime && !this.connections.targets().some(target => target.runtime === runtime)) throw new Error('Select a current FerroUI app session.');
        let panel = this.panels.get(runtime);
        if (!panel) {
            panel = new RuntimePanel(this.context, this.connections, this.reload, this.source, runtime, () => this.panels.delete(runtime));
            this.panels.set(runtime, panel);
        }
        panel.show();
    }

    dispose(): void {
        this.command.dispose();
        for (const panel of this.panels.values()) panel.dispose();
        this.panels.clear();
    }
}
