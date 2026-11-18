import * as path from 'node:path';
import * as vscode from 'vscode';
import type { PreviewSourceBuffer } from './protocol';

export function xamlPath(uri: vscode.Uri): string | undefined {
    if (uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.xaml')) return;
    return process.platform === 'win32' ? uri.fsPath.toLowerCase() : uri.fsPath;
}

export class DesignerDependencies {
    private watched = new Set<string>();
    private duringRender?: Set<string>;

    change(uri: vscode.Uri): boolean {
        const key = xamlPath(uri);
        if (!key) return false;
        this.duringRender?.add(key);
        return this.watched.has(key);
    }

    start(): void { this.duringRender = new Set(); }

    accept(uris: string[]): boolean {
        this.watched = new Set(uris.flatMap(value => xamlPath(vscode.Uri.parse(value, true)) ?? []));
        return [...this.watched].some(key => this.duringRender?.has(key));
    }

    stop(): void { this.duringRender = undefined; }
}

export function dirtyXamlSources(document: vscode.TextDocument, folder?: vscode.WorkspaceFolder): PreviewSourceBuffer[] {
    const sourceFolder = folder?.uri.toString();
    const localDirectory = path.dirname(document.uri.fsPath);
    let remaining = 8 * 1024 * 1024;
    return vscode.workspace.textDocuments.flatMap<PreviewSourceBuffer>(candidate => {
        if (candidate === document || !candidate.isDirty || !xamlPath(candidate.uri)) return [];
        const parent = vscode.workspace.getWorkspaceFolder(candidate.uri);
        if (sourceFolder ? sourceFolder !== parent?.uri.toString() : localDirectory !== path.dirname(candidate.uri.fsPath)) return [];
        const input = { uri: candidate.uri.toString(), source: candidate.getText() };
        const length = Buffer.byteLength(JSON.stringify(input));
        // Preserve the unavailable identity when a draft exceeds the frame budget.
        if (length > remaining || Buffer.byteLength(input.source) > 1024 * 1024) return [{ uri: input.uri, source: null }];
        remaining -= length;
        return [input];
    });
}
