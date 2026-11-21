import * as vscode from 'vscode';

let historyQueue = Promise.resolve();

export function runEditorHistory(
    document: vscode.TextDocument,
    action: 'undo' | 'redo',
    requireCurrent: () => void,
    signal: AbortSignal,
): Promise<void> {
    const operation = historyQueue.then(() => applyEditorHistory(document, action, requireCurrent, signal));
    historyQueue = operation.catch(() => {});
    return operation;
}

async function applyEditorHistory(
    document: vscode.TextDocument,
    action: 'undo' | 'redo',
    requireCurrent: () => void,
    signal: AbortSignal,
): Promise<void> {
    signal.throwIfAborted();
    requireCurrent();
    const editor = await vscode.window.showTextDocument(document, {
        viewColumn: sourceColumn(document), preserveFocus: false, preview: false,
    });
    signal.throwIfAborted();
    requireCurrent();
    const active = vscode.window.tabGroups.activeTabGroup.activeTab?.input;
    if (document.isClosed || vscode.window.activeTextEditor !== editor
        || !(active instanceof vscode.TabInputText) || active.uri.toString() !== document.uri.toString()) {
        throw new Error('The authored editor lost focus.');
    }
    const version = document.version;
    let changed!: () => void;
    const update = new Promise<void>(resolve => { changed = resolve; });
    const listeners = [
        vscode.workspace.onDidChangeTextDocument(event => {
            if (event.document === document && event.contentChanges.length) changed();
        }),
        vscode.workspace.onDidCloseTextDocument(closed => { if (closed === document) changed(); }),
    ];
    let timeout: NodeJS.Timeout | undefined;
    signal.addEventListener('abort', changed, { once: true });
    try {
        // Keep focus and history dispatch together in the workbench. Both operations
        // use the active editor, which a dock, preview, or concurrent tool can change.
        await vscode.commands.executeCommand('runCommands', {
            commands: ['workbench.action.focusActiveEditorGroup', action],
        });
        // Allow the extension host to receive the change even if dispatch was slow.
        if (!document.isClosed && document.version === version) {
            timeout = setTimeout(changed, 2_000);
            await update;
        }
        signal.throwIfAborted();
        if (!document.isClosed && document.version === version) {
            throw new Error(`VS Code did not ${action} an edit in the authored source.`);
        }
    } finally {
        for (const listener of listeners) listener.dispose();
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
