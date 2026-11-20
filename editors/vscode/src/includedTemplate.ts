import * as path from 'node:path';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { Range } from './protocol';

export interface IncludedTemplateSource {
    uri: string;
    range: Range;
    key: string;
    sourceStamp: string;
}

export interface IncludedTemplatePreview {
    source: IncludedTemplateSource;
    sample: unknown;
}

export async function includedTemplateUri(consumer: vscode.Uri, target: IncludedTemplateSource): Promise<vscode.Uri> {
    const folder = vscode.workspace.getWorkspaceFolder(consumer);
    const uri = vscode.Uri.parse(target.uri, true);
    if (folder?.uri.scheme !== 'file' || uri.scheme !== 'file' || !uri.path.toLowerCase().endsWith('.xaml')) {
        throw new Error('Open the workspace containing the included template.');
    }
    const [root, file] = await Promise.all([realpath(folder.uri.fsPath), realpath(uri.fsPath)]);
    const relative = path.relative(root, file);
    if (!relative || path.isAbsolute(relative) || relative === '..' || relative.startsWith(`..${path.sep}`)) {
        throw new Error('The template source is outside the preview workspace.');
    }
    return vscode.Uri.file(path.join(folder.uri.fsPath, relative));
}
