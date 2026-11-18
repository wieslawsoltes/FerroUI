import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';

const commands = [
    ['align_left', 'Align left edges', 2], ['align_horizontal_center', 'Align horizontal centers', 2],
    ['align_right', 'Align right edges', 2], ['align_top', 'Align top edges', 2],
    ['align_vertical_center', 'Align vertical centers', 2], ['align_bottom', 'Align bottom edges', 2],
    ['match_width', 'Match width', 2], ['match_height', 'Match height', 2], ['match_size', 'Match size', 2],
    ['distribute_horizontal', 'Distribute horizontally', 3], ['distribute_vertical', 'Distribute vertically', 3],
    ['reset_margin', 'Reset margins', 1], ['reset_alignment', 'Reset alignment', 1],
    ['reset_width', 'Reset width', 1], ['reset_height', 'Reset height', 1], ['reset_size', 'Reset size', 1],
    ['reset_all', 'Reset local layout values', 1],
] as const;

export async function chooseLayout(panel: DesignerPanel, stillActive: () => boolean): Promise<void> {
    const token = panel.currentToken;
    const selection = [...panel.selectedIds];
    if (token === undefined || panel.status || !selection.length) throw new Error('Select authored controls in the current preview.');
    const choice = await vscode.window.showQuickPick(commands.filter(([, , count]) => selection.length >= count)
        .map(([operation, label]) => ({ label, operation })), {
        title: 'FerroUI Layout',
        placeHolder: selection.length > 1 ? `${selection.length} selected controls` : 'Reset a local layout value',
    });
    if (!choice) return;
    const current = () => stillActive() && panel.currentToken === token && selection.length === panel.selectedIds.length
        && selection.every((id, index) => id === panel.selectedIds[index]);
    if (!current()) throw new Error('The preview or selection changed. Choose the layout command again.');
    const signal = AbortSignal.timeout(15_000);
    const args = { operation: choice.operation };
    const plan = await panel.studioQuery('designer_selection_layout', args, selection, signal) as {
        model?: string; edit_count?: number;
    };
    if (!current() || typeof plan.model !== 'string') throw new Error('The layout plan expired. Choose the command again.');
    if (plan.edit_count === 0) {
        void vscode.window.setStatusBarMessage('FerroUI: the selection already has this layout.', 3000);
        return;
    }
    await panel.studioEdit(token, 'designer_selection_layout_edit', { ...args, expected_model: plan.model }, selection, signal);
}
