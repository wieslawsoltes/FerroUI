import * as path from 'node:path';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { Range } from './protocol';

export interface IncludedSource {
    uri: string;
    range: Range;
    sourceStamp: string;
}

export async function includedSourceUri(consumer: vscode.Uri, target: IncludedSource): Promise<vscode.Uri> {
    return previewSourceUri(consumer, target.uri);
}

export async function previewSourceUri(consumer: vscode.Uri, address: string): Promise<vscode.Uri> {
    const folder = vscode.workspace.getWorkspaceFolder(consumer);
    const uri = vscode.Uri.parse(address, true);
    if (folder?.uri.scheme !== 'file' || uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.xaml')) {
        throw new Error('Open the workspace containing the included XAML.');
    }
    const [root, file] = await Promise.all([realpath(folder.uri.fsPath), realpath(uri.fsPath)]);
    const relative = path.relative(root, file);
    if (!relative || path.isAbsolute(relative) || relative === '..' || relative.startsWith(`..${path.sep}`)) {
        throw new Error('The included XAML source is outside the preview workspace.');
    }
    return vscode.Uri.file(path.join(folder.uri.fsPath, relative));
}
