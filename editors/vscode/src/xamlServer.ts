import * as vscode from 'vscode';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';
import type { DictionaryMethod } from './dictionaryProtocol';
import type { HandlerMethod } from './eventHandlerProtocol';
import { errorMessage } from './protocol';
import type { ProjectMetadata } from './reloadProcess';
import { commandFor } from './tools';

export const xamlLanguage = 'ferroui-xaml';

export class XamlServer implements vscode.Disposable {
    private client?: LanguageClient;
    private transition = Promise.resolve();
    private disposed = false;
    private readonly watch = vscode.workspace.createFileSystemWatcher('**/*.[xX][aA][mM][lL]');
    private readonly projects = new Map<string, ProjectMetadata>();
    eventDefinition?: (document: vscode.TextDocument, at: vscode.Position, token: vscode.CancellationToken) => Promise<vscode.Location | undefined>;

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
                initializationOptions: { ferrouiHandlerCommands: true, ferrouiExtractionCommands: vscode.workspace.isTrusted },
                middleware: {
                    provideDefinition: async (document, position, token, next) => {
                        const declaration = await this.eventDefinition?.(document, position, token);
                        return declaration ?? next(document, position, token);
                    },
                },
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

    async handlerRequest<T>(method: HandlerMethod, params: object): Promise<T> {
        await this.start();
        const client = this.client;
        if (client?.initializeResult?.capabilities.experimental?.ferrouiEventHandlers !== true) {
            throw new Error('Use a FerroUI XAML language server that supports event handlers.');
        }
        return client.sendRequest<T>(`ferroui/${method}`, params);
    }

    async dictionaryRequest<T>(method: DictionaryMethod, params: object): Promise<T> {
        await this.start();
        const client = this.client;
        if (client?.initializeResult?.capabilities.experimental?.ferrouiDictionaryExtraction !== true) {
            throw new Error('Use a FerroUI XAML language server that supports resource extraction.');
        }
        return client.sendRequest<T>(`ferroui/${method}`, params);
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
