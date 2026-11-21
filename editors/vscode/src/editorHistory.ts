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
    let changed!: () => void;
    const update = new Promise<void>(resolve => { changed = resolve; });
    const listener = vscode.workspace.onDidChangeTextDocument(event => {
        if (event.document === document && event.contentChanges.length) changed();
    });
    const timeout = setTimeout(changed, 1_000);
    signal.addEventListener('abort', changed, { once: true });
    try {
        await vscode.commands.executeCommand(action);
        // VS Code can acknowledge the command before sending the version change to this host.
        if (!document.isClosed && document.version === version) await update;
        signal.throwIfAborted();
        if (!document.isClosed && document.version === version) {
            throw new Error(`VS Code did not ${action} an edit in the authored source.`);
        }
    } finally {
        listener.dispose();
        clearTimeout(timeout);
        signal.removeEventListener('abort', changed);
    }
}

export function sourceColumn(document: vscode.TextDocument): vscode.ViewColumn | undefined {
    // A Design or diff tab can own the active group while VS Code still reports the last source editor.
    const active = vscode.window.activeTextEditor;
    const visible = active?.document === document ? active : vscode.window.visibleTextEditors.find(editor => editor.document === document);
    if (visible?.viewColumn !== undefined) return visible.viewColumn;
    const uri = document.uri.toString();
    return vscode.window.tabGroups.all.find(group => group.tabs.some(tab =>
        tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri))?.viewColumn;
}
