import * as path from 'node:path';
import * as vscode from 'vscode';
import type { FileMovePlan } from './fileMoveProtocol';
import { recoverFileMove } from './fileMoveRecovery';
import { isCompiledXamlRust, resolveFileMoveRust } from './fileMoveRust';
import { checkedFileMove, fileMoveDrafts } from './fileMoveSources';
import { SourceChangeReview } from './sourceChangeReview';
import { isWorkspaceFile, newWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import type { XamlServer } from './xamlServer';

export interface FileMoveRequest { path: string; folder?: string; destination: string }

export class FileMoves implements vscode.Disposable {
    private readonly review = new SourceChangeReview({
        scheme: 'ferroui-file-move-review', context: 'ferroui.reviewingFileMove', command: 'ferroui.fileMoveReview',
    });

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
        const relative = (file: vscode.Uri) => vscode.workspace.asRelativePath(file, false);
        const documents = [...plan.documents].sort((left, right) => Number(right.edit.uri === plan.from) - Number(left.edit.uri === plan.from));
        await this.review.open(`Move ${relative(uri)} → ${relative(to)}`, documents,
            validate => this.apply(uri, to, plan.review, undefined, validate).then(() => to), { from: plan.from, to });
    }

    async agent(request: FileMoveRequest, signal: AbortSignal, review?: string): Promise<FileMovePlan> {
        const from = await resolveWorkspaceFile(request.path, request.folder);
        const folder = await requireXaml(from);
        const to = await newWorkspaceFile(request.destination, folder.uri.toString());
        signal.throwIfAborted();
        const result = review === undefined ? await this.plan(from, to, signal) : await this.apply(from, to, review, signal);
        signal.throwIfAborted();
        return result;
    }

    private async plan(from: vscode.Uri, to: vscode.Uri, signal?: AbortSignal): Promise<FileMovePlan> {
        await requireXaml(from);
        const plan = await this.server.planFileMove(from, to, await this.drafts());
        if (!plan.generatedRust?.length) return plan;
        const resolutions = await resolveFileMoveRust(plan.generatedRust, signal);
        return this.server.completeFileMove(from, to, await this.drafts(), plan.review, resolutions);
    }

    private async drafts() {
        return fileMoveDrafts(document => {
            // Opening a clean buffer during semantic rename does not change the
            // saved source snapshot that the server is reviewing.
            if (!document.isDirty) return false;
            const extension = path.extname(document.uri.path).toLowerCase();
            return (extension === '.rs' && !isCompiledXamlRust(document)) || extension === '.json' || extension === '.ferrostages';
        });
    }

    private async apply(from: vscode.Uri, to: vscode.Uri, review: string, signal?: AbortSignal, validate?: () => void): Promise<FileMovePlan> {
        const plan = await this.plan(from, to, signal);
        if (plan.review !== review) throw new Error('The file move changed. Review the current source patches before applying it.');
        const { identity, sources } = await checkedFileMove(from, to, plan);
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
