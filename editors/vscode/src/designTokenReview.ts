import * as path from 'node:path';
import * as vscode from 'vscode';
import type { TokenDirection, TokenOptions, TokenPlan, TokenReviewState } from './designTokenProtocol';
import type { ProjectInventory } from './projectInventory';
import { errorMessage } from './protocol';
import { choose } from './studioContext';
import { tokenReviewHtml, webviewOptions } from './view';
import { localFolders } from './workspacePaths';
import { WorkspaceTools } from './workspaceTools';

const title = 'FerroUI Design Tokens';

export class DesignTokenReview implements vscode.Disposable {
    private readonly panel: vscode.WebviewPanel;
    private readonly lifetime = new AbortController();
    private readonly subscriptions: vscode.Disposable[];
    private readonly files = new WorkspaceTools();
    private options: TokenOptions = { mappings: Object.create(null) as TokenOptions['mappings'] };
    private accepted?: { plan: TokenPlan; version: number; document: vscode.TextDocument };
    private mappingRevision = 0;
    private candidateColumn: vscode.ViewColumn = vscode.ViewColumn.Beside;
    private busy = false;
    private problem?: string;

    constructor(root: vscode.Uri, private readonly projects: ProjectInventory, private document: vscode.TextDocument,
        private readonly direction: TokenDirection, private readonly name: string, column: vscode.ViewColumn | undefined,
        private readonly candidate: (source: string) => Thenable<vscode.TextDocument>, onClose: () => void) {
        const label = path.basename(document.uri.path);
        this.panel = vscode.window.createWebviewPanel('ferroui.designTokenReview', `Tokens · ${label}`,
            column ?? vscode.ViewColumn.Active, { ...webviewOptions(root), retainContextWhenHidden: true });
        this.subscriptions = [
            this.panel.onDidDispose(() => {
                this.lifetime.abort();
                this.subscriptions.forEach(subscription => subscription.dispose());
                onClose();
            }),
            this.panel.webview.onDidReceiveMessage(message => { void this.receive(message); }),
            vscode.workspace.onDidChangeTextDocument(event => { if (event.document === this.document) this.publish(); }),
            vscode.workspace.onDidCloseTextDocument(closed => { if (closed === this.document) this.publish(); }),
        ];
        this.panel.webview.html = tokenReviewHtml(this.panel.webview, root);
    }

    reveal(): void { this.panel.reveal(); }
    start(): Promise<void> { return this.receive({ type: 'refresh' }); }

    private async receive(message: { type?: string; path?: unknown }): Promise<void> {
        if (message?.type === 'ready') { this.publish(); return; }
        const action = message?.type;
        if (this.busy || this.lifetime.signal.aborted || !vscode.workspace.isTrusted
            || !['refresh', 'candidate', 'scale', 'edit', 'save'].includes(action ?? '')) return;
        this.busy = true;
        this.problem = undefined;
        this.publish();
        try {
            if (action === 'refresh') await this.refresh(this.options);
            else if (action === 'candidate' && this.accepted) await this.openCandidate(this.accepted.plan);
            else {
                const accepted = this.current();
                if (action === 'save') await this.save(accepted);
                else if (action === 'scale' && this.direction === 'import') {
                    const scale = await vscode.window.showInputBox({ title, prompt: 'Logical pixels for one rem',
                        value: String(this.options.pixelsPerRem ?? 16), validateInput: value => Number(value) > 0 && Number.isFinite(Number(value)) ? undefined : 'Enter a positive finite number.' });
                    if (scale !== undefined && this.current() === accepted) await this.refresh({ ...this.options, pixelsPerRem: Number(scale) });
                } else if (action === 'edit' && typeof message.path === 'string') {
                    const options = await this.editMapping(accepted.plan, message.path);
                    if (options && this.current() === accepted) await this.refresh(options);
                }
            }
        } catch (error) {
            if (!this.lifetime.signal.aborted) this.problem = errorMessage(error);
        } finally {
            this.busy = false;
            this.publish();
        }
    }

    private current() {
        this.lifetime.signal.throwIfAborted();
        const accepted = this.accepted;
        if (!accepted || accepted.document !== this.document || this.document.isClosed || this.document.version !== accepted.version) {
            throw new Error('The source changed. Refresh this review.');
        }
        return accepted;
    }

