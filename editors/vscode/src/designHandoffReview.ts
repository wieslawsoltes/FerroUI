import * as path from 'node:path';
import * as vscode from 'vscode';
import type { DesignHandoffActions, HandoffAction } from './designHandoffActions';
import type { HandoffFile, HandoffPlan, HandoffState } from './designHandoffProtocol';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { handoffHtml, webviewOptions } from './view';

type Action = HandoffAction | 'refresh' | 'manifest';
const actions = new Set<Action>(['copy', 'place', 'source', 'preview', 'openAsset', 'refresh', 'manifest']);

export class DesignHandoffReview implements vscode.Disposable {
    private readonly panel: vscode.WebviewPanel;
    private readonly lifetime = new AbortController();
    private readonly subscriptions: vscode.Disposable[];
    private accepted?: { plan: HandoffPlan; identity: string; project: DesignerPanel['publishedAssetProject'] };
    private observed = new Set<string>();
    private revision = 0;
    private invalidated = false;
    private busy = false;
    private current = false;
    private status = '';
    private error = false;

    constructor(root: vscode.Uri, private readonly service: DesignHandoffActions, private readonly design: DesignerPanel,
        private readonly file: HandoffFile, private readonly name: string, onClose: () => void) {
        const document = path.basename(design.document.fileName);
        this.panel = vscode.window.createWebviewPanel('ferroui.designHandoff', `Handoff · ${document}`,
            vscode.ViewColumn.Active, { ...webviewOptions(root), retainContextWhenHidden: true });
        const uri = vscode.Uri.parse(file.uri);
        const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(path.dirname(uri.fsPath), path.basename(uri.fsPath)));
        const invalidate = () => { this.invalidated = true; this.update(); };
        this.subscriptions = [
            this.panel.onDidDispose(() => {
                this.lifetime.abort();
                this.subscriptions.forEach(subscription => subscription.dispose());
                onClose();
            }),
            this.panel.webview.onDidReceiveMessage(message => { void this.receive(message); }),
            design.onDidChange(() => this.update()),
            vscode.workspace.onDidChangeTextDocument(event => {
                const uri = event.document.uri.toString();
                if (event.contentChanges.length && (uri === file.uri || this.observed.has(uri))) invalidate();
            }),
            watcher, watcher.onDidCreate(invalidate), watcher.onDidChange(invalidate), watcher.onDidDelete(invalidate),
        ];
        this.panel.webview.html = handoffHtml(this.panel.webview, root);
    }

    reveal(): void { this.panel.reveal(); }
    start(): Promise<void> { return this.receive({ action: 'refresh' }); }

    private async receive(message: { action?: string; id?: unknown }): Promise<void> {
        if (message?.action === 'ready') { this.publish(); return; }
        const action = message?.action as Action;
        if (!actions.has(action) || this.busy || this.lifetime.signal.aborted || !vscode.workspace.isTrusted) return;
        this.busy = true;
        this.status = '';
        this.error = false;
        this.publish();
        const signal = AbortSignal.any([this.lifetime.signal, AbortSignal.timeout(action === 'place' ? 120_000 : 55_000)]);
        try {
            if (action === 'refresh') await this.refresh(signal);
            else if (action === 'manifest') await vscode.window.showTextDocument(vscode.Uri.parse(this.file.uri), { preview: false, viewColumn: vscode.ViewColumn.Beside });
            else {
                const row = this.accepted?.plan.rows.find(row => row.id === message.id);
                if (!this.isCurrent() || !row?.review) throw new Error('Refresh the handoff before applying a mapping.');
                const result = await this.service.apply(this.design, this.file, { action, id: row.id, review: row.review }, signal);
                if (result?.placed || (result?.uri && action === 'copy')) {
                    this.status = result.placed ? `Placed ${row.name}.` : `Copied ${row.name}.`;
                    await this.refresh(signal);
                }
            }
        } catch (error) {
            const message = errorMessage(error);
            this.status = this.status ? `${this.status} ${message}` : message;
            this.error = true;
        } finally {
            this.busy = false;
            this.publish();
        }
    }

    private async refresh(signal: AbortSignal) {
        this.invalidated = false;
        let plan: HandoffPlan;
        try {
            plan = await this.service.plan(this.design, this.file, signal);
        } catch (error) {
            this.invalidated = true;
            throw error;
        }
        this.accepted = { plan, identity: this.design.handoffIdentity(), project: this.design.publishedAssetProject };
        this.observed = new Set(plan.rows.flatMap(row => {
            const folder = vscode.workspace.getWorkspaceFolder(this.design.document.uri);
            return row.definition && folder ? [vscode.Uri.joinPath(folder.uri, row.definition.path).toString()] : [];
        }));
        this.revision++;
    }

    private isCurrent() {
        return !this.invalidated && !this.design.isDisposed && this.accepted !== undefined
            && this.accepted.identity === this.design.handoffIdentity() && this.accepted.project === this.design.publishedAssetProject;
    }

    private update() {
        if (this.current !== this.isCurrent()) this.publish();
    }

    private publish() {
        if (this.lifetime.signal.aborted) return;
        this.current = this.isCurrent();
        const state: HandoffState = {
            type: 'handoff', manifest: this.name, document: path.basename(this.design.document.fileName),
            revision: this.revision, busy: this.busy, current: this.current, rows: this.accepted?.plan.rows,
            status: this.status || (this.accepted && !this.current ? 'The source changed. Refresh the handoff review.' : ''),
            error: this.error,
        };
        void this.panel.webview.postMessage(state);
    }

    dispose(): void { this.panel.dispose(); }
}
