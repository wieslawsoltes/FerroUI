import * as vscode from 'vscode';
import type { TokenDirection, TokenOptions, TokenPlan } from './designTokenProtocol';
import type { ProjectInventory } from './projectInventory';
import { errorMessage } from './protocol';
import { choose } from './studioContext';
import { localFolders } from './workspacePaths';
import { WorkspaceTools } from './workspaceTools';

const scheme = 'ferroui-token-review';
const title = 'FerroUI Design Tokens';

export class DesignTokenCommands implements vscode.Disposable {
    private readonly content = new Map<string, string>();
    private readonly files = new WorkspaceTools();
    private readonly subscriptions: vscode.Disposable[];
    private nextPreview = 0;

    constructor(private readonly projects: ProjectInventory) {
        this.subscriptions = [
            vscode.workspace.registerTextDocumentContentProvider(scheme, {
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
            const extension = direction === 'import' ? '.tokens.json' : '.xaml';
            const active = vscode.window.activeTextEditor?.document.uri;
            const uri = explicit ?? (active?.scheme === 'file' && active.path.toLowerCase().endsWith(extension) ? active
                : (await vscode.window.showOpenDialog({ canSelectMany: false,
                    filters: direction === 'import' ? { 'Design tokens': ['json'] } : { 'XAML resource dictionary': ['xaml'] },
                }))?.[0]);
            if (!uri) return;
            const document = await vscode.workspace.openTextDocument(uri);
            const sourceEditor = await vscode.window.showTextDocument(document, { preview: false });
            const reportColumn = sourceEditor.viewColumn;
            let candidateColumn: vscode.ViewColumn = vscode.ViewColumn.Beside;
            const version = document.version;
            const options: TokenOptions = { mappings: Object.create(null) as TokenOptions['mappings'] };
            const destination = direction === 'import' ? 'DesignTokens.xaml' : 'design.tokens.json';
            const source = document.getText();
            const requireSource = () => {
                if (document.isClosed || document.version !== version) throw new Error('The source changed. Reopen the token conversion.');
            };
            while (true) {
                requireSource();
                const plan = await this.projects.designTokens({ direction, source, options }, AbortSignal.timeout(20_000));
                requireSource();
                const query = String(++this.nextPreview);
                const report = await this.preview('Token Mappings.md', query, mappingReport(direction, plan));
                const candidate = await this.preview(destination, query, plan.source);
                await vscode.window.showTextDocument(report, { preview: true, viewColumn: reportColumn });
                const editor: vscode.TextEditor = await vscode.window.showTextDocument(candidate, { preview: true, viewColumn: candidateColumn });
                candidateColumn = editor.viewColumn ?? candidateColumn;
                const saveLabel = direction === 'import' ? 'Create XAML dictionary…' : 'Create design tokens JSON…';
                const actions = [
                    ...(plan.mappings.length ? [{ label: saveLabel, description: `${plan.mappings.length} mapped · ${plan.issues.length} skipped`, action: 'save' }] : []),
                    { label: 'Edit a token mapping…', action: 'mapping' },
                    ...(direction === 'import' ? [{ label: 'Set logical pixels per rem…', action: 'scale' }] : []),
                ];
                const choice = await choose(title, actions);
                if (!choice) return;
                if (choice.action === 'save') {
                    await this.save(document.uri, destination, plan, requireSource);
                    return;
                }
                if (choice.action === 'mapping') await editMapping(direction, plan, options);
                if (choice.action === 'scale') {
                    const value = await vscode.window.showInputBox({ title, prompt: 'Logical pixels for one rem',
                        value: String(options.pixelsPerRem ?? 16), validateInput: value => Number(value) > 0 && Number.isFinite(Number(value)) ? undefined : 'Enter a positive finite number.' });
                    if (value !== undefined) options.pixelsPerRem = Number(value);
                }
            }
        } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private preview(name: string, query: string, source: string): Thenable<vscode.TextDocument> {
        const uri = vscode.Uri.from({ scheme, path: `/${name}`, query });
        this.content.set(uri.toString(), source);
        return vscode.workspace.openTextDocument(uri);
    }

    private async save(original: vscode.Uri, name: string, plan: TokenPlan, requireSource: () => void): Promise<void> {
        const folders = localFolders();
        if (!folders.length) throw new Error('Open a workspace to save the reviewed token conversion.');
        const owner = vscode.workspace.getWorkspaceFolder(original);
        const folder = owner?.uri.scheme === 'file' ? owner : folders.length === 1 ? folders[0]
            : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Select a workspace folder for the token conversion' });
        if (!folder) return;
        const extension = name.endsWith('.json') ? '.json' : '.xaml';
        const destination = await vscode.window.showInputBox({ title, prompt: `New file path relative to ${folder.name}`, value: name,
            validateInput: value => value.trim().toLowerCase().endsWith(extension) ? undefined : `Choose a ${extension} file.` });
        if (!destination) return;
        requireSource();
        await this.files.call('ferroui_workspace_create', { path: destination.trim(),
            folder: folder.uri.toString(), source: plan.source, open: true }, AbortSignal.timeout(15_000));
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.content.clear();
    }
}

async function editMapping(direction: TokenDirection, plan: TokenPlan, options: TokenOptions): Promise<void> {
    const importing = direction === 'import';
    const rows = plan.mappings.map(mapping => ({ label: importing ? mapping.token : mapping.key,
        description: importing ? mapping.key : mapping.token, detail: `${mapping.kind} · ${mapping.xamlType}`, mapping }));
    const skipped = plan.issues.filter(issue => issue.path && !rows.some(row => row.label === issue.path))
        .map(issue => ({ label: issue.path, description: undefined, detail: issue.message, mapping: undefined }));
    const row = await choose('Choose a token or resource key', [...rows, ...skipped]);
    if (!row) return;
    const value = await vscode.window.showInputBox({ title, prompt: importing ? 'XAML resource key' : 'Dotted DTCG token path', value: row.description ?? row.label });
    if (!value?.trim()) return;
    if (importing) {
        const mapping = { ...options.mappings[row.label], key: value.trim() };
        if (row.mapping?.kind === 'color') {
            const form = await choose('XAML color representation', ['SolidColorBrush', 'Color'].map(label => ({ label })));
            if (!form) return;
            mapping.xamlType = form.label;
        }
        options.mappings[row.label] = mapping;
    } else {
        const mapping = { ...options.mappings[row.label], token: value.trim() };
        if (row.mapping?.xamlType === 'x:Double') {
            const form = await choose('DTCG numeric representation', [{ label: 'Number', tokenKind: 'number' }, { label: 'Dimension in logical pixels', tokenKind: 'dimension' }]);
            if (!form) return;
            mapping.kind = form.tokenKind;
        }
        options.mappings[row.label] = mapping;
    }
}

function mappingReport(direction: TokenDirection, plan: TokenPlan): string {
    const cell = (value: string) => value.replaceAll('\\', '\\\\').replaceAll('|', '\\|').replace(/[\r\n]/g, ' ');
    const headings = ['Token path', 'XAML key', 'DTCG type', 'XAML type', 'Value', 'Conversion'];
    const table = (columns: string[]) => `| ${columns.map(cell).join(' | ')} |`;
    const rows = plan.mappings.map(row => table([row.token, row.key, row.kind, row.xamlType, row.value, row.note ?? '']));
    const warnings = plan.issues.map(issue => table([issue.path || 'Document', issue.message]));
    return [`# ${direction === 'import' ? 'Import' : 'Export'} design tokens`, '', `${rows.length} mapped, ${warnings.length} skipped`, '',
        table(headings), table(headings.map(() => '---')), ...rows, '',
        ...(warnings.length ? ['## Skipped entries', '', table(['Source', 'Reason']), table(['---', '---']), ...warnings, ''] : [])].join('\n');
}
