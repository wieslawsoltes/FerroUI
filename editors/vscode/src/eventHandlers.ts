import * as vscode from 'vscode';
import type { HandlerDraft, HandlerLocation, HandlerPlan, HandlerRequest, HandlerTarget } from './eventHandlerProtocol';
import { exactRange } from './textDocument';
import { workspaceSourceUri } from './workspacePaths';
import { xamlLanguage, type XamlServer } from './xamlServer';

export interface HandlerEditSite {
    document: vscode.TextDocument;
    request: HandlerRequest;
    validate?: () => void;
}

interface HandlerEditTarget {
    site: HandlerEditSite;
    target: HandlerTarget;
    rust: vscode.TextDocument;
    rustVersion: number;
}

export class EventHandlers {
    constructor(private readonly server: XamlServer) {}

    async create(site: HandlerEditSite): Promise<void> {
        const context = await this.target(site);
        const { target } = context;
        const existing = await this.definition(context);
        if (existing) return this.reveal(context.rust, existing);
        const name = target.existingName ?? await vscode.window.showInputBox({
            title: `Create ${target.event} handler`,
            prompt: `Register a Rust handler in ${vscode.workspace.asRelativePath(context.rust.uri)}`,
            value: target.suggestedName,
        });
        if (name === undefined) return;
        const plan = await this.plan(context, name.trim());
        await this.apply(context, plan);
        if (context.rust.version === context.rustVersion + 1) {
            await this.reveal(context.rust, plan.declaration);
        }
    }

    async definitionAt(document: vscode.TextDocument, position: vscode.Position, token: vscode.CancellationToken): Promise<vscode.Location | undefined> {
        try {
            if (token.isCancellationRequested) return;
            const context = await this.target({ document, request: { position, version: document.version } });
            const location = await this.definition(context);
            if (location && !token.isCancellationRequested) {
                return new vscode.Location(context.rust.uri, exactRange(context.rust, location.range));
            }
        } catch { /* Let the language server handle ordinary XAML navigation. */ }
    }

    async revealExisting(site: HandlerEditSite): Promise<boolean> {
        const context = await this.target(site);
        const location = await this.definition(context);
        if (!location) return false;
        this.check(context);
        await this.reveal(context.rust, location);
        return true;
    }

    async inspect(site: HandlerEditSite, name: string | undefined, signal: AbortSignal): Promise<unknown> {
        const context = await this.target(site);
        signal.throwIfAborted();
        const declaration = name === undefined ? await this.definition(context) : undefined;
        const plan = name === undefined ? undefined : await this.plan(context, name);
        signal.throwIfAborted();
        return { ...context.target, rustVersion: context.rustVersion, declaration, plan };
    }

    async scaffold(site: HandlerEditSite, name: string, expectedRustVersion: number, signal: AbortSignal): Promise<HandlerPlan> {
        const context = await this.target(site);
        signal.throwIfAborted();
        if (context.rustVersion !== expectedRustVersion) throw new Error('The Rust buffer changed. Read the event-handler target again.');
        const plan = await this.plan(context, name);
        await this.apply(context, plan, signal);
        return plan;
    }

    private async target(site: HandlerEditSite): Promise<HandlerEditTarget> {
        const { document, request } = site;
        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        if (!vscode.workspace.isTrusted || !folder || document.languageId !== xamlLanguage || document.uri.scheme !== 'file') {
            throw new Error('Open a XAML file in a trusted Cargo workspace to scaffold its Rust handler.');
        }
        this.checkXaml(site);
        const target = await this.server.handlerRequest<HandlerTarget>('eventHandlerTarget', { uri: document.uri.toString(), request });
        const rustUri = await workspaceSourceUri(vscode.Uri.parse(target.rustUri, true));
        if (!rustUri || vscode.workspace.getWorkspaceFolder(rustUri)?.uri.toString() !== folder.uri.toString()) {
            throw new Error('The Rust registration is outside this workspace folder.');
        }
        const rust = await vscode.workspace.openTextDocument(rustUri);
        this.checkXaml(site);
        return { site, target: { ...target, rustUri: rust.uri.toString() }, rust, rustVersion: rust.version };
    }

    private async definition(context: HandlerEditTarget): Promise<HandlerLocation | null> {
        if (!context.target.existingName) return null;
        const location = await this.server.handlerRequest<HandlerLocation | null>('eventHandlerDefinition', this.parameters(context));
        this.check(context);
        return location;
    }

    private async plan(context: HandlerEditTarget, name: string): Promise<HandlerPlan> {
        const plan = await this.server.handlerRequest<HandlerPlan>('planEventHandler', { ...this.parameters(context), name });
        this.check(context);
        return plan;
    }

    private parameters(context: HandlerEditTarget) {
        this.check(context);
        const rust: HandlerDraft = { uri: context.rust.uri.toString(), version: context.rustVersion, source: context.rust.getText() };
        return { uri: context.site.document.uri.toString(), request: context.site.request, rust };
    }

    private checkXaml(site: HandlerEditSite): void {
        site.validate?.();
        if (site.document.isClosed || site.document.version !== site.request.version) {
            throw new Error('The XAML changed. Prepare the event handler again.');
        }
    }

    private check(context: HandlerEditTarget): void {
        this.checkXaml(context.site);
        if (context.rust.isClosed || context.rust.version !== context.rustVersion) {
            throw new Error('The Rust buffer changed. Prepare the event handler again.');
        }
    }

    private async apply(context: HandlerEditTarget, plan: HandlerPlan, signal?: AbortSignal): Promise<void> {
        this.check(context);
        const [xaml, rust] = plan.edit.document_changes;
        const document = context.site.document;
        if (plan.edit.document_changes.length !== 2 || !xaml || !rust
            || xaml.version !== document.version || rust.version !== context.rustVersion
            || vscode.Uri.parse(rust.uri).fsPath !== context.rust.uri.fsPath) {
            throw new Error('The checked handler patch does not match the current editor buffers.');
        }
        const edit = new vscode.WorkspaceEdit();
        for (const { edits, target } of [{ edits: xaml.edits, target: document }, { edits: rust.edits, target: context.rust }]) {
            edit.set(target.uri, edits.map(item => new vscode.TextEdit(exactRange(target, item.range), item.newText)));
        }
        signal?.throwIfAborted();
        if (!await vscode.workspace.applyEdit(edit)) {
            throw new Error('VS Code could not apply the event handler to both buffers.');
        }
    }

    private async reveal(document: vscode.TextDocument, location: HandlerLocation): Promise<void> {
        const selection = exactRange(document, location.range);
        await vscode.window.showTextDocument(document, { selection });
    }
}
