import * as path from 'node:path';
import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { McpBridge } from './mcpBridge';
import type { RuntimeConnections } from './runtimeConnections';
import { commandFor } from './tools';

interface StdioProfile { label: string; command: string; args?: string[] }

export class FerroMcpProvider implements vscode.McpServerDefinitionProvider, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeMcpServerDefinitions = this.changed.event;
    private readonly bridge: McpBridge;
    private readonly subscriptions: vscode.Disposable[];
    constructor(private readonly context: vscode.ExtensionContext, tools: AgentTools, private readonly output: vscode.OutputChannel,
        private readonly runtimes: RuntimeConnections) {
        this.bridge = new McpBridge(tools);
        this.subscriptions = [
            vscode.lm.registerMcpServerDefinitionProvider('ferroui.mcp', this),
            vscode.workspace.onDidGrantWorkspaceTrust(() => this.changed.fire()),
            runtimes.onDidChange(() => this.changed.fire()),
            vscode.workspace.onDidChangeConfiguration(event => {
                if (event.affectsConfiguration('ferroui.mcp') || event.affectsConfiguration('ferroui.preview.path')) this.changed.fire();
            }),
        ];
    }

    async provideMcpServerDefinitions(): Promise<vscode.McpServerDefinition[]> {
        if (!vscode.workspace.isTrusted || !vscode.workspace.workspaceFolders?.length) return [];
        const definitions: vscode.McpServerDefinition[] = [];
        const config = vscode.workspace.getConfiguration('ferroui.mcp');
        if (config.get('enableDesigner', true)) {
            try {
                const command = await commandFor('preview', this.context);
                definitions.push(new vscode.McpStdioServerDefinition('FerroUI Designer in VS Code', command,
                    ['--mcp-editor'], await this.bridge.environment(), this.context.extension.packageJSON.version));
            } catch (error) { this.output.appendLine(String(error)); }
        }
        for (const { target, endpoint } of this.runtimes.endpoints()) {
            // Android's forwarded socket and its HTTP authority differ. Editor tools use the workspace transport.
            if (endpoint.devicePort !== undefined) continue;
            const kind = { managed: 'Managed', attached: 'Attached', simulator: 'Simulator', android: 'Android' }[target.kind];
            const name = `FerroUI ${kind} · ${target.label} (${target.runtime.split(':')[1].slice(0, 8)})`;
            definitions.push(new vscode.McpHttpServerDefinition(name, vscode.Uri.parse(endpoint.url),
                { Authorization: `Bearer ${endpoint.token}` }, String(target.generation)));
        }
        for (const profile of config.get<StdioProfile[]>('stdioServers') ?? []) {
            if (!profile.label || !profile.command || !path.isAbsolute(profile.command)) continue;
            definitions.push(new vscode.McpStdioServerDefinition(profile.label, profile.command, profile.args ?? []));
        }
        return definitions;
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.bridge.dispose();
        this.changed.dispose();
    }
}
