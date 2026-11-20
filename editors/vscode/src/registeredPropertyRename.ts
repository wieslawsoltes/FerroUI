import * as vscode from 'vscode';
import type { Position, Range } from './protocol';
import { checkedSources, reviewedSourceUri, type ReviewedDocument } from './reviewedSources';
import { SourceChangeReview } from './sourceChangeReview';
import { asRange, exactRange } from './textDocument';
import { isWorkspaceFile, resolveWorkspaceFile, workspaceSourceUri } from './workspacePaths';
import type { XamlServer } from './xamlServer';

interface Request { version: number; position: Position }
interface Location { uri: string; range: Range }
interface Target { name: string; range: Range; rustUri: string }
interface Plan { name: string; newName: string; review: string; documents: ReviewedDocument[]; unmapped: Location[] }
export interface RegisteredPropertyAgentRequest { path: string; folder?: string; position: Position; new_name: string; review?: string }

export class RegisteredPropertyRename implements vscode.Disposable {
    private readonly review = new SourceChangeReview({
        scheme: 'ferroui-property-rename-review', context: 'ferroui.reviewingPropertyRename', command: 'ferroui.propertyRenameReview',
    });

    constructor(private readonly server: XamlServer) {}

    async open(editor: vscode.TextEditor | undefined): Promise<void> {
        if (!editor) throw new Error('Place the caret on a registered XAML property.');
        const { document } = editor;
        const request = requestAt(document, editor.selection.active);
        const target = await this.target(document, request);
        const newName = await vscode.window.showInputBox({
            title: 'Rename Registered Property', value: target.name, prompt: 'New public XAML property name',
            validateInput: value => /^[\p{L}_][\p{L}\p{N}_]*$/u.test(value) && value !== target.name
                ? undefined : 'Enter a different property identifier.',
        });
        if (!newName) return;
        await this.openReview(document, request, newName);
    }

    async openReview(document: vscode.TextDocument, request: Request, newName: string): Promise<void> {
        const plan = await this.plan(document, request, newName);
        await this.review.open(`Rename ${plan.name} → ${plan.newName}`, plan.documents, async validate => {
            if (!await confirmUnmapped(plan.unmapped)) return;
            await this.apply(document, request, newName, plan.review, undefined, validate);
            return document.uri;
        });
    }

    async agent(input: RegisteredPropertyAgentRequest, signal: AbortSignal): Promise<Plan> {
        const uri = await resolveWorkspaceFile(input.path, input.folder);
        const document = await vscode.workspace.openTextDocument(uri);
        const position = exactRange(document, { start: input.position, end: input.position }).start;
        const request = requestAt(document, position);
        signal.throwIfAborted();
        if (input.review === undefined) return this.plan(document, request, input.new_name);
        return this.apply(document, request, input.new_name, input.review, signal);
    }

    private async target(document: vscode.TextDocument, request: Request): Promise<Target> {
        if (!vscode.workspace.isTrusted || document.languageId !== 'ferroui-xaml' || !await isWorkspaceFile(document.uri)) {
            throw new Error('Choose an authored FerroUI XAML file in a trusted workspace.');
        }
        return this.server.registeredPropertyRequest('registeredPropertyTarget', {
            textDocument: { uri: document.uri.toString() }, request,
        });
    }

    private async plan(document: vscode.TextDocument, request: Request, newName: string): Promise<Plan> {
        const target = await this.target(document, request);
        const uri = await reviewedSourceUri(vscode.Uri.parse(target.rustUri, true));
        if (!uri) throw new Error('Open the workspace containing the Rust registration.');
        const rust = vscode.workspace.textDocuments.find(draft => draft.uri.toString() === uri.toString());
        const version = rust?.version;
        const plan = await this.server.registeredPropertyRequest<Plan>('planRegisteredPropertyRename', {
            textDocument: { uri: document.uri.toString() }, request, newName,
            rust: rust ? { uri: rust.uri.toString(), version, source: rust.getText() } : null,
        });
        if (document.isClosed || document.version !== request.version || (rust && (rust.isClosed || rust.version !== version))) {
            throw new Error('The source changed. Prepare the property review again.');
        }
        return plan;
    }

    private async apply(document: vscode.TextDocument, request: Request, newName: string, review: string,
        signal?: AbortSignal, validate?: () => void): Promise<Plan> {
        const plan = await this.plan(document, request, newName);
        if (plan.review !== review) throw new Error('The property rename changed. Review the current source patches before applying it.');
        const sources = await checkedSources(plan.documents);
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        for (const source of sources) edit.set(source.uri, source.edits);
        if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not apply the reviewed property rename.');
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

function requestAt(document: vscode.TextDocument, position: vscode.Position): Request {
    return { version: document.version, position: { line: position.line, character: position.character } };
}

async function confirmUnmapped(locations: Location[]): Promise<boolean> {
    if (!locations.length) return true;
    const action = await vscode.window.showWarningMessage(
        `${locations.length} potential property references need manual review.`,
        { modal: true, detail: 'The reviewed patches update only references with a resolved registration.' },
        'Show Locations', 'Apply Mapped Rename',
    );
    if (action === 'Show Locations') {
        const selection = await vscode.window.showQuickPick(locations.map(location => ({
            label: vscode.workspace.asRelativePath(vscode.Uri.parse(location.uri)),
            description: `Line ${location.range.start.line + 1}`, location,
        })), { placeHolder: 'Choose a potential property reference' });
        if (selection) {
            const { uri, range } = selection.location;
            const source = await workspaceSourceUri(vscode.Uri.parse(uri, true));
            if (source) await vscode.window.showTextDocument(source, { selection: asRange(range), preview: false });
        }
    }
    return action === 'Apply Mapped Rename';
}
