import { open, realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { assetFiles, assetFormat, ignoredAssets } from './assetFormats';
import { isInside } from './workspacePaths';
import type { ProjectAsset } from './protocol';

const maxThumbnailBytes = 1024 * 1024;

interface IndexEntry {
    uri: vscode.Uri;
    title: string;
    path: string;
    kind: string;
    mime?: string;
}

export class ProjectAssets implements vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private readonly index = new Map<string, Promise<IndexEntry[]>>();
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;

    constructor() {
        const invalidate = () => { this.index.clear(); this.changed.fire(); };
        const watcher = vscode.workspace.createFileSystemWatcher(assetFiles, false, true, false);
        this.subscriptions = [watcher, watcher.onDidCreate(invalidate), watcher.onDidDelete(invalidate),
            vscode.workspace.onDidChangeWorkspaceFolders(invalidate)];
    }

    async list(folder: vscode.WorkspaceFolder, document?: vscode.Uri, canPlace = false): Promise<ProjectAsset[]> {
        const entries = await this.scan(folder);
        return entries.filter(asset => asset.uri.toString() !== document?.toString()).map(asset => {
            const relative = document ? relativeTo(document, asset.uri) : asset.path;
            return {
                id: asset.path, label: asset.title, path: asset.path, kind: asset.kind, relative,
                thumbnail: Boolean(asset.mime),
                source: canPlace && asset.mime && relative ? relative.split('/').map(encodeURIComponent).join('/') : undefined,
            };
        });
    }

    async resolve(folder: vscode.WorkspaceFolder, id: string): Promise<IndexEntry> {
        const asset = (await this.scan(folder)).find(asset => asset.path === id);
        if (!asset) throw new Error('Choose a file in the current project Assets view.');
        const [root, file] = await Promise.all([realpath(folder.uri.fsPath), realpath(asset.uri.fsPath)]);
        if (!isInside(root, file)) throw new Error('The asset resolves outside the workspace.');
        return { ...asset, uri: vscode.Uri.file(file) };
    }

    async thumbnail(folder: vscode.WorkspaceFolder, id: string): Promise<string | undefined> {
        const asset = await this.resolve(folder, id);
        if (!asset.mime) return undefined;
        const file = await open(asset.uri.fsPath, 'r');
        try {
            const stat = await file.stat();
            if (!stat.isFile() || stat.size <= 0 || stat.size > maxThumbnailBytes) return undefined;
            const buffer = Buffer.alloc(stat.size);
            const { bytesRead } = await file.read(buffer, 0, stat.size, 0);
            return `data:${asset.mime};base64,${buffer.subarray(0, bytesRead).toString('base64')}`;
        } finally { await file.close(); }
    }

    async imageSource(document: vscode.Uri, id: string): Promise<string> {
        const folder = vscode.workspace.getWorkspaceFolder(document);
        if (!folder || folder.uri.scheme !== 'file') throw new Error('Open a local workspace to place a project image.');
        const asset = await this.resolve(folder, id);
        if (!asset.mime) throw new Error('Choose a raster image from the project Assets view.');
        const directory = await realpath(path.dirname(document.fsPath));
        const root = await realpath(folder.uri.fsPath);
        if (!isInside(root, directory) && root !== directory) throw new Error('The design resolves outside the workspace.');
        const selected = vscode.Uri.file(path.resolve(folder.uri.fsPath, asset.path));
        return relativeTo(document, selected).split('/').map(encodeURIComponent).join('/');
    }

    private scan(folder: vscode.WorkspaceFolder): Promise<IndexEntry[]> {
        const key = folder.uri.toString();
        const cached = this.index.get(key);
        if (cached) return cached;
        const result = this.find(folder).catch(error => { this.index.delete(key); throw error; });
        this.index.set(key, result);
        return result;
    }

    private async find(folder: vscode.WorkspaceFolder): Promise<IndexEntry[]> {
        if (folder.uri.scheme !== 'file') return [];
        const root = await realpath(folder.uri.fsPath);
        const files = await vscode.workspace.findFiles(new vscode.RelativePattern(folder, assetFiles), ignoredAssets);
        const entries = await Promise.all(files.map(async (uri): Promise<IndexEntry | undefined> => {
            const actual = await realpath(uri.fsPath).catch(() => undefined);
            if (!actual || !isInside(root, actual)) return undefined;
            const format = assetFormat(uri.fsPath);
            if (!format) return undefined;
            return { uri, title: path.basename(uri.fsPath), path: path.relative(folder.uri.fsPath, uri.fsPath).split(path.sep).join('/'),
                kind: format.kind, mime: format.mime };
        }));
        return entries.filter((asset): asset is IndexEntry => asset !== undefined).sort((a, b) => a.path.localeCompare(b.path));
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}

function relativeTo(document: vscode.Uri, file: vscode.Uri): string {
    return path.relative(path.dirname(document.fsPath), file.fsPath).split(path.sep).join('/');
}
