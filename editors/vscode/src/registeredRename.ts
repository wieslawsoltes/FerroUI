import * as vscode from 'vscode';
import type { Position } from './protocol';
import { planRegisteredType, registeredRustDrafts, type RegisteredRenamePlan, type RenameLocation, type RenamePosition, type RenameTarget } from './registeredTypePlan';
import { checkedSources, reviewedSourceUri } from './reviewedSources';
import { SourceChangeReview } from './sourceChangeReview';
import { asRange, exactRange } from './textDocument';
import { isWorkspaceFile, resolveWorkspaceFile, workspaceSourceUri } from './workspacePaths';
import type { XamlServer } from './xamlServer';

export interface RegisteredRenameAgentRequest { path: string; folder?: string; position: Position; new_name: string; review?: string }

export class RegisteredRename implements vscode.Disposable {
    private readonly review: SourceChangeReview;

    constructor(private readonly server: XamlServer, private readonly kind: 'property' | 'type') {
        const stem = kind === 'property' ? 'Property' : 'Type';
        this.review = new SourceChangeReview({
            scheme: `ferroui-${kind}-rename-review`,
            context: `ferroui.reviewing${stem}Rename`,
            command: `ferroui.${kind}RenameReview`,
        });
    }

    async open(editor: vscode.TextEditor | undefined): Promise<void> {
        if (!editor) throw new Error(`Place the caret on a registered XAML ${this.kind}.`);
        const { document } = editor;
        const request = requestAt(document, editor.selection.active);
        const target = await this.target(document, request);
        const newName = await vscode.window.showInputBox({
            title: this.kind === 'type' ? 'Rename Registered Type' : 'Rename Registered Property',
            value: target.name, prompt: `New public XAML ${this.kind} name`,
            validateInput: value => /^[\p{L}_][\p{L}\p{N}_]*$/u.test(value) && value !== target.name
                ? undefined : `Enter a different ${this.kind} identifier.`,
        });
        if (!newName) return;
        await this.openReview(document, request, newName);
    }

    async openReview(document: vscode.TextDocument, request: RenamePosition, newName: string): Promise<void> {
        const plan = await this.plan(document, request, newName);
        await this.review.open(`Rename ${plan.name} → ${plan.newName}`, plan.documents, async validate => {
            if (!await confirmUnmapped(plan.unmapped, this.kind)) return;
            const applied = await this.apply(document, request, newName, plan.review, undefined, validate);
            if (applied.documents.some(change => change.edit.uri === document.uri.toString())) return document.uri;
            const changed = applied.documents[0]?.edit.uri;
            return changed ? await reviewedSourceUri(vscode.Uri.parse(changed, true)) ?? document.uri : document.uri;
        });
    }

    async agent(input: RegisteredRenameAgentRequest, signal: AbortSignal): Promise<RegisteredRenamePlan> {
        const uri = await resolveWorkspaceFile(input.path, input.folder);
        const document = await vscode.workspace.openTextDocument(uri);
        const position = exactRange(document, { start: input.position, end: input.position }).start;
        const request = requestAt(document, position);
        signal.throwIfAborted();
        if (input.review === undefined) return this.plan(document, request, input.new_name, signal);
        return this.apply(document, request, input.new_name, input.review, signal);
    }

    private async target(document: vscode.TextDocument, request: RenamePosition): Promise<RenameTarget> {
        if (!vscode.workspace.isTrusted || document.languageId !== 'ferroui-xaml' || !await isWorkspaceFile(document.uri)) {
            throw new Error('Choose an authored FerroUI XAML file in a trusted workspace.');
        }
        const params = { textDocument: { uri: document.uri.toString() }, request };
        return this.kind === 'type' ? this.server.registeredTypeRequest('registeredTypeTarget', params)
            : this.server.registeredPropertyRequest('registeredPropertyTarget', params);
    }

    private async plan(document: vscode.TextDocument, request: RenamePosition, newName: string, signal?: AbortSignal): Promise<RegisteredRenamePlan> {
        const target = await this.target(document, request);
        if (this.kind === 'type') {
            const plan = await planRegisteredType(this.server, document, request, target, newName, signal);
            if (document.isClosed || document.version !== request.version) {
                throw new Error('The XAML source changed. Prepare the type review again.');
            }
            return plan;
        }
        const uri = await reviewedSourceUri(vscode.Uri.parse(target.rustUri, true));
        if (!uri) throw new Error('Open the workspace containing the Rust registration.');
        const rust = vscode.workspace.textDocuments.find(draft => draft.uri.toString() === uri.toString());
        const version = rust?.version;
        const plan = await this.server.registeredPropertyRequest<RegisteredRenamePlan>('planRegisteredPropertyRename', {
            textDocument: { uri: document.uri.toString() }, request, newName,
            sourceDrafts: await registeredRustDrafts(),
        });
        if (document.isClosed || document.version !== request.version || (rust && (rust.isClosed || rust.version !== version))) {
            throw new Error('The source changed. Prepare the property review again.');
        }
        return plan;
    }

    private async apply(document: vscode.TextDocument, request: RenamePosition, newName: string, review: string,
        signal?: AbortSignal, validate?: () => void): Promise<RegisteredRenamePlan> {
        const plan = await this.plan(document, request, newName, signal);
        if (plan.review !== review) throw new Error(`The ${this.kind} rename changed. Review the current source patches before applying it.`);
        if (plan.rustRename) throw new Error('Complete the Rust symbol review before applying.');
        const sources = await checkedSources(plan.documents);
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        for (const source of sources) edit.set(source.uri, source.edits);
        if (!await vscode.workspace.applyEdit(edit)) throw new Error(`VS Code could not apply the reviewed ${this.kind} rename.`);
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

function requestAt(document: vscode.TextDocument, position: vscode.Position): RenamePosition {
    return { version: document.version, position: { line: position.line, character: position.character } };
}

async function confirmUnmapped(locations: RenameLocation[], kind: 'property' | 'type'): Promise<boolean> {
    if (!locations.length) return true;
    const action = await vscode.window.showWarningMessage(
        `${locations.length} potential ${kind} references need manual review.`,
        { modal: true, detail: 'The reviewed patches update only references with a resolved registration.' },
        'Show Locations', 'Apply Mapped Rename',
    );
    if (action === 'Show Locations') {
        const selection = await vscode.window.showQuickPick(locations.map(location => ({
            label: vscode.workspace.asRelativePath(vscode.Uri.parse(location.uri)),
            description: `Line ${location.range.start.line + 1}`, location,
        })), { placeHolder: `Choose a potential ${kind} reference` });
        if (selection) {
            const { uri, range } = selection.location;
            const source = await workspaceSourceUri(vscode.Uri.parse(uri, true));
            if (source) await vscode.window.showTextDocument(source, { selection: asRange(range), preview: false });
        }
    }
    return action === 'Apply Mapped Rename';
}
