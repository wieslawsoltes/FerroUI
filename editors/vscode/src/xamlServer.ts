import * as vscode from 'vscode';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import { errorMessage } from './protocol';
import { commandFor } from './tools';

export const xamlLanguage = 'ferroui-xaml';

export class XamlServer implements vscode.Disposable {
    private client?: LanguageClient;
    private transition = Promise.resolve();
    private disposed = false;

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
                synchronize: { configurationSection: 'ferroui' },
                outputChannel: this.channel,
            });
            this.client = client;
            try {
                await client.start();
            } catch (error) {
                await this.stop();
                throw error;
            }
        }).catch(error => {
            this.channel.appendLine(`XAML language server: ${errorMessage(error)}`);
        });
        return this.transition;
    }

    private async stop(): Promise<void> {
        const client = this.client;
        this.client = undefined;
        if (client) await client.dispose();
    }

    dispose(): void {
        this.disposed = true;
        void this.transition.then(() => this.stop()).catch(error => this.channel.appendLine(errorMessage(error)));
    }
}
