import { pathToFileURL } from 'node:url';
import * as vscode from 'vscode';
import type { Language, Position, Range, Source, TextEdit } from './protocol';
import { xamlLanguage } from './xamlServer';

export function languageOf(document: vscode.TextDocument): Language | undefined {
    if (document.uri.scheme !== 'file') return undefined;
    if (document.languageId === xamlLanguage) return 'xaml';
    if (document.languageId === 'rust') return 'rust';
    return undefined;
}

export function sourceOf(document: vscode.TextDocument, language: Language, caret: vscode.Position): Source {
    return { language, uri: pathToFileURL(document.uri.fsPath).toString(), source: document.getText(), caret: position(caret) };
}

export function asPosition(pos: Position): vscode.Position {
    return new vscode.Position(pos.line, pos.character);
}

function position(pos: vscode.Position): Position {
    return { line: pos.line, character: pos.character };
}

export function wireRange(range: vscode.Range): Range {
    return { start: position(range.start), end: position(range.end) };
}

export function asRange(range: Range): vscode.Range {
    return new vscode.Range(asPosition(range.start), asPosition(range.end));
}

export function exactRange(document: vscode.TextDocument, range: Range): vscode.Range {
    const start = asPosition(range.start);
    const end = asPosition(range.end);
    const result = new vscode.Range(start, end);
    if (end.isBefore(start) || !document.validateRange(result).isEqual(result)) throw new Error('The requested source range is invalid.');
    return result;
}

export async function applyTextEdits(document: vscode.TextDocument, version: number, edits: TextEdit[]): Promise<void> {
    if (document.isClosed || document.version !== version) throw new Error('The source changed. Use the current preview to retry.');
    const workspaceEdit = new vscode.WorkspaceEdit();
    for (const edit of edits) {
        const range = asRange(edit.range);
        if (!document.validateRange(range).isEqual(range)) throw new Error('The preview returned an invalid source range.');
        workspaceEdit.replace(document.uri, range, edit.newText);
    }
    if (edits.length && !await vscode.workspace.applyEdit(workspaceEdit)) {
        throw new Error('VS Code could not apply the designer edit to the current document.');
    }
}
