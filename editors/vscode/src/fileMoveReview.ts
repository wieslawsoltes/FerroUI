import * as vscode from 'vscode';
import type { FileMovePlan } from './fileMoveProtocol';
import { errorMessage } from './protocol';

const scheme = 'ferroui-file-move-review';
const context = 'ferroui.reviewingFileMove';

interface Review {
    id: string;
    title: string;
    contents: Map<string, string>;
    apply: (validate: () => void) => Promise<vscode.Uri>;
    origin?: { uri: vscode.Uri; column: vscode.ViewColumn; selection: vscode.Selection };
}

export class FileMoveReview implements vscode.Disposable {
    private current?: Review;
    private serial = 0;
    private applying = false;
    private readonly subscriptions = [
        vscode.workspace.registerTextDocumentContentProvider(scheme, {
            provideTextDocumentContent: uri => this.current?.contents.get(uri.toString()),
        }),
        vscode.window.tabGroups.onDidChangeTabs(() => this.setContext()),
        vscode.window.tabGroups.onDidChangeTabGroups(() => this.setContext()),
        vscode.commands.registerCommand('ferroui.fileMoveReview.cancel', () => this.run(review => this.cancel(review))),
        vscode.commands.registerCommand('ferroui.fileMoveReview.apply', () => this.run(async review => {
            this.applying = true;
            try {
                const column = vscode.window.tabGroups.activeTabGroup.viewColumn;
                const destination = await review.apply(() => {
                    if (this.current !== review) throw new Error('Open the current file move review before applying it.');
                });
                try {
                    await vscode.window.showTextDocument(destination, { viewColumn: column, preview: false });
                } finally { await this.close(review); }
            } finally { this.applying = false; }
        })),
    ];

    async open(plan: FileMovePlan, apply: Review['apply']): Promise<void> {
        if (this.applying) throw new Error('A file move is being applied.');
        const serial = ++this.serial;
        if (this.current) await this.cancel(this.current);
        if (serial !== this.serial) return;
        const from = vscode.Uri.parse(plan.from, true);
        const to = vscode.Uri.parse(plan.to, true);
        const relative = (uri: vscode.Uri) => vscode.workspace.asRelativePath(uri, false);
        const title = `Move ${relative(from)} → ${relative(to)}`;
        const active = vscode.window.activeTextEditor;
        const origin = active?.viewColumn && vscode.window.tabGroups.activeTabGroup.activeTab?.isPreview
            ? { uri: active.document.uri, column: active.viewColumn, selection: active.selection } : undefined;
        const review = { id: String(serial), title, contents: new Map<string, string>(), apply, origin };
        const documents = [...plan.documents].sort((left, right) => Number(right.edit.uri === plan.from) - Number(left.edit.uri === plan.from));
        const resources = documents.map((document, index) => {
            const source = vscode.Uri.parse(document.edit.uri, true);
            const destination = document.edit.uri === plan.from ? to : source;
            const store = (part: string, uri: vscode.Uri, text: string) => {
                const name = vscode.workspace.asRelativePath(uri).replaceAll('\\', '/');
                const version = vscode.Uri.from({ scheme, path: `/${name.replace(/^\/+/, '')}`, query: `${review.id}/${index}/${part}` });
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
        void vscode.commands.executeCommand('setContext', context, !!this.current && this.isActive(this.current));
    }

    private isReviewSource(tab: vscode.Tab, review: Review): boolean {
        const input = tab.input;
        const uri = input instanceof vscode.TabInputTextDiff ? input.modified : input instanceof vscode.TabInputText ? input.uri : undefined;
        return uri?.scheme === scheme && uri.query.startsWith(`${review.id}/`);
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
