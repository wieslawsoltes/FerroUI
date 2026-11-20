import { lstat, realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { FileMovePlan } from './fileMoveProtocol';
import { checkedSources } from './reviewedSources';
import { isWorkspaceFile } from './workspacePaths';

export async function fileMoveDrafts(select: (document: vscode.TextDocument) => boolean) {
    const open = vscode.workspace.textDocuments.filter(select);
    const projectFiles = await Promise.all(open.map(document => isWorkspaceFile(document.uri)));
    const drafts = open.filter((document, index) => projectFiles[index] && !document.isClosed).map(document => ({
        uri: document.uri.toString(), version: document.version, source: document.getText(),
    }));
    if (Buffer.byteLength(JSON.stringify(drafts)) > 12 * 1024 * 1024) throw new Error('Save larger drafts before reviewing this move.');
    return drafts;
}

export async function checkedFileMove(from: vscode.Uri, to: vscode.Uri, plan: FileMovePlan) {
    const [actual, parent, identity] = await Promise.all([
        realpath(from.fsPath), realpath(path.dirname(to.fsPath)), lstat(from.fsPath, { bigint: true }),
    ]);
    if (path.relative(actual, vscode.Uri.parse(plan.from, true).fsPath)
        || path.relative(path.join(parent, path.basename(to.fsPath)), vscode.Uri.parse(plan.to, true).fsPath)) {
        throw new Error('The file or destination moved. Prepare the review again.');
    }
    const sources = await checkedSources(plan.documents, { from: plan.from, source: from });
    for (const source of sources) source.validate();
    return { identity, sources };
}
