import * as vscode from 'vscode';

let historyQueue = Promise.resolve();
let windowBlur = 0;
let refreshedBlur = 0;

export function watchEditorHistoryFocus(): vscode.Disposable {
    if (!vscode.window.state.focused) windowBlur++;
    return vscode.window.onDidChangeWindowState(({ focused }) => { if (!focused) windowBlur++; });
}

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
    // Search and other widgets can finish taking focus after their command returned.
    // Retry only after the first command finished without updating this document.
    for (let attempt = 0; attempt < 2; attempt++) {
        if (await dispatchEditorHistory(document, action, requireCurrent, signal)) return;
    }
    throw new Error(`VS Code did not ${action} an edit in the authored source.`);
}

async function dispatchEditorHistory(
    document: vscode.TextDocument,
    action: 'undo' | 'redo',
    requireCurrent: () => void,
    signal: AbortSignal,
): Promise<boolean> {
    signal.throwIfAborted();
    requireCurrent();
    const viewColumn = sourceColumn(document);
    await focusHistoryWindow(requireCurrent, signal);
    const blur = windowBlur;
    if (refreshedBlur !== blur) await refreshVisibleEditorFocus(document, requireCurrent, signal);
    const editor = await vscode.window.showTextDocument(document, {
        viewColumn, preserveFocus: false, preview: false,
    });
    signal.throwIfAborted();
    requireCurrent();
    if (!vscode.window.state.focused || windowBlur !== blur) throw historyFocusError();
    const active = vscode.window.tabGroups.activeTabGroup.activeTab?.input;
    if (document.isClosed || vscode.window.activeTextEditor !== editor
        || !(active instanceof vscode.TabInputText) || active.uri.toString() !== document.uri.toString()) {
        throw new Error('The authored editor lost focus.');
    }
    refreshedBlur = blur;
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
        return document.isClosed || document.version !== version;
    } finally {
        for (const listener of listeners) listener.dispose();
        clearTimeout(timeout);
        signal.removeEventListener('abort', changed);
    }
}

async function focusHistoryWindow(requireCurrent: () => void, signal: AbortSignal): Promise<void> {
    if (vscode.window.state.focused) return;
    const focusWindow = 'workbench.action.focusWindow';
    const commands = await vscode.commands.getCommands(true);
    signal.throwIfAborted();
    requireCurrent();
    if (!vscode.window.state.focused && commands.includes(focusWindow)) {
        await vscode.commands.executeCommand(focusWindow);
        if (!vscode.window.state.focused && !signal.aborted) {
            await new Promise<void>(resolve => {
                const listener = vscode.window.onDidChangeWindowState(({ focused }) => { if (focused) finish(); });
                const timeout = setTimeout(finish, 1_000);
                signal.addEventListener('abort', finish, { once: true });
                function finish(): void {
                    listener.dispose();
                    clearTimeout(timeout);
                    signal.removeEventListener('abort', finish);
                    resolve();
                }
            });
        }
    }
    signal.throwIfAborted();
    if (!vscode.window.state.focused) throw historyFocusError();
}

async function refreshVisibleEditorFocus(document: vscode.TextDocument, requireCurrent: () => void, signal: AbortSignal): Promise<void> {
    // A native app can take OS focus while VS Code changes groups. Refocus each visible
    // text editor once so its input context receives a blur when the source takes focus.
    for (const editor of vscode.window.visibleTextEditors) {
        if (editor.document === document) continue;
        const tab = vscode.window.tabGroups.all.find(group => group.viewColumn === editor.viewColumn)?.activeTab;
        if (!(tab?.input instanceof vscode.TabInputText) || tab.input.uri.toString() !== editor.document.uri.toString()) continue;
        signal.throwIfAborted();
        requireCurrent();
        await vscode.window.showTextDocument(editor.document, { viewColumn: editor.viewColumn, preserveFocus: false, preview: tab.isPreview });
    }
}

function historyFocusError(): Error {
    return new Error('Focus the VS Code window and retry the source history request.');
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
