import * as path from 'node:path';
import * as vscode from 'vscode';
import type { PreviewSourceBuffer } from './protocol';

export function xamlPath(uri: vscode.Uri): string | undefined {
    if (uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.xaml')) return;
    return fileKey(uri);
}

function previewPath(uri: vscode.Uri, rust: boolean): string | undefined {
    if (!rust || uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.rs')) return xamlPath(uri);
    return fileKey(uri);
}

function fileKey(uri: vscode.Uri): string {
    return process.platform === 'win32' ? uri.fsPath.toLowerCase() : uri.fsPath;
}

export class DesignerDependencies implements vscode.Disposable {
    private watched = new Set<string>();
    private duringRender?: Set<string>;
    private readonly assetWatchers = new Map<string, vscode.FileSystemWatcher>();

    constructor(private readonly rust = false, private readonly assetChanged?: () => void) {}

    change(uri: vscode.Uri): boolean {
        const key = this.assetChanged && uri.scheme === 'file' ? fileKey(uri) : previewPath(uri, this.rust);
        if (!key) return false;
        this.duringRender?.add(key);
        return this.watched.has(key);
    }

    start(): void { this.duringRender = new Set(); }

    accept(uris: string[]): boolean {
        const assetFolders = new Map<string, vscode.Uri>();
        const watched = new Set<string>();
        for (const value of uris) {
            const uri = vscode.Uri.parse(value, true);
            const source = previewPath(uri, this.rust);
            if (source) watched.add(source);
            else if (this.assetChanged && uri.scheme === 'file') {
                watched.add(fileKey(uri));
                const folder = vscode.Uri.file(path.dirname(uri.fsPath));
                assetFolders.set(fileKey(folder), folder);
            }
        }
        this.watched = watched;
        this.watchAssetFolders(assetFolders);
        return [...this.watched].some(key => this.duringRender?.has(key));
    }

    stop(): void { this.duringRender = undefined; }

    dispose(): void {
        for (const watcher of this.assetWatchers.values()) watcher.dispose();
        this.assetWatchers.clear();
        this.watched.clear();
        this.stop();
    }

    private watchAssetFolders(folders: Map<string, vscode.Uri>): void {
        for (const [key, watcher] of this.assetWatchers) {
            if (folders.has(key)) continue;
            watcher.dispose();
            this.assetWatchers.delete(key);
        }
        for (const [key, folder] of folders) {
            if (this.assetWatchers.has(key)) continue;
            // The recursive watcher also supports volumes that VS Code's single-directory watcher refuses.
            const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(folder, '**/*'));
            const changed = (uri: vscode.Uri) => {
                if (this.change(uri)) this.assetChanged?.();
            };
            watcher.onDidCreate(changed);
            watcher.onDidChange(changed);
            watcher.onDidDelete(changed);
            this.assetWatchers.set(key, watcher);
        }
    }
}

export function dirtyPreviewSources(documents: readonly vscode.Uri[], exclude?: vscode.TextDocument, rust = false): PreviewSourceBuffer[] {
    const scope = (uri: vscode.Uri) => vscode.workspace.getWorkspaceFolder(uri)?.uri.toString() ?? path.dirname(uri.fsPath);
    const roots = new Set(documents.map(scope));
    let remaining = 8 * 1024 * 1024;
    return vscode.workspace.textDocuments.flatMap<PreviewSourceBuffer>(candidate => {
        if (candidate === exclude || !candidate.isDirty || !previewPath(candidate.uri, rust) || !roots.has(scope(candidate.uri))) return [];
        const input = { uri: candidate.uri.toString(), source: candidate.getText() };
        const length = Buffer.byteLength(JSON.stringify(input));
        // Preserve the unavailable identity when a draft exceeds the frame budget.
        if (length > remaining || Buffer.byteLength(input.source) > 1024 * 1024) return [{ uri: input.uri, source: null }];
        remaining -= length;
        return [input];
    });
}
