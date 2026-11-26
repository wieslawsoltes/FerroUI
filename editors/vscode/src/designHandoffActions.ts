import * as vscode from 'vscode';
import type { HandoffFile, HandoffPlan, HandoffRow } from './designHandoffProtocol';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import { choosePlacement } from './placementPicker';
import type { OutlinePosition } from './protocol';
import { resolveWorkspaceFile } from './workspacePaths';

export type HandoffAction = 'copy' | 'place' | 'source' | 'preview' | 'openAsset';

interface ReviewInput { session: number; manifest: string }
interface ApplyInput extends ReviewInput {
    action: HandoffAction;
    id: string;
    review: string;
    anchor?: number;
    position?: OutlinePosition;
    expected_revision?: number;
}

export type HandoffAgentInput = ReviewInput | ApplyInput | (ReviewInput & { action: 'openReview' });

export class DesignHandoffActions {
    constructor(private readonly designer: DesignerWorkbench,
        private readonly openReview: (panel: DesignerPanel, file: HandoffFile) => Promise<void>) {}

    plan(panel: DesignerPanel, file: HandoffFile, signal: AbortSignal) {
        return panel.designHandoff<HandoffPlan>(file, { action: 'review' }, signal);
    }

    async agent(input: HandoffAgentInput, signal: AbortSignal) {
        const panel = this.designer.designs().find(panel => panel.session === input.session && !panel.isDisposed);
        if (!panel) throw new Error('Choose an open designer session from ferroui_designs.');
        const folder = vscode.workspace.getWorkspaceFolder(panel.document.uri);
        if (!folder) throw new Error('Open the design in its workspace folder.');
        const uri = await resolveWorkspaceFile(input.manifest, folder.uri.toString());
        const file = { uri: uri.toString(), selected: false };
        if (!('action' in input)) return this.plan(panel, file, signal);
        if (input.action === 'openReview') {
            signal.throwIfAborted();
            await this.openReview(panel, file);
            return { opened: true };
        }
        if (!input.id || !input.review) throw new Error('Provide the mapping ID and review returned by designer_design_handoff.');
        if (input.action === 'place' && (input.anchor === undefined || input.position === undefined
            || input.expected_revision === undefined || panel.currentToken !== input.expected_revision)) {
            throw new Error('Read the current frame and provide an anchor and position before placing a mapped control or raster.');
        }
        return this.apply(panel, file, input, signal, input);
    }

    async apply(panel: DesignerPanel, file: HandoffFile, input: Pick<ApplyInput, 'action' | 'id' | 'review'>,
        signal: AbortSignal, target?: ApplyInput): Promise<{ uri?: string; placed?: boolean } | undefined> {
        const { action, id, review } = input;
        if (action === 'copy') return panel.designHandoff<{ uri: string }>(file, { action, id, review }, signal);
        const check = () => panel.designHandoff<HandoffRow>(file, { action: 'check', id, review }, signal);
        const frame = { session: panel.session, token: panel.currentToken ?? -1 };
        const row = await check();
        if (action === 'place') {
            if (!panel.canUse(frame) || !row.insertion) throw new Error('Choose a mapped control or raster with a current design frame.');
            const placement = target?.anchor !== undefined && target.position
                ? { origin: row.insertion, target: { type: 'outline' as const, node: target.anchor, position: target.position } }
                : await choosePlacement(panel, frame, row.insertion, row.name);
            if (!placement) return;
            await check();
            await panel.place(frame, placement);
            return { placed: true };
        }
        if (action === 'source' || action === 'preview') {
            const folder = vscode.workspace.getWorkspaceFolder(panel.document.uri);
            if (!row.definition || !folder) throw new Error('Choose a resolved Library definition.');
            return this.designer.openProjectDefinition({ ...row.definition, folder: folder.uri.toString() }, action, signal);
        }
        if (action === 'openAsset' && row.assetUri) {
            await vscode.commands.executeCommand('vscode.open', vscode.Uri.parse(row.assetUri));
            return { uri: row.assetUri };
        }
        throw new Error('Choose an available asset.');
    }
}
