import * as vscode from 'vscode';
import type { HotReload } from './hotReload';
import { localMcpEndpoint, type RuntimeMcpEndpoint } from './runtimeMcp';

export type RuntimeTarget = 'managed' | 'attached';

/** Credentials stay in the workspace extension host. */
export class RuntimeConnections implements vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly subscriptions: vscode.Disposable[];
    private external?: RuntimeMcpEndpoint;
    private managedVersion = 0;
    private attachedVersion = 0;

    constructor(private readonly reload: HotReload) {
        this.subscriptions = [
            reload.onDidChangeRuntime(() => { this.managedVersion++; this.changed.fire(); }),
            vscode.commands.registerCommand('ferroui.mcp.attach', () => this.attach()),
            vscode.commands.registerCommand('ferroui.mcp.detach', () => {
                this.external = undefined;
                this.attachedVersion++;
                this.changed.fire();
            }),
        ];
    }

    get managedRevision() { return this.managedVersion; }
    revision(target: RuntimeTarget): number {
        return target === 'managed' ? this.managedVersion : this.attachedVersion;
    }

    get(target: RuntimeTarget): RuntimeMcpEndpoint | undefined {
        if (!vscode.workspace.isTrusted || !vscode.workspace.workspaceFolders?.length) return undefined;
        return target === 'managed' ? this.reload.runtimeMcp : this.external;
    }

    private async attach(): Promise<void> {
        const url = await vscode.window.showInputBox({ prompt: 'FerroUI MCP endpoint from the running application or Designer Studio',
            placeHolder: 'http://127.0.0.1:7491/mcp', validateInput: value => localMcpEndpoint(value) ? undefined : 'Use a loopback HTTP address ending in /mcp.' });
        if (!url || !localMcpEndpoint(url)) return;
        const token = await vscode.window.showInputBox({ prompt: 'Bearer token displayed by the running FerroUI MCP server', password: true,
            validateInput: value => /^[a-fA-F0-9]{64}$/.test(value) ? undefined : 'Enter the 64-character FerroUI bearer token.' });
        if (!token || !/^[a-fA-F0-9]{64}$/.test(token)) return;
        this.external = { url, token };
        this.attachedVersion++;
        this.changed.fire();
    }

    dispose(): void {
        this.external = undefined;
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}
