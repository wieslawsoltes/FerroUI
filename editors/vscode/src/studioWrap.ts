import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { StudioContext } from './studioContext';

interface WrapperChoice {
    container: string;
    label: string;
    detail: string;
    reason: string | null;
}

export async function chooseWrapper(panel: DesignerPanel, stillActive: () => boolean): Promise<void> {
    const token = panel.currentToken;
    if (token === undefined || !panel.supportsSelectionWrapping || !panel.selectedIds.length) {
        throw new Error('Select authored controls in a current design before wrapping.');
    }
    const context = new StudioContext(panel, token, stillActive);
    const { choices } = await context.read<{ choices: WrapperChoice[] }>('designer_wrap_selection');
    const available = choices.filter(option => !option.reason);
    if (!available.length) throw new Error(choices[0]?.reason ?? 'The selected controls have no supported wrapper.');
    const choice = await vscode.window.showQuickPick(available, {
        title: 'Wrap Selection',
        placeHolder: context.selection.length === 1 ? 'Choose a container for the selected control'
            : `Choose a container for ${context.selection.length} controls`,
    });
    if (!choice) return;
    const args = { container: choice.container };
    const { model } = await context.read<{ model: string }>('designer_wrap_selection', args);
    await context.edit('designer_wrap_selection_edit', { ...args, expected_model: model });
}
