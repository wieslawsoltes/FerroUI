import * as path from 'node:path';
import * as vscode from 'vscode';
import { AgentTools } from './agentTools';
import { McpBridge } from './mcpBridge';
import { commandFor } from './tools';

interface StdioProfile { label: string; command: string; args?: string[] }

export class FerroMcpProvider implements vscode.McpServerDefinitionProvider, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeMcpServerDefinitions = this.changed.event;
    private readonly bridge: McpBridge;
    private readonly subscriptions: vscode.Disposable[];
    private attached?: vscode.McpHttpServerDefinition;

    constructor(private readonly context: vscode.ExtensionContext, tools: AgentTools, private readonly output: vscode.OutputChannel) {
        this.bridge = new McpBridge(tools);
        this.subscriptions = [
            vscode.lm.registerMcpServerDefinitionProvider('ferroui.mcp', this),
            vscode.workspace.onDidGrantWorkspaceTrust(() => this.changed.fire()),
            vscode.workspace.onDidChangeConfiguration(event => {
                if (event.affectsConfiguration('ferroui.mcp') || event.affectsConfiguration('ferroui.preview.path')) this.changed.fire();
            }),
            vscode.commands.registerCommand('ferroui.mcp.attach', () => this.attach()),
            vscode.commands.registerCommand('ferroui.mcp.detach', () => { this.attached = undefined; this.changed.fire(); }),
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
        if (this.attached) definitions.push(this.attached);
        for (const profile of config.get<StdioProfile[]>('stdioServers') ?? []) {
            if (!profile.label || !profile.command || !path.isAbsolute(profile.command)) continue;
            definitions.push(new vscode.McpStdioServerDefinition(profile.label, profile.command, profile.args ?? []));
        }
        return definitions;
    }

    private async attach(): Promise<void> {
        const endpoint = await vscode.window.showInputBox({ prompt: 'FerroUI MCP endpoint from the running application or Designer Studio',
            placeHolder: 'http://127.0.0.1:7491/mcp', validateInput: value => localEndpoint(value) ? undefined : 'Use a loopback HTTP address ending in /mcp.' });
        if (!endpoint || !localEndpoint(endpoint)) return;
        const token = await vscode.window.showInputBox({ prompt: 'Bearer token displayed by the running FerroUI MCP server', password: true,
            validateInput: value => /^[a-fA-F0-9]{64}$/.test(value) ? undefined : 'Enter the 64-character FerroUI bearer token.' });
        if (!token) return;
        this.attached = new vscode.McpHttpServerDefinition('FerroUI Running Application or Studio', vscode.Uri.parse(endpoint), { Authorization: `Bearer ${token}` });
        this.changed.fire();
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.bridge.dispose();
        this.changed.dispose();
    }
}

function localEndpoint(value: string): boolean {
    try {
        const url = new URL(value);
        return url.protocol === 'http:' && ['127.0.0.1', '[::1]'].includes(url.hostname)
            && url.pathname === '/mcp' && !url.search && !url.hash && !url.username && !url.password;
    } catch { return false; }
}
