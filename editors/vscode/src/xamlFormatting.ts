import * as vscode from 'vscode';
import { DocumentFormattingRequest, DocumentRangeFormattingRequest, type LanguageClient } from 'vscode-languageclient/node';

export async function xamlFormattingEdits(client: LanguageClient, document: vscode.TextDocument,
    editorOptions: vscode.FormattingOptions, token: vscode.CancellationToken, range?: vscode.Range): Promise<vscode.TextEdit[] | undefined> {
    const version = document.version;
    const style = vscode.workspace.getConfiguration('ferroui.xaml.formatting', document);
    const files = vscode.workspace.getConfiguration('files', document);
    const options = {
        attributeLayout: style.get('attributeLayout', 'preserve'),
        wrapColumn: style.get('wrapColumn', 100),
        closingBracketOnNewLine: style.get('closingBracketOnNewLine', false),
        selfClosingSpace: style.get('selfClosingSpace', 'preserve'),
        preserveBlankLines: style.get('preserveBlankLines', false),
        trimTrailingWhitespace: files.get('trimTrailingWhitespace', false),
        insertFinalNewline: files.get('insertFinalNewline', false),
        trimFinalNewlines: files.get('trimFinalNewlines', false),
        ...editorOptions,
    };
    const textDocument = client.code2ProtocolConverter.asTextDocumentIdentifier(document);
    try {
        const edits = range
            ? await client.sendRequest(DocumentRangeFormattingRequest.type, {
                textDocument, range: client.code2ProtocolConverter.asRange(range), options,
            }, token)
            : await client.sendRequest(DocumentFormattingRequest.type, { textDocument, options }, token);
        const formatted = await client.protocol2CodeConverter.asTextEdits(edits, token);
        return token.isCancellationRequested || document.version !== version ? undefined : formatted;
    } catch (error) {
        const request = range ? DocumentRangeFormattingRequest.type : DocumentFormattingRequest.type;
        return client.handleFailedRequest(request, token, error, undefined);
    }
}
