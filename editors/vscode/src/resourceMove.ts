import * as vscode from 'vscode';
import type { Position, Range } from './protocol';
import { checkedSources, reviewedSourceUri, type ReviewedDocument } from './reviewedSources';
import { SourceChangeReview } from './sourceChangeReview';
import { exactRange } from './textDocument';
import { isWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import { xamlLanguage, type XamlServer } from './xamlServer';

interface Location { uri: string; range: Range }
interface Request { version: number; position: Position }
interface Anchor { document: vscode.TextDocument; request: Request }

export interface ResourceMoveDestination { uri: string; request: Request }
interface Destination extends ResourceMoveDestination { label: string; range: Range }
interface Target { name: string; declaration: Location; destinations: Destination[] }
interface Plan {
    name: string;
    from: Location;
    to: Location;
    review: string;
    linkedDestination: boolean;
    documents: ReviewedDocument[];
}
export interface ResourceMoveInput {
    path: string;
    folder?: string;
    position: Position;
    destination?: ResourceMoveDestination;
    review?: string;
}

export class ResourceMove implements vscode.Disposable {
    private readonly review = new SourceChangeReview({
        scheme: 'ferroui-resource-move-review',
        context: 'ferroui.reviewingResourceMove',
        command: 'ferroui.resourceMoveReview',
    });

    constructor(private readonly server: XamlServer) {}

    async open(document: vscode.TextDocument, position: Position): Promise<void> {
        const anchor = this.anchor(document, position);
        const target = await this.request<Target>('resourceMoveTarget', anchor);
        if (!target.destinations.length) throw new Error('Create or open another resource dictionary in this workspace.');
        const choices = target.destinations.map(destination => ({
            label: destination.label,
            description: vscode.workspace.asRelativePath(vscode.Uri.parse(destination.uri, true)),
            detail: `Line ${destination.range.start.line + 1}`,
            destination,
        }));
        const selected = await vscode.window.showQuickPick(choices, {
            title: `Move ${target.name}`,
            placeHolder: 'Choose a destination dictionary',
            matchOnDescription: true,
        });
        if (selected) await this.openReview(anchor, selected.destination);
    }

    async agent(input: ResourceMoveInput, signal: AbortSignal): Promise<Target | Plan> {
        const uri = await resolveWorkspaceFile(input.path, input.folder);
        const document = await vscode.workspace.openTextDocument(uri);
        const anchor = this.anchor(document, input.position);
        if (!input.destination) return this.request<Target>('resourceMoveTarget', anchor, undefined, signal);
        if (input.review === undefined) return this.plan(anchor, input.destination, signal);
        return this.apply(anchor, input.destination, input.review, signal);
    }

    async openReview(anchor: Anchor, destination: ResourceMoveDestination): Promise<void> {
        const plan = await this.plan(anchor, destination);
        await this.review.open(`Move Resource · ${plan.name}`, plan.documents, async validate => {
            const moved = await this.apply(anchor, destination, plan.review, undefined, validate);
            return await reviewedSourceUri(vscode.Uri.parse(moved.to.uri, true));
        });
    }

    private anchor(document: vscode.TextDocument, position: Position): Anchor {
        exactRange(document, { start: position, end: position });
        return { document, request: { version: document.version, position: { line: position.line, character: position.character } } };
    }

    private plan(anchor: Anchor, destination: ResourceMoveDestination, signal?: AbortSignal): Promise<Plan> {
        return this.request('planResourceMove', anchor, destination, signal);
    }

    private async request<T>(method: 'resourceMoveTarget' | 'planResourceMove', anchor: Anchor,
        destination?: ResourceMoveDestination, signal?: AbortSignal): Promise<T> {
        const { document, request } = anchor;
        if (!vscode.workspace.isTrusted || document.languageId !== xamlLanguage || !await isWorkspaceFile(document.uri)) {
            throw new Error('Open authored FerroUI XAML in a trusted workspace.');
        }
        requireCurrent(anchor);
        const result = await this.server.resourceMoveRequest<T>(method, {
            textDocument: { uri: document.uri.toString() }, request, destination,
        }, signal);
        requireCurrent(anchor);
        signal?.throwIfAborted();
        return result;
    }

    private async apply(anchor: Anchor, destination: ResourceMoveDestination, review: string,
        signal?: AbortSignal, validate?: () => void): Promise<Plan> {
        const plan = await this.plan(anchor, destination, signal);
        if (plan.review !== review) throw new Error('The resource move changed. Review the current patches before applying.');
        const sources = await checkedSources(plan.documents);
        requireCurrent(anchor);
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        for (const source of sources) edit.set(source.uri, source.edits);
        if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not apply the reviewed resource move.');
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

function requireCurrent({ document, request }: Anchor): void {
    if (document.isClosed || document.version !== request.version) throw new Error('The source changed. Start a new resource move review.');
}
