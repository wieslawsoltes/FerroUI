import { createHash } from 'node:crypto';
import { open, realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { assetFilters, assetFormat, isAssetLocation } from './assetFormats';
import { isInside, localFolders, newWorkspaceFile, resolveWorkspaceFile } from './workspacePaths';

export interface AssetImportRequest { source: string; destination: string; folder?: string }

export class AssetImport {
    async choose(current?: vscode.WorkspaceFolder): Promise<void> {
        const folder = await chooseFolder(current);
        if (!folder) return;
        const [source] = await vscode.window.showOpenDialog({ title: 'Import FerroUI Asset', filters: assetFilters,
            canSelectFiles: true, canSelectMany: false, openLabel: 'Choose Asset' }) ?? [];
        if (!source) return;
        const name = path.basename(source.fsPath);
        const destination = await vscode.window.showInputBox({
            title: 'Import FerroUI Asset', prompt: `Destination relative to ${folder.name}`, value: `Assets/${name}`,
        });
        if (destination === undefined) return;
        const targetPath = destination.trim();
        const { plan } = await this.prepare(folder, source, targetPath, true);
        const choice = await vscode.window.showInformationMessage(`Import ${name}?`, {
            modal: true, detail: `${source.fsPath}\n→ ${folder.name}/${plan.destination}\n${plan.kind} · ${plan.bytes.toLocaleString()} bytes`,
        }, 'Import');
        if (choice !== 'Import') return;
        const imported = await this.apply(folder, source, targetPath, true, plan.review);
        void vscode.window.showInformationMessage(`Imported ${imported.destination}.`);
    }

    async agent(request: AssetImportRequest, review: string | undefined, signal: AbortSignal) {
        const source = await resolveWorkspaceFile(request.source, request.folder);
        const folder = vscode.workspace.getWorkspaceFolder(source);
        if (!folder) throw new Error('Choose an asset in a workspace folder.');
        signal.throwIfAborted();
        if (review !== undefined) return this.apply(folder, source, request.destination, false, review, signal);
        const { plan } = await this.prepare(folder, source, request.destination, false);
        signal.throwIfAborted();
        return plan;
    }

    private async prepare(folder: vscode.WorkspaceFolder, source: vscode.Uri, destination: string, allowExternal: boolean) {
        if (!vscode.workspace.isTrusted || source.scheme !== 'file') throw new Error('Open a trusted local workspace to import assets.');
        const format = assetFormat(source.fsPath);
        if (!format) throw new Error('Choose a supported image, font, media file, or XAML asset.');
        if (path.isAbsolute(destination) || path.extname(destination).toLowerCase() !== format.extension) {
            throw new Error(`Choose a workspace-relative destination ending in ${format.extension}.`);
        }
        const target = await newWorkspaceFile(destination, folder.uri.toString());
        const relative = path.relative(folder.uri.fsPath, target.fsPath);
        if (!isAssetLocation(relative)) throw new Error('Choose a destination in the project Assets index.');
        const actual = await realpath(source.fsPath);
        if (!allowExternal && !isInside(await realpath(folder.uri.fsPath), actual)) throw new Error('The selected asset is outside the workspace.');
        if (vscode.workspace.textDocuments.some(document => document.isDirty && document.uri.toString() === source.toString())) {
            throw new Error('Save the source asset before importing a copy.');
        }
        const file = await open(actual, 'r');
        try {
            const snapshot = await file.stat({ bigint: true });
            if (!snapshot.isFile()) throw new Error('Choose a regular file to import.');
            const bytes = await file.readFile();
            const current = await file.stat({ bigint: true });
            if (snapshot.size !== BigInt(bytes.length) || snapshot.mtimeNs !== current.mtimeNs || snapshot.ctimeNs !== current.ctimeNs) {
                throw new Error('The asset changed while reading it. Prepare the import again.');
            }
            const review = createHash('sha256').update(JSON.stringify([actual, target.toString()])).update(bytes).digest('hex');
            return { target, bytes, plan: { folder: folder.uri.toString(), source: source.toString(),
                destination: relative.split(path.sep).join('/'), kind: format.kind, bytes: bytes.length, review } };
        } finally { await file.close(); }
    }

    private async apply(folder: vscode.WorkspaceFolder, source: vscode.Uri, destination: string, allowExternal: boolean, review: string, signal?: AbortSignal) {
        const { target, bytes, plan } = await this.prepare(folder, source, destination, allowExternal);
        if (plan.review !== review) throw new Error('The asset changed. Review the current source and destination before importing it.');
        // Recheck the destination after reading a potentially large source file.
        await newWorkspaceFile(plan.destination, plan.folder);
        signal?.throwIfAborted();
        const edit = new vscode.WorkspaceEdit();
        edit.createFile(target, { contents: bytes }, { label: `Import ${path.basename(target.fsPath)}`, needsConfirmation: false });
        if (!await vscode.workspace.applyEdit(edit)) throw new Error('VS Code could not create the asset. Check its destination before retrying.');
        return plan;
    }
}

async function chooseFolder(current?: vscode.WorkspaceFolder): Promise<vscode.WorkspaceFolder | undefined> {
    const folders = localFolders();
    if (!folders.length) throw new Error('Open a local workspace folder to import assets.');
    if (current?.uri.scheme === 'file') return current;
    if (folders.length === 1) return folders[0];
    const target = await vscode.window.showQuickPick(folders.map(folder => ({ label: folder.name, description: folder.uri.fsPath, folder })), {
        title: 'Import Asset into Workspace Folder',
    });
    return target?.folder;
}
