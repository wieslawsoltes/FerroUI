import * as vscode from 'vscode';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import { errorMessage } from './protocol';
import type { ProjectMetadata } from './reloadProcess';
import { commandFor } from './tools';

export const xamlLanguage = 'ferroui-xaml';

export class XamlServer implements vscode.Disposable {
    private client?: LanguageClient;
    private transition = Promise.resolve();
    private disposed = false;
    private readonly watch = vscode.workspace.createFileSystemWatcher('**/*.xaml');
    private readonly projects = new Map<string, ProjectMetadata>();

    constructor(private readonly context: vscode.ExtensionContext, private readonly channel: vscode.OutputChannel) {}

    start(restart = false): Promise<void> {
        this.transition = this.transition.then(async () => {
            if (this.disposed || (this.client && !restart)) return;
            await this.stop();
            const command = await commandFor('languageServer', this.context);
            if (this.disposed) return;
            const client = new LanguageClient('ferrouiXaml', 'FerroUI XAML', {
                command, transport: TransportKind.stdio,
            }, {
                documentSelector: [{ language: xamlLanguage, scheme: 'file' }],
                synchronize: { configurationSection: 'ferroui', fileEvents: this.watch },
                outputChannel: this.channel,
            });
            this.client = client;
            try {
                await client.start();
                for (const metadata of this.projects.values()) await this.publishMetadata(client, metadata);
            } catch (error) {
                await this.stop();
                throw error;
            }
        }).catch(error => {
            this.channel.appendLine(`XAML language server: ${errorMessage(error)}`);
        });
        return this.transition;
    }

    updateProject(metadata: ProjectMetadata): void {
        this.projects.set(metadata.manifest, metadata);
        this.transition = this.transition.then(async () => {
            if (this.client && !this.disposed && this.projects.get(metadata.manifest) === metadata) {
                await this.publishMetadata(this.client, metadata);
            }
        });
    }

    private async publishMetadata(client: LanguageClient, metadata: ProjectMetadata) {
        if (client.initializeResult?.capabilities.experimental?.ferrouiProjectMetadata !== true) return;
        try { await client.sendRequest('ferroui/projectMetadata', metadata); }
        catch (error) { this.channel.appendLine(`Project XAML metadata: ${errorMessage(error)}`); }
    }

    private async stop(): Promise<void> {
        const client = this.client;
        this.client = undefined;
        if (client) await client.dispose();
    }

    dispose(): void {
        this.disposed = true;
        this.watch.dispose();
        void this.transition.then(() => this.stop()).catch(error => this.channel.appendLine(errorMessage(error)));
    }
}
