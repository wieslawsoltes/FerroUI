import * as path from 'node:path';
import * as vscode from 'vscode';
import type { PreviewSourceBuffer } from './protocol';

export function xamlPath(uri: vscode.Uri): string | undefined {
    if (uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.xaml')) return;
    return process.platform === 'win32' ? uri.fsPath.toLowerCase() : uri.fsPath;
}

function previewPath(uri: vscode.Uri, rust: boolean): string | undefined {
    if (!rust || uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.rs')) return xamlPath(uri);
    return process.platform === 'win32' ? uri.fsPath.toLowerCase() : uri.fsPath;
}

export class DesignerDependencies {
    private watched = new Set<string>();
    private duringRender?: Set<string>;

    constructor(private readonly rust = false) {}

    change(uri: vscode.Uri): boolean {
        const key = previewPath(uri, this.rust);
        if (!key) return false;
        this.duringRender?.add(key);
        return this.watched.has(key);
    }

    start(): void { this.duringRender = new Set(); }

    accept(uris: string[]): boolean {
        this.watched = new Set(uris.flatMap(value => previewPath(vscode.Uri.parse(value, true), this.rust) ?? []));
        return [...this.watched].some(key => this.duringRender?.has(key));
    }

    stop(): void { this.duringRender = undefined; }
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
