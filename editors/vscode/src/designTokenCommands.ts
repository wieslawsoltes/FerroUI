import * as vscode from 'vscode';
import type { TokenDirection } from './designTokenProtocol';
import { DesignTokenReview } from './designTokenReview';
import type { ProjectInventory } from './projectInventory';
import { errorMessage } from './protocol';

const scheme = 'ferroui-token-review';

export class DesignTokenCommands implements vscode.Disposable {
    private readonly content = new Map<string, string>();
    private readonly changed = new vscode.EventEmitter<vscode.Uri>();
    private readonly reviews = new Map<string, DesignTokenReview>();
    private readonly subscriptions: vscode.Disposable[];
    private nextPreview = 0;

    constructor(private readonly context: vscode.ExtensionContext, private readonly projects: ProjectInventory) {
        this.subscriptions = [
            vscode.workspace.registerTextDocumentContentProvider(scheme, {
                onDidChange: this.changed.event,
                provideTextDocumentContent: uri => this.content.get(uri.toString()) ?? '',
            }),
            vscode.workspace.onDidCloseTextDocument(document => this.content.delete(document.uri.toString())),
            vscode.commands.registerCommand('ferroui.designTokens.import', (uri?: vscode.Uri) => this.run('import', uri)),
            vscode.commands.registerCommand('ferroui.designTokens.export', (uri?: vscode.Uri) => this.run('export', uri)),
        ];
    }

    private async run(direction: TokenDirection, explicit?: vscode.Uri): Promise<void> {
        try {
            if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before converting design tokens.');
            const extensions = direction === 'import' ? ['.tokens.json', '.tokens'] : ['.xaml'];
            const active = vscode.window.activeTextEditor?.document.uri;
            const uri = explicit ?? (active?.scheme === 'file' && extensions.some(extension => active.path.toLowerCase().endsWith(extension)) ? active
                : (await vscode.window.showOpenDialog({ canSelectMany: false,
                    filters: direction === 'import' ? { 'Design tokens': ['json', 'tokens'] } : { 'XAML resource dictionary': ['xaml'] },
                }))?.[0]);
            if (!uri) return;
            const document = await vscode.workspace.openTextDocument(uri);
            const key = `${direction}:${uri}`;
            const existing = this.reviews.get(key);
            if (existing) { existing.reveal(); return; }
            const editor = await vscode.window.showTextDocument(document, { preview: false });
            const opened = this.reviews.get(key);
            if (opened) { opened.reveal(); return; }
            const destination = direction === 'import' ? 'DesignTokens.xaml' : 'design.tokens.json';
            const preview = vscode.Uri.from({ scheme, path: `/${destination}`, query: String(++this.nextPreview) });
            const review = new DesignTokenReview(this.context.extensionUri, this.projects, document, direction, destination, editor.viewColumn,
                async source => {
                    const key = preview.toString();
                    if (this.content.get(key) !== source) {
                        this.content.set(key, source);
                        this.changed.fire(preview);
                    }
                    return vscode.workspace.openTextDocument(preview);
                }, () => this.reviews.delete(key));
            this.reviews.set(key, review);
            await review.start();
        } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void {
        for (const review of this.reviews.values()) review.dispose();
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
        this.content.clear();
    }
}
