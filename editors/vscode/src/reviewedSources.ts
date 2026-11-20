import { readFile, realpath } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { TextEdit } from './protocol';
import { asRange } from './textDocument';
import { workspaceSourceUri } from './workspacePaths';

export interface ReviewedDocument {
    edit: { uri: string; version: number; edits: TextEdit[] };
    before: string;
    after: string;
}

async function openSources() {
    const documents = vscode.workspace.textDocuments.filter(document =>
        document.uri.scheme === 'file' && vscode.workspace.getWorkspaceFolder(document.uri));
    return Promise.all(documents.map(async document => ({
        document, file: await realpath(document.uri.fsPath).catch(() => undefined),
    })));
}

/** Match physical XAML/Rust identities to the buffer the workspace is editing. */
export async function reviewedSourceUri(physical: vscode.Uri, sources?: Awaited<ReturnType<typeof openSources>>): Promise<vscode.Uri | undefined> {
    const fallback = await workspaceSourceUri(physical);
    if (!fallback) return;
    const actual = await realpath(physical.fsPath);
    const open = sources ?? await openSources();
    const matches = open.filter(({ document, file }) => file && !document.isClosed && path.relative(file, actual) === '');
    if (matches.length > 1) throw new Error(`Close duplicate buffers for ${path.basename(physical.fsPath)} before applying a source review.`);
    return matches[0]?.document.uri ?? fallback;
}

export async function checkedSources(plans: readonly ReviewedDocument[], preferred?: { from: string; source: vscode.Uri }) {
    const open = await openSources();
    return Promise.all(plans.map(plan => checkedSource(plan, open,
        preferred?.from === plan.edit.uri ? preferred.source : undefined)));
}

async function checkedSource(plan: ReviewedDocument, open: Awaited<ReturnType<typeof openSources>>, preferred?: vscode.Uri) {
    const physical = vscode.Uri.parse(plan.edit.uri, true);
    const uri = preferred ?? await reviewedSourceUri(physical, open);
    if (!uri) throw new Error('A file in the review is no longer in the workspace.');
    const find = () => vscode.workspace.textDocuments.find(document => document.uri.toString() === uri.toString());
    const live = find();
    const source = live?.getText() ?? (await readFile(uri.fsPath, 'utf8'));
    const stripBom = plan.edit.version < 0 && plan.before.startsWith('\uFEFF');
    const withoutBom = (text: string) => stripBom && text.startsWith('\uFEFF') ? text.slice(1) : text;
    const expected = live ? withoutBom(plan.before) : plan.before;
    if (source !== expected || (plan.edit.version >= 0 && live?.version !== plan.edit.version)) {
        throw new Error(`The source changed. Review ${path.basename(uri.fsPath)} again.`);
    }
    const edits = plan.edit.edits.map(patch => {
        const range = asRange(patch.range);
        const normalize = (point: vscode.Position) => stripBom && point.line === 0 ? point.translate(0, -1) : point;
        return new vscode.TextEdit(new vscode.Range(normalize(range.start), normalize(range.end)), patch.newText);
    });
    const version = live?.version;
    return { uri, edits, before: withoutBom(plan.before), after: withoutBom(plan.after), validate: () => {
        if (find() !== live || live?.version !== version) {
            throw new Error(`The source changed. Review ${path.basename(uri.fsPath)} again.`);
        }
    } };
}
