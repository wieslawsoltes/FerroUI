import * as vscode from 'vscode';
import type { BuiltProject, BuiltProjects } from './builtProjects';
import { NativePreview } from './nativePreview';
import { errorMessage, type Source } from './protocol';
import type { RsxLanguageDocument, RsxProperty } from './rsxLanguageTypes';
import { asRange } from './textDocument';

const rust = { language: 'rust', scheme: 'file' };

export class RsxLanguage implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private drafts = new WeakMap<vscode.TextDocument, { version: number; project?: BuiltProject; value: Promise<RsxLanguageDocument | undefined> }>();
    private client?: Promise<NativePreview>;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly projects: BuiltProjects) {
        this.subscriptions = [
            vscode.languages.registerCompletionItemProvider(rust, { provideCompletionItems: this.complete.bind(this) }),
            vscode.languages.registerHoverProvider(rust, { provideHover: this.hover.bind(this) }),
            vscode.languages.registerInlayHintsProvider(rust, { provideInlayHints: this.inlays.bind(this) }),
            vscode.workspace.onDidChangeConfiguration(change => {
                if (change.affectsConfiguration('ferroui.preview.path')) this.reset();
            }),
        ];
    }

    private async analyze(document: vscode.TextDocument, token: vscode.CancellationToken) {
        const version = document.version;
        const project = this.projects.forDocument(document.uri);
        let draft = this.drafts.get(document);
        if (draft?.version !== version || draft.project !== project) {
            const source = document.getText();
            if (token.isCancellationRequested || !/\brsx\s*!/.test(source) || Buffer.byteLength(source) > 1024 * 1024) return;
            const input: Source = { source, uri: document.uri.toString(), language: 'rust', caret: { line: 0, character: 0 } };
            const value: Promise<RsxLanguageDocument | undefined> = this.read(input, project).catch(error => {
                if (this.drafts.get(document)?.value === value) this.drafts.delete(document);
                this.output.appendLine(`RSX language tools: ${errorMessage(error)}`);
                return undefined;
            });
            draft = { version, project, value };
            this.drafts.set(document, draft);
        }
        const model = await draft.value;
        if (!token.isCancellationRequested && !document.isClosed && document.version === version
            && this.drafts.get(document) === draft && this.projects.forDocument(document.uri) === project) return model;
    }

    private async read(source: Source, project?: BuiltProject) {
        let client = this.client;
        if (client && (await client).closed) {
            if (client === this.client) this.client = undefined;
            client = this.client;
        }
        if (!client) {
            client = NativePreview.start(this.context, this.output);
            this.client = client;
            void client.catch(() => { if (this.client === client) this.client = undefined; });
        }
        const native = await client;
        return native.rsxLanguage(await native.inProject(source, project));
    }

    private async complete(document: vscode.TextDocument, position: vscode.Position, token: vscode.CancellationToken) {
        if (!vscode.workspace.getConfiguration('ferroui', document.uri).get('rsx.completion', true)) return;
        const model = await this.analyze(document, token);
        for (const control of model?.nodes ?? []) {
            const field = control.fields.find(field => !field.attached && asRange(field.range).contains(position));
            if (!field || !model) continue;
            const existing = new Set(control.fields.filter(other => other !== field).map(other => other.name));
            return (model.properties[control.key ?? control.class] ?? []).filter(property => !existing.has(property.setter)).map(property => {
                const item = new vscode.CompletionItem(property.setter, vscode.CompletionItemKind.Property);
                item.range = asRange(field.range);
                item.detail = `${control.class}.${property.xamlName} · ${property.valueType}`;
                if (field.shorthand) {
                    const snippet = new vscode.SnippetString(`${property.setter}: `);
                    snippet.appendPlaceholder(literalDefault(property));
                    item.insertText = snippet;
                }
                return item;
            });
        }
        return undefined;
    }

    private async hover(document: vscode.TextDocument, position: vscode.Position, token: vscode.CancellationToken) {
        const model = await this.analyze(document, token);
        for (const control of model?.nodes ?? []) {
            const field = control.fields.find(field => field.property && asRange(field.range).contains(position));
            if (!field?.property) continue;
            const markup = new vscode.MarkdownString();
            markup.appendCodeblock(`${control.class}.${field.property.xamlName}: ${field.property.valueType}`, 'rust');
            if (field.literal) markup.appendText('This literal maps to the visual designer.');
            return new vscode.Hover(markup, asRange(field.range));
        }
        return undefined;
    }

    private async inlays(document: vscode.TextDocument, range: vscode.Range, token: vscode.CancellationToken) {
        if (!vscode.workspace.getConfiguration('ferroui', document.uri).get('rsx.inlayHints', false)) return;
        const model = await this.analyze(document, token);
        return (model?.nodes ?? []).flatMap(control => control.fields.flatMap(field => {
            const position = asRange(field.range).end;
            if (!field.literal || !field.property || !range.contains(position)) return [];
            const hint = new vscode.InlayHint(position, ` ${field.property.valueType}`, vscode.InlayHintKind.Type);
            hint.paddingLeft = true;
            hint.tooltip = `FerroUI ${control.class}.${field.property.xamlName}`;
            return [hint];
        }));
    }

    private reset(): void {
        void this.client?.then(client => client.dispose(), () => {});
        this.client = undefined;
        this.drafts = new WeakMap();
    }

    dispose(): void {
        this.reset();
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

function literalDefault(property: RsxProperty): string {
    switch (property.valueType) {
        case 'bool': case 'Option<bool>': return 'true';
        case 'f64': return '0.0';
        case 'i32': return '0';
        case 'String': case 'Option<String>': case 'Option<BoxedValue>': return '""';
        default: return 'value';
    }
}
