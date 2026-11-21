import * as vscode from 'vscode';
import type { ReviewedDocument } from './reviewedSources';
import { errorMessage } from './protocol';

interface Review {
    id: string;
    title: string;
    contents: Map<string, string>;
    apply: (validate: () => void) => Promise<vscode.Uri | undefined>;
    origin?: { uri: vscode.Uri; column: vscode.ViewColumn; selection: vscode.Selection };
}

interface ReviewActions {
    scheme: string;
    context: string;
    command: string;
    reveal?: (destination: vscode.Uri, column: vscode.ViewColumn) => Thenable<unknown>;
}

export class SourceChangeReview implements vscode.Disposable {
    private current?: Review;
    private serial = 0;
    private applying = false;
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly actions: ReviewActions) {
        this.subscriptions = [
            vscode.workspace.registerTextDocumentContentProvider(actions.scheme, {
                provideTextDocumentContent: uri => this.current?.contents.get(uri.toString()),
            }),
            vscode.window.tabGroups.onDidChangeTabs(() => this.setContext()),
            vscode.window.tabGroups.onDidChangeTabGroups(() => this.setContext()),
            vscode.commands.registerCommand(`${actions.command}.cancel`, () => this.run(review => this.cancel(review))),
            vscode.commands.registerCommand(`${actions.command}.apply`, () => this.run(async review => {
                this.applying = true;
                try {
                    const column = vscode.window.tabGroups.activeTabGroup.viewColumn;
                    const destination = await review.apply(() => {
                        if (this.current !== review) throw new Error('Open the current source review before applying it.');
                    });
                    if (!destination) return;
                    try {
                        if (actions.reveal) await actions.reveal(destination, column);
                        else await vscode.window.showTextDocument(destination, { viewColumn: column, preview: false });
                    } finally { await this.close(review); }
                } finally { this.applying = false; }
            })),
        ];
    }

    async open(title: string, documents: readonly ReviewedDocument[], apply: Review['apply'],
        retarget?: { from: string; to: vscode.Uri }): Promise<void> {
        if (this.applying) throw new Error('A source review is being applied.');
        const serial = ++this.serial;
        if (this.current) await this.cancel(this.current);
        if (serial !== this.serial) return;
        const active = vscode.window.activeTextEditor;
        const origin = active?.viewColumn && vscode.window.tabGroups.activeTabGroup.activeTab?.isPreview
            ? { uri: active.document.uri, column: active.viewColumn, selection: active.selection } : undefined;
        const review = { id: String(serial), title, contents: new Map<string, string>(), apply, origin };
        const resources = documents.map((document, index) => {
            const source = vscode.Uri.parse(document.edit.uri, true);
            const destination = retarget && document.edit.uri === retarget.from ? retarget.to : source;
            const store = (part: string, uri: vscode.Uri, text: string) => {
                const name = vscode.workspace.asRelativePath(uri).replaceAll('\\', '/');
                const version = vscode.Uri.from({ scheme: this.actions.scheme, path: `/${name.replace(/^\/+/, '')}`, query: `${review.id}/${index}/${part}` });
                review.contents.set(version.toString(), text);
                return version;
            };
            return [destination, store('before', source, document.before), store('after', destination, document.after)];
        });
        this.current = review;
        try { await vscode.commands.executeCommand('vscode.changes', title, resources); }
        catch (error) { await this.close(review); throw error; }
        this.setContext();
    }

    private async run(action: (review: Review) => Promise<void>): Promise<void> {
        const review = this.current;
        if (!review || !this.isActive(review) || this.applying) return;
        try { await action(review); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private isActive(review: Review): boolean {
        const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
        return !!tab && (isReviewTab(tab, review) || this.isReviewSource(tab, review));
    }

    private setContext(): void {
        void vscode.commands.executeCommand('setContext', this.actions.context, !!this.current && this.isActive(this.current));
    }

    private isReviewSource(tab: vscode.Tab, review: Review): boolean {
        const input = tab.input;
        const uri = input instanceof vscode.TabInputTextDiff ? input.modified : input instanceof vscode.TabInputText ? input.uri : undefined;
        return uri?.scheme === this.actions.scheme && uri.query.startsWith(`${review.id}/`);
    }

    private async close(review: Review): Promise<void> {
        if (this.current === review) this.current = undefined;
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs.filter(tab => isReviewTab(tab, review) || this.isReviewSource(tab, review)));
        if (tabs.length) await vscode.window.tabGroups.close(tabs);
        this.setContext();
    }

    private async cancel(review: Review): Promise<void> {
        try {
            const origin = review.origin;
            const group = origin && vscode.window.tabGroups.all.find(item => item.viewColumn === origin.column);
            if (origin && group?.tabs.every(tab => isReviewTab(tab, review) || this.isReviewSource(tab, review))) {
                // Restore the source when the Changes tab replaced its preview editor.
                await vscode.window.showTextDocument(origin.uri, { viewColumn: origin.column, selection: origin.selection, preview: true });
            }
        } finally { await this.close(review); }
    }

    dispose(): void {
        this.current = undefined;
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}

function isReviewTab(tab: vscode.Tab, review: Review): boolean {
    // The native Changes editor appends its localized file count to the label.
    return tab.label === review.title || tab.label.startsWith(`${review.title} (`);
}
