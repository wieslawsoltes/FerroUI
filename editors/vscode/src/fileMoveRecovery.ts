import type { BigIntStats } from 'node:fs';
import { lstat, readFile } from 'node:fs/promises';
import * as vscode from 'vscode';

interface MoveDraft {
    uri: vscode.Uri;
    before: string;
    after: string;
}

// A rejected mixed WorkspaceEdit can leave an earlier resource or text edit applied.
export async function recoverFileMove(from: vscode.Uri, to: vscode.Uri, identity: BigIntStats, drafts: readonly MoveDraft[]) {
    const [original, destination] = await Promise.all([fileEntry(from), fileEntry(to)]);
    const moved = !original && sameFile(destination, identity);
    if (!moved && !sameFile(original, identity)) {
        throw new Error('The file location changed during the move. Check the affected paths before retrying.');
    }
    const buffers = await Promise.all(drafts.filter(draft => draft.before !== draft.after).map(async draft => {
        const uri = moved && draft.uri.toString() === from.toString() ? to : draft.uri;
        const live = vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
        const text = live?.getText() ?? (await readFile(uri.fsPath, 'utf8')).replace(/^\uFEFF/, '');
        if (text === draft.before) return;
        const document = live ?? await vscode.workspace.openTextDocument(uri);
        return { draft, document, revision: document.version };
    }));
    const edit = new vscode.WorkspaceEdit();
    if (moved) edit.renameFile(to, from);
    for (const buffer of buffers) {
        if (!buffer) continue;
        const { draft, document, revision } = buffer;
        if (document.isClosed || document.version !== revision || document.getText() !== draft.after) {
            throw new Error('Source changed during the move. Check the affected edits before retrying.');
        }
        edit.replace(draft.uri, new vscode.Range(document.positionAt(0), document.positionAt(draft.after.length)), draft.before);
    }
    if (moved || edit.size) return { edit, moved };
}

async function fileEntry(uri: vscode.Uri): Promise<BigIntStats | undefined> {
    try { return await lstat(uri.fsPath, { bigint: true }); }
    catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }
}

function sameFile(entry: BigIntStats | undefined, original: BigIntStats): boolean {
    return !!entry && entry.isFile() && entry.dev === original.dev && entry.ino === original.ino;
}
