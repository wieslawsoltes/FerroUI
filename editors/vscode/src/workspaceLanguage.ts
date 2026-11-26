import * as vscode from 'vscode';
import type { Position } from './protocol';
import { exactRange, wireRange } from './textDocument';

export interface LanguageQuery {
    action: 'symbols' | 'completion' | 'hover' | 'definition' | 'references' | 'rename_range';
    position?: Position;
    limit?: number;
}

const commands = {
    symbols: 'vscode.executeDocumentSymbolProvider',
    completion: 'vscode.executeCompletionItemProvider',
    hover: 'vscode.executeHoverProvider',
    definition: 'vscode.executeDefinitionProvider',
    references: 'vscode.executeReferenceProvider',
    rename_range: 'vscode.prepareRename',
} satisfies Record<LanguageQuery['action'], string>;

export async function queryLanguage(document: vscode.TextDocument, query: LanguageQuery): Promise<unknown> {
    const limit = query.limit ?? 40;
    const { action, position } = query;
    const command = commands[action];
    const uri = document.uri;
    if (action === 'symbols') {
        const symbols = await vscode.commands.executeCommand<(vscode.DocumentSymbol | vscode.SymbolInformation)[]>(command, uri);
        const items = [];
        for (const symbol of symbols ?? []) {
            for (const row of symbolRows(symbol, [])) {
                items.push(row);
                if (items.length === limit) return { items };
            }
        }
        return { items };
    }
    if (!position) throw new Error('Supply a source position for this language query.');
    const at = exactRange(document, { start: position, end: position }).start;
    if (action === 'completion') {
        const completion = await vscode.commands.executeCommand<vscode.CompletionList>(command, uri, at);
        const items = completion?.items ?? [];
        return { incomplete: Boolean(completion?.isIncomplete) || items.length > limit, items: items.slice(0, limit).map(item => {
            const insertText = item.insertText instanceof vscode.SnippetString ? item.insertText.value : item.insertText;
            const range = item.range instanceof vscode.Range ? { replace: wireRange(item.range) } : item.range && {
                insert: wireRange(item.range.inserting), replace: wireRange(item.range.replacing),
            };
            return { label: item.label, kind: item.kind, detail: item.detail, insertText, range,
                snippet: item.insertText instanceof vscode.SnippetString };
        }) };
    }
    if (action === 'hover') {
        const results = await vscode.commands.executeCommand<vscode.Hover[]>(command, uri, at);
        return { items: (results ?? []).slice(0, limit).map(hover => ({
            range: hover.range && wireRange(hover.range),
            contents: hover.contents.map(part => typeof part === 'string' ? part : part.value).join('\n\n').slice(0, 8000),
        })) };
    }
    if (action === 'rename_range') {
        const result = await vscode.commands.executeCommand<{ range: vscode.Range; placeholder: string }>(command, uri, at);
        return { range: result && wireRange(result.range), placeholder: result?.placeholder };
    }
    const locations = await vscode.commands.executeCommand<(vscode.Location | vscode.LocationLink)[]>(command, uri, at);
    return { items: (locations ?? []).slice(0, limit).map(location => 'targetUri' in location
        ? { uri: location.targetUri.toString(), range: wireRange(location.targetSelectionRange ?? location.targetRange) }
        : { uri: location.uri.toString(), range: wireRange(location.range) }) };
}

interface SymbolRow { name: string; kind: vscode.SymbolKind; parents: string[]; range: ReturnType<typeof wireRange>; uri?: string }

function* symbolRows(symbol: vscode.DocumentSymbol | vscode.SymbolInformation, parents: string[]): Generator<SymbolRow> {
    const location = 'location' in symbol ? symbol.location : undefined;
    if ('selectionRange' in symbol && 'children' in symbol) {
        yield { name: symbol.name, kind: symbol.kind, parents, range: wireRange(symbol.selectionRange), uri: location?.uri.toString() };
        for (const child of symbol.children) yield* symbolRows(child, [...parents, symbol.name]);
    } else if ('containerName' in symbol && location) {
        yield { name: symbol.name, kind: symbol.kind, parents: symbol.containerName ? [symbol.containerName] : [],
            range: wireRange(location.range), uri: location.uri.toString() };
    }
}
