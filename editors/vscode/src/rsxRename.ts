import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';

const controlName = /^[\p{L}_][\p{L}\p{Nd}\p{M}_]{0,127}$/u;
const invalidName = 'Use a letter or underscore followed by letters, digits or underscores.';

export async function promptRsxControlName(panel: DesignerPanel): Promise<void> {
    await panel.waitForSnapshot(AbortSignal.timeout(15_000));
    const selected = panel.selectedNode;
    const token = panel.currentToken;
    if (!selected?.name || token === undefined) throw new Error('Select an authored RSX control with a name literal.');
    const value = await vscode.window.showInputBox({ title: 'Rename RSX Control', prompt: 'Enter a new name for the selected control', value: selected.name,
        validateInput: name => controlName.test(name) ? undefined : invalidName });
    if (value !== undefined) await renameRsxControl(panel, token, selected.id, value, AbortSignal.timeout(15_000));
}

export async function renameRsxControl(panel: DesignerPanel, token: number, node: number, name: string, signal: AbortSignal): Promise<void> {
    if (panel.language !== 'rust') throw new Error('The RSX control rename is available in a Rust designer.');
    if (!controlName.test(name)) throw new Error(invalidName);
    const snapshot = panel.snapshot;
    if (!snapshot || panel.currentToken !== token) throw new Error('Read the current RSX designer revision before renaming.');
    const selected = snapshot.nodes.find(candidate => candidate.id === node);
    if (!selected?.name || !selected.properties.some(field => field.name === 'Name' && field.editable)) {
        throw new Error('Select a mapped RSX control with an authored name literal.');
    }
    if (name === selected.name) return;
    if (snapshot.nodes.some(candidate => candidate.id !== node && candidate.name === name)) {
        throw new Error('Another authored control in this RSX invocation already uses that name.');
    }
    await panel.agentEdit(token, { type: 'setProperty', node, name: 'Name', value: name }, signal);
}
