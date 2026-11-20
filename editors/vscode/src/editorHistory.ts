import * as vscode from 'vscode';

export async function runEditorHistory(
    document: vscode.TextDocument,
    action: 'undo' | 'redo',
    requireCurrent: () => void,
    signal: AbortSignal,
): Promise<void> {
    await vscode.window.showTextDocument(document, { viewColumn: sourceColumn(document), preserveFocus: false });
    await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
    signal.throwIfAborted();
    requireCurrent();
    if (document.isClosed || vscode.window.activeTextEditor?.document !== document) {
        throw new Error('The authored editor lost focus.');
    }
    const version = document.version;
    await vscode.commands.executeCommand(action);
    if (!document.isClosed && document.version === version) {
        throw new Error(`VS Code did not ${action} an edit in the authored source.`);
    }
}

function sourceColumn(document: vscode.TextDocument): vscode.ViewColumn | undefined {
    // A Design or diff tab can own the active group while VS Code still reports the last source editor.
    const active = vscode.window.activeTextEditor;
    const visible = active?.document === document ? active : vscode.window.visibleTextEditors.find(editor => editor.document === document);
    if (visible?.viewColumn !== undefined) return visible.viewColumn;
    const uri = document.uri.toString();
    return vscode.window.tabGroups.all.find(group => group.tabs.some(tab =>
        tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri))?.viewColumn;
}
