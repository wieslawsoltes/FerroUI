import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import type { Range } from './protocol';
import { exactRange } from './textDocument';

/** Re-read only the Rust draft that supplies this definition. Standard LSP uses the saved file. */
export async function projectDefinition(client: LanguageClient, document: vscode.TextDocument,
    position: vscode.Position, token: vscode.CancellationToken): Promise<vscode.Location[] | undefined> {
    if (!client.initializeResult?.capabilities.experimental?.ferrouiProjectNavigation
        || token.isCancellationRequested) return;
    const drafts = vscode.workspace.textDocuments.filter(candidate =>
        candidate.isDirty && candidate.languageId === 'rust' && candidate.uri.scheme === 'file');
    if (!drafts.length) return;
    const version = document.version;
    const textDocument = { uri: document.uri.toString(), version };
    const params = { textDocument, position: { line: position.line, character: position.character } };
    const uri = await client.sendRequest<string | null>('ferroui/projectDefinitionTarget', params, token);
    if (!uri) return;
    const target = vscode.Uri.parse(uri, true);
    const physical = await realpath(target.fsPath).catch(() => undefined);
    if (!physical) return [];
    const paths = await Promise.all(drafts.map(candidate => realpath(candidate.uri.fsPath).catch(() => undefined)));
    const index = paths.findIndex(file => file === physical);
    if (index === -1) return;
    const draft = drafts[index];
    if (token.isCancellationRequested || draft.isClosed || document.isClosed || document.version !== version) return [];
    const rust = { uri: draft.uri.toString(), version: draft.version, source: draft.getText() };
    const locations = await client.sendRequest<Array<{ uri: string; range: Range }>>(
        'ferroui/projectDefinition', { ...params, rust }, token);
    if (token.isCancellationRequested || document.isClosed || document.version !== version
        || draft.isClosed || draft.version !== rust.version) return [];
    return locations.map(location => new vscode.Location(draft.uri, exactRange(draft, location.range)));
}