    private async refresh(options: TokenOptions): Promise<void> {
        if (this.document.isClosed) this.document = await vscode.workspace.openTextDocument(this.document.uri);
        const first = this.accepted === undefined;
        const { version } = this.document;
        const signal = this.signal(20_000);
        const plan = await this.projects.designTokens({ direction: this.direction, source: this.document.getText(), options }, signal);
        signal.throwIfAborted();
        if (this.document.isClosed || version !== this.document.version) throw new Error('The source changed. Refresh this review.');
        this.accepted = { plan, version, document: this.document };
        this.mappingRevision++;
        this.options = options;
        await this.candidate(plan.source);
        if (first) await this.openCandidate(plan);
    }

    private async openCandidate(plan: TokenPlan): Promise<void> {
        const document = await this.candidate(plan.source);
        this.lifetime.signal.throwIfAborted();
        const editor = await vscode.window.showTextDocument(document, { preview: true, preserveFocus: true, viewColumn: this.candidateColumn });
        this.candidateColumn = editor.viewColumn ?? this.candidateColumn;
    }

    private async save(accepted: NonNullable<DesignTokenReview['accepted']>): Promise<void> {
        if (!accepted.plan.mappings.length) return;
        const folders = localFolders();
        if (!folders.length) throw new Error('Open a workspace to save the reviewed token conversion.');
        const owner = vscode.workspace.getWorkspaceFolder(this.document.uri);
        const folder = owner?.uri.scheme === 'file' ? owner : folders.length === 1 ? folders[0]
            : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Select a workspace folder for the token conversion' });
        if (!folder) return;
        const extension = this.direction === 'import' ? '.xaml' : '.json';
        const destination = await vscode.window.showInputBox({ title, prompt: `New file path relative to ${folder.name}`, value: this.name,
            validateInput: value => value.trim().toLowerCase().endsWith(extension) ? undefined : `Choose a ${extension} file.` });
        if (!destination || this.current() !== accepted) return;
        await this.files.call('ferroui_workspace_create', { path: destination.trim(),
            folder: folder.uri.toString(), source: accepted.plan.source, open: true }, this.signal(15_000));
    }

    private async editMapping(plan: TokenPlan, key: string): Promise<TokenOptions | undefined> {
        const importing = this.direction === 'import';
        const row = plan.mappings.find(mapping => (importing ? mapping.token : mapping.key) === key);
        if (!row && !plan.issues.some(issue => issue.path === key && key)) return;
        const value = await vscode.window.showInputBox({ title, prompt: importing ? 'XAML resource key' : 'Dotted DTCG token path',
            value: (importing ? row?.key : row?.token) ?? this.options.mappings[key]?.[importing ? 'key' : 'token'] ?? key });
        if (!value?.trim()) return;
        const mapping = { ...this.options.mappings[key], [importing ? 'key' : 'token']: value.trim() };
        const forms = importing ? row?.kind === 'color' ? ['SolidColorBrush', 'Color']
            : row?.kind === 'gradient' ? ['LinearGradientBrush', 'RadialGradientBrush', 'ConicGradientBrush'] : undefined
            : row?.xamlType === 'x:Double' ? ['number', 'dimension'] : undefined;
        if (forms) {
            const form = await choose('Resource representation', forms.map(label => ({ label })));
            if (!form) return;
            mapping[importing ? 'xamlType' : 'kind'] = form.label;
        }
        const mappings = Object.assign(Object.create(null) as TokenOptions['mappings'], this.options.mappings);
        mappings[key] = mapping;
        return { ...this.options, mappings };
    }

    private signal(timeout: number): AbortSignal {
        return AbortSignal.any([this.lifetime.signal, AbortSignal.timeout(timeout)]);
    }

    private publish(): void {
        if (this.lifetime.signal.aborted) return;
        const current = this.accepted?.document === this.document && this.accepted.version === this.document.version && !this.document.isClosed;
        const state: TokenReviewState = {
            type: 'review', direction: this.direction, sourceName: path.basename(this.document.uri.path),
            ready: this.accepted !== undefined, current, busy: this.busy, mappingRevision: this.mappingRevision,
            mappingRows: this.accepted?.plan.mappings ?? [], issues: this.accepted?.plan.issues ?? [],
            problem: this.problem ?? (this.accepted && !current ? 'The source changed. Refresh this review.' : undefined),
        };
        void this.panel.webview.postMessage(state);
    }

    dispose(): void { this.panel.dispose(); }
}
