import { lstat } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { assetFormat, isAssetLocation } from './assetFormats';
import { recoverFileMove } from './fileMoveRecovery';
import { isCompiledXamlRust } from './fileMoveRust';
import { checkedFileMove, fileMoveDrafts } from './fileMoveSources';
import type { FileMoveRequest } from './fileMoves';
import { SourceChangeReview } from './sourceChangeReview';
import { isWorkspaceFile, newWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';
import type { XamlServer } from './xamlServer';

export class AssetMoves implements vscode.Disposable {
    private readonly review = new SourceChangeReview({
        scheme: 'ferroui-asset-move-review', context: 'ferroui.reviewingAssetMove', command: 'ferroui.assetMoveReview',
        reveal: (uri, viewColumn) => vscode.commands.executeCommand('vscode.open', uri, { viewColumn, preview: false }),
    });

    constructor(private readonly server: XamlServer) {}

    async open(from: vscode.Uri | undefined): Promise<void> {
        if (!from) throw new Error('Choose an image, font, or media file in Assets.');
        const { folder, extension } = await assetInWorkspace(from);
        const display = (uri: vscode.Uri) => vscode.workspace.asRelativePath(uri, false);
        const destination = await vscode.window.showInputBox({
            title: 'Move FerroUI Asset', prompt: `Destination relative to ${folder.name}`, value: display(from),
            validateInput: input => path.extname(input.trim()).toLowerCase() === extension ? undefined : `Use a ${extension} destination.`,
        });
        if (destination === undefined) return;
        const to = await assetDestination(destination.trim(), folder, extension);
        const plan = await this.plan(from, to);
        const title = `Move ${display(from)} → ${display(to)}`;
        if (plan.documents.length) {
            await this.review.open(title, plan.documents, async validate => {
                await this.apply(from, to, plan.review, undefined, validate);
                return to;
            });
        } else {
            const choice = await vscode.window.showInformationMessage(title, { modal: true,
                detail: 'This move affects the asset file. The index found no authored path references.' }, 'Move');
            if (choice === 'Move') {
                await this.apply(from, to, plan.review);
                await vscode.commands.executeCommand('vscode.open', to);
            }
        }
    }

    async agent(request: FileMoveRequest, review: string | undefined, signal: AbortSignal) {
        signal.throwIfAborted();
        const from = await resolveWorkspaceFile(request.path, request.folder);
        const { folder, extension } = await assetInWorkspace(from);
        const to = await assetDestination(request.destination, folder, extension);
        if (review !== undefined) return this.apply(from, to, review, signal);
        const plan = await this.plan(from, to);
        signal.throwIfAborted();
        return plan;
    }

    private async plan(from: vscode.Uri, to: vscode.Uri) {
        const drafts = await fileMoveDrafts(document => document.uri.path.toLowerCase().endsWith('.rs') && !isCompiledXamlRust(document));
        return this.server.planAssetMove(from, to, drafts);
    }

    private async apply(from: vscode.Uri, to: vscode.Uri, review: string, signal?: AbortSignal, validate?: () => void) {
        const original = await lstat(from.fsPath, { bigint: true });
        const plan = await this.plan(from, to);
        if (plan.review !== review) throw new Error('The asset move changed. Review the current file and reference patches before applying it.');
        const { identity, sources } = await checkedFileMove(from, to, plan);
        const current = await lstat(from.fsPath, { bigint: true });
        if (original.dev !== current.dev || original.ino !== current.ino || original.size !== current.size
            || original.mtimeNs !== current.mtimeNs || original.ctimeNs !== current.ctimeNs) {
            throw new Error('The asset changed during review. Prepare the move again.');
        }
        for (const source of sources) source.validate();
        validate?.();
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        edit.renameFile(from, to);
        for (const source of sources) {
            for (const change of source.edits) edit.replace(source.uri, change.range, change.newText);
        }
        if (!await vscode.workspace.applyEdit(edit)) {
            const recovery = await recoverFileMove(from, to, identity, sources);
            if (recovery && !await vscode.workspace.applyEdit(recovery.edit)) {
                throw new Error('VS Code could not restore the asset move. Check the affected files before retrying.');
            }
            throw new Error('VS Code could not apply the reviewed asset move. Review the files and destination before retrying.');
        }
        return plan;
    }

    dispose(): void { this.review.dispose(); }
}

async function assetInWorkspace(uri: vscode.Uri) {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    const format = assetFormat(uri.fsPath);
    if (!vscode.workspace.isTrusted || !folder || !format || format.kind === 'XAML' || !await isWorkspaceFile(uri)) {
        throw new Error('Choose an image, font, or media file in a trusted workspace.');
    }
    return { folder, extension: format.extension };
}

async function assetDestination(destination: string, folder: vscode.WorkspaceFolder, extension: string) {
    if (path.isAbsolute(destination) || path.extname(destination).toLowerCase() !== extension) {
        throw new Error(`Choose a workspace-relative destination ending in ${extension}.`);
    }
    const target = await newWorkspaceFile(destination, folder.uri.toString());
    if (!isAssetLocation(path.relative(folder.uri.fsPath, target.fsPath))) {
        throw new Error('Choose a destination in the project Assets index.');
    }
    return target;
}
