import * as vscode from 'vscode';
import type { DictionaryPlan } from './dictionaryProtocol';
import { errorMessage } from './protocol';

const scheme = 'ferroui-dictionary-review';
interface Review {
    key: string;
    documents: Map<string, string>;
    file: vscode.Uri;
    apply: (validate: () => void) => Promise<DictionaryPlan>;
}

export class DictionaryReview implements vscode.Disposable {
    private current?: Review;
    private serial = 0;
    private applying = false;
    private readonly subscriptions = [
        vscode.workspace.registerTextDocumentContentProvider(scheme, {
            provideTextDocumentContent: uri => this.current?.documents.get(uri.toString()),
        }),
        vscode.commands.registerCommand('ferroui.dictionaryReview.showFile', (uri?: vscode.Uri) =>
            this.run(uri, async review => { await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(review.file), { preview: false }); })),
        vscode.commands.registerCommand('ferroui.dictionaryReview.cancel', (uri?: vscode.Uri) => this.run(uri, review => this.close(review))),
        vscode.commands.registerCommand('ferroui.dictionaryReview.apply', (uri?: vscode.Uri) => this.run(uri, review => this.apply(review))),
    ];

    async open(name: string, before: string, after: string, plan: DictionaryPlan, apply: Review['apply']): Promise<void> {
        if (this.applying) throw new Error('An extraction is being applied.');
        const serial = ++this.serial;
        if (this.current) await this.close(this.current);
        if (serial !== this.serial) return;
        const key = String(serial);
        const uri = (part: string, name: string) => vscode.Uri.from({ scheme, path: `/${part}/${name}`, query: key });
        const original = uri('before', name);
        const proposed = uri('after', name);
        const file = uri('new', plan.file.fileName);
        const review = { key, file, apply, documents: new Map([
            [original.toString(), before], [proposed.toString(), after], [file.toString(), plan.file.source],
        ]) };
        this.current = review;
        try {
            await vscode.commands.executeCommand('vscode.diff', original, proposed, `Extract Resources · ${name}`, { preview: false });
        } catch (error) {
            await this.close(review);
            throw error;
        }
    }

    private async run(uri: vscode.Uri | undefined, action: (review: Review) => Promise<void>): Promise<void> {
        const target = uri ?? vscode.window.activeTextEditor?.document.uri;
        const review = this.current;
        if (!review || !target || target.scheme !== scheme || target.query !== review.key || this.applying) return;
        try { await action(review); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private async apply(review: Review): Promise<void> {
        this.applying = true;
        try {
            const applied = await review.apply(() => {
                if (this.current !== review) throw new Error('Open the current extraction review before applying it.');
            });
            await this.close(review);
            const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(applied.file.uri, true));
            await vscode.window.showTextDocument(document, { preview: false });
        } finally {
            this.applying = false;
        }
    }

    private async close(review: Review): Promise<void> {
        if (this.current === review) this.current = undefined;
        const belongs = (uri: vscode.Uri) => uri.scheme === scheme && uri.query === review.key;
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs.filter(tab =>
            tab.input instanceof vscode.TabInputTextDiff ? belongs(tab.input.modified)
                : tab.input instanceof vscode.TabInputText && belongs(tab.input.uri)));
        if (tabs.length) await vscode.window.tabGroups.close(tabs);
    }

    dispose(): void {
        this.current = undefined;
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}
