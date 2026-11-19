import * as vscode from 'vscode';

export async function runEditorHistory(
    document: vscode.TextDocument,
    action: 'undo' | 'redo',
    requireCurrent: () => void,
    signal: AbortSignal,
): Promise<void> {
    await vscode.window.showTextDocument(document, { preserveFocus: false });
    await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
    signal.throwIfAborted();
    requireCurrent();
    if (vscode.window.activeTextEditor?.document !== document) throw new Error('The authored editor lost focus.');
    await vscode.commands.executeCommand(action);
}
