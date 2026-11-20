import { lstat, readFile, realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { FileMoveDocument, FileMovePlan } from './fileMoveProtocol';
import { recoverFileMove } from './fileMoveRecovery';
import { FileMoveReview } from './fileMoveReview';
import { asRange } from './textDocument';
import { isWorkspaceFile, newWorkspaceFile, resolveWorkspaceFile, workspaceSourceUri } from './workspacePaths';
import type { XamlServer } from './xamlServer';

export interface FileMoveRequest { path: string; folder?: string; destination: string }

export class FileMoves implements vscode.Disposable {
    private readonly review = new FileMoveReview();

    constructor(private readonly server: XamlServer, private readonly moveFailed: (from: vscode.Uri) => void) {}

    async open(uri: vscode.Uri | undefined): Promise<void> {
        if (!uri) throw new Error('Choose a project XAML file to move.');
        const folder = await requireXaml(uri);
        const destination = await vscode.window.showInputBox({
            title: 'Move XAML File', prompt: 'Destination path relative to the workspace folder',
            value: path.relative(folder.uri.fsPath, uri.fsPath).replaceAll(path.sep, '/'),
            validateInput: value => value.trim().toLowerCase().endsWith('.xaml') ? undefined : 'Enter a destination .xaml file.',
        });
        if (destination === undefined) return;
        await this.reviewMove(uri, destination.trim());
    }

    async reviewMove(uri: vscode.Uri, destination: string): Promise<void> {
        const folder = await requireXaml(uri);
        const to = await newWorkspaceFile(destination, folder.uri.toString());
        const plan = await this.plan(uri, to);
        await this.review.open(plan, validate => this.apply(uri, to, plan.review, undefined, validate).then(() => to));
    }

    async agent(request: FileMoveRequest, signal: AbortSignal, review?: string): Promise<FileMovePlan> {
        const from = await resolveWorkspaceFile(request.path, request.folder);
        const folder = await requireXaml(from);
        const to = await newWorkspaceFile(request.destination, folder.uri.toString());
        signal.throwIfAborted();
        const result = review === undefined ? await this.plan(from, to) : await this.apply(from, to, review, signal);
        signal.throwIfAborted();
        return result;
    }

    private async plan(from: vscode.Uri, to: vscode.Uri): Promise<FileMovePlan> {
        await requireXaml(from);
        const open = vscode.workspace.textDocuments.filter(document => document.uri.path.toLowerCase().endsWith('.rs'));
        const projectFiles = await Promise.all(open.map(document => isWorkspaceFile(document.uri)));
        const drafts = open.filter((document, index) => projectFiles[index] && !document.isClosed).map(document => ({
            uri: document.uri.toString(), version: document.version, source: document.getText(),
        }));
        if (Buffer.byteLength(JSON.stringify(drafts)) > 12 * 1024 * 1024) throw new Error('Close unused Rust buffers before reviewing this move.');
        return this.server.planFileMove(from, to, drafts);
    }

    private async apply(from: vscode.Uri, to: vscode.Uri, review: string, signal?: AbortSignal, validate?: () => void): Promise<FileMovePlan> {
        const plan = await this.plan(from, to);
        if (plan.review !== review) throw new Error('The file move changed. Review the current source patches before applying it.');
        const [actual, parent, identity] = await Promise.all([
            realpath(from.fsPath), realpath(path.dirname(to.fsPath)), lstat(from.fsPath, { bigint: true }),
        ]);
        if (!samePath(actual, vscode.Uri.parse(plan.from, true).fsPath)
            || !samePath(path.join(parent, path.basename(to.fsPath)), vscode.Uri.parse(plan.to, true).fsPath)) {
            throw new Error('The file or destination moved. Prepare the review again.');
        }
        const sources = await Promise.all(plan.documents.map(document => checkedSource(document, document.edit.uri === plan.from ? from : undefined)));
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        const relocated = sources.find(source => source.uri.toString() === from.toString());
        if (!relocated) throw new Error('The file moved. Prepare the review again.');
        // Keep rebasing on the original text model so repeated history follows the file.
        for (const patch of relocated.edits) edit.replace(from, patch.range, patch.newText);
        edit.renameFile(from, to);
        for (const source of sources) {
            if (source === relocated) continue;
            for (const patch of source.edits) edit.replace(source.uri, patch.range, patch.newText);
        }
        const applied = await this.server.withReviewedFileMove(from, to, () => vscode.workspace.applyEdit(edit));
        if (!applied) {
            this.moveFailed(from);
            const recovery = await recoverFileMove(from, to, identity, sources);
            if (recovery) {
                const restore = () => vscode.workspace.applyEdit(recovery.edit);
                const restored = recovery.moved ? await this.server.withReviewedFileMove(to, from, restore) : await restore();
                if (!restored) throw new Error('VS Code could not restore the file move. Check the affected sources before retrying.');
            }
            throw new Error('VS Code could not apply the reviewed file move. Review the files and destination before retrying.');
        }
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

async function requireXaml(uri: vscode.Uri): Promise<vscode.WorkspaceFolder> {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (!vscode.workspace.isTrusted || !folder || !uri.path.toLowerCase().endsWith('.xaml') || !await isWorkspaceFile(uri)) {
        throw new Error('Choose an authored XAML file in a trusted workspace.');
    }
    return folder;
}

async function checkedSource(plan: FileMoveDocument, preferred?: vscode.Uri) {
    const physical = vscode.Uri.parse(plan.edit.uri, true);
    const uri = preferred ?? await workspaceSourceUri(physical);
    if (!uri) throw new Error('A file in the review is no longer in the workspace.');
    const find = () => vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
    const live = find();
    const source = live?.getText() ?? (await readFile(uri.fsPath, 'utf8'));
    if (source !== plan.before || (live && live.version !== plan.edit.version) || (!live && plan.edit.version >= 0)) {
        throw new Error(`The source changed. Review ${path.basename(uri.fsPath)} again.`);
    }
    const stripBom = !live && source.startsWith('\uFEFF');
    const edits = plan.edit.edits.map(patch => {
        const range = asRange(patch.range);
        const normalize = (point: vscode.Position) => stripBom && point.line === 0 ? point.translate(0, -1) : point;
        return new vscode.TextEdit(new vscode.Range(normalize(range.start), normalize(range.end)), patch.newText);
    });
    const withoutBom = (text: string) => stripBom ? text.slice(1) : text;
    return { uri, edits, before: withoutBom(plan.before), after: withoutBom(plan.after), validate: () => {
        if (find() !== live || (live && live.version !== plan.edit.version)) {
            throw new Error(`The source changed. Review ${path.basename(uri.fsPath)} again.`);
        }
    } };
}

function samePath(first: string, second: string): boolean { return path.relative(first, second) === ''; }
