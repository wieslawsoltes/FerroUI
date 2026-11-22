import * as path from 'node:path';
import * as vscode from 'vscode';
import { DesignHandoffActions } from './designHandoffActions';
import type { HandoffFile } from './designHandoffProtocol';
import { DesignHandoffReview } from './designHandoffReview';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import { errorMessage } from './protocol';
import { xamlLanguage } from './xamlServer';

const suffix = '.ferroui-handoff.json';

export class DesignHandoff implements vscode.Disposable {
    readonly actions: DesignHandoffActions;
    private readonly reviews = new Map<string, DesignHandoffReview>();
    private readonly command: vscode.Disposable;

    constructor(private readonly context: vscode.ExtensionContext, private readonly designer: DesignerWorkbench) {
        this.actions = new DesignHandoffActions(designer, (panel, file) => this.show(panel, file));
        this.command = vscode.commands.registerCommand('ferroui.designHandoff.review', async (uri?: vscode.Uri) => {
            try { await this.open(uri); }
            catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
        });
    }

    private async open(explicit?: vscode.Uri): Promise<void> {
        if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before reviewing a design handoff.');
        const editor = vscode.window.activeTextEditor;
        const source = editor?.document;
        const active = source?.uri.scheme === 'file' && source.uri.path.toLowerCase().endsWith(suffix) ? source.uri : undefined;
        const panel = editor && (editor.document.languageId === 'rust' || editor.document.languageId === xamlLanguage)
            ? await this.designer.openDocument(editor.document.uri, editor.selection.active) : this.designer.activeDesign;
        if (!panel || !vscode.workspace.getWorkspaceFolder(panel.document.uri)) {
            throw new Error('Open a XAML or RSX design in a workspace to review its handoff.');
        }
        const address = explicit?.scheme === 'file' ? explicit : active;
        const uri = address ?? (await vscode.window.showOpenDialog({
            title: 'Open FerroUI Design Handoff', openLabel: 'Review', filters: { 'Design handoff': ['json'] }, canSelectMany: false,
        }))?.[0];
        if (!uri) return;
        await this.show(panel, { uri: uri.toString(), selected: !address });
    }

    private async show(panel: DesignerPanel, file: HandoffFile): Promise<void> {
        await panel.waitForSnapshot(AbortSignal.timeout(20_000));
        const uri = vscode.Uri.parse(file.uri);
        const key = JSON.stringify([panel.session, file.uri]);
        const existing = this.reviews.get(key);
        if (existing) { existing.reveal(); return; }
        const review = new DesignHandoffReview(this.context.extensionUri, this.actions, panel, file,
            path.basename(uri.path), () => this.reviews.delete(key));
        this.reviews.set(key, review);
        await review.start();
    }

    dispose(): void {
        this.command.dispose();
        for (const review of this.reviews.values()) review.dispose();
    }
}
