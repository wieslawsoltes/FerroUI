import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import type { DragOrigin, FrameTarget, OutlinePosition, Placement } from './protocol';

const positions: readonly OutlinePosition[] = ['before', 'inside', 'after'];

export async function choosePlacement(panel: DesignerPanel, frame: FrameTarget, origin: DragOrigin, title: string): Promise<Placement | undefined> {
    const selected = () => panel.selectedNode ?? panel.nodes[0];
    const anchor = selected();
    if (!anchor) throw new Error('Select an authored control for placement.');
    const label = anchor.name ?? anchor.label;
    const results = await Promise.allSettled(positions.map(async position => {
        const placement: Placement = { origin, target: { type: 'outline', node: anchor.id, position } };
        const hint = await panel.previewPlacement(frame, placement);
        const prefix = position[0].toUpperCase() + position.slice(1);
        return { label: `${prefix} ${label}`, description: hint.description, placement };
    }));
    const options = results.flatMap(result => result.status === 'fulfilled' ? [result.value] : []);
    if (!options.length) throw new Error(`Choose a control that accepts placement near ${label}.`);
    if (!panel.canUse(frame) || selected()?.id !== anchor.id) throw new Error('The selected target changed. Choose placement again.');
    const item = await vscode.window.showQuickPick(options, { title: `Place ${title}`, matchOnDescription: true });
    if (item && (!panel.canUse(frame) || selected()?.id !== anchor.id)) throw new Error('The selected target changed. Choose placement again.');
    return item?.placement;
}
