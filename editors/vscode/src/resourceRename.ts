import * as vscode from 'vscode';
import type { Position } from './protocol';
import { checkedSources, reviewedSourceUri, type ReviewedDocument, type SourceRenameAgentRequest } from './reviewedSources';
import { SourceChangeReview } from './sourceChangeReview';
import { exactRange } from './textDocument';
import { isWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import { xamlLanguage, type XamlServer } from './xamlServer';

export interface ResourceRenameRequest {
    version: number;
    position: Position;
}

interface RenameSite {
    document: vscode.TextDocument;
    request: ResourceRenameRequest;
}

interface ResourceRenamePlan {
    name: string;
    newName: string;
    review: string;
    documents: ReviewedDocument[];
}

export class ResourceRename implements vscode.Disposable {
    private readonly review = new SourceChangeReview({
        scheme: 'ferroui-resource-rename-review',
        context: 'ferroui.reviewingResourceRename',
        command: 'ferroui.resourceRenameReview',
    });

    constructor(private readonly server: XamlServer) {}

    async open(site: RenameSite): Promise<void> {
        const target = await this.request<{ name: string }>('resourceRenameTarget', site);
        const newName = await vscode.window.showInputBox({
            title: 'Rename Resource and References',
            value: target.name,
            prompt: 'New resource key',
            validateInput: value => value.trim() && value !== target.name ? undefined : 'Enter a different resource key.',
        });
        if (newName === undefined) return;
        await this.openReview(site, newName);
    }

    async openReview(site: RenameSite, newName: string): Promise<void> {
        const plan = await this.plan(site, newName);
        await this.review.open(`Rename Resource · ${plan.name} → ${plan.newName}`, plan.documents, async validate => {
            const applied = await this.apply(site, newName, plan.review, undefined, validate);
            const source = site.document.uri;
            if (applied.documents.some(change => change.edit.uri === source.toString())) return source;
            const destination = vscode.Uri.parse(applied.documents[0].edit.uri, true);
            return await reviewedSourceUri(destination) ?? source;
        });
    }

    async agent(input: SourceRenameAgentRequest, signal: AbortSignal): Promise<ResourceRenamePlan> {
        const uri = await resolveWorkspaceFile(input.path, input.folder);
        const document = await vscode.workspace.openTextDocument(uri);
        const position = exactRange(document, { start: input.position, end: input.position }).start;
        const site = { document, request: { version: document.version, position } };
        signal.throwIfAborted();
        if (input.review === undefined) return this.plan(site, input.new_name, signal);
        return this.apply(site, input.new_name, input.review, signal);
    }

    private plan(site: RenameSite, newName: string, signal?: AbortSignal): Promise<ResourceRenamePlan> {
        return this.request('planResourceRename', site, newName, signal);
    }

    private async request<T>(method: 'resourceRenameTarget' | 'planResourceRename', site: RenameSite,
        newName?: string, signal?: AbortSignal): Promise<T> {
        if (!vscode.workspace.isTrusted || site.document.languageId !== xamlLanguage
            || !await isWorkspaceFile(site.document.uri)) {
            throw new Error('Open an authored FerroUI XAML file in a trusted workspace.');
        }
        requireCurrent(site);
        signal?.throwIfAborted();
        const { line, character } = site.request.position;
        const result = await this.server.resourceRenameRequest<T>(method, {
            textDocument: { uri: site.document.uri.toString() },
            request: { version: site.request.version, position: { line, character } }, newName,
        });
        requireCurrent(site);
        signal?.throwIfAborted();
        return result;
    }

    private async apply(site: RenameSite, newName: string, review: string,
        signal?: AbortSignal, validate?: () => void): Promise<ResourceRenamePlan> {
        const plan = await this.plan(site, newName, signal);
        if (plan.review !== review) throw new Error('The resource rename changed. Review the current patches before applying it.');
        const sources = await checkedSources(plan.documents);
        requireCurrent(site);
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        for (const source of sources) edit.set(source.uri, source.edits);
        if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not apply the reviewed resource rename.');
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

function requireCurrent({ document, request }: RenameSite): void {
    if (document.isClosed || document.version !== request.version) {
        throw new Error('The XAML source changed. Prepare the resource rename again.');
    }
}
