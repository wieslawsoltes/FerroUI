import * as path from 'node:path';
import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import type { DesignerWorkbench } from './designerWorkbench';
import { errorMessage, type PreviewFinding } from './protocol';
import type { ReloadHistory, ReloadRecord } from './reloadHistory';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections } from './runtimeConnections';
import { readRuntimeDiagnostics, runtimeLogText, type RuntimeFrameStats, type RuntimeLog } from './runtimeDiagnostics';
import type { RuntimeSourceEditor } from './runtimeSource';
import { isWorkspaceFile } from './workspacePaths';

const viewId = 'ferroui.diagnostics';
const captions = { applied: 'Reloaded in place', rejected: 'Edit rejected', rebuilt: 'Candidate activated', buildFailed: 'Candidate failed',
    superseded: 'Superseded by a newer save', stopped: 'Session stopped' };
const frameCaptions = { pending: 'Waiting for a native frame', rendered: 'Native frame rendered', noSceneUpdate: 'No scene drew in this batch',
    discarded: 'Render batch discarded', processExited: 'Process ended before the frame receipt' };
type Item = { type: 'group'; group: 'preview' | 'reload' | 'binding' | 'layout' }
    | { type: 'notice'; label: string }
    | { type: 'reload'; record: ReloadRecord }
    | { type: 'binding'; log: RuntimeLog }
    | { type: 'preview'; panel: DesignerPanel; token: number; finding: PreviewFinding }
    | { type: 'layout'; stats: RuntimeFrameStats };

export class DiagnosticsView implements vscode.TreeDataProvider<Item>, vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private current?: Promise<Awaited<ReturnType<typeof readRuntimeDiagnostics>>>;

    constructor(private readonly history: ReloadHistory, private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor,
        private readonly designer: DesignerWorkbench) {
        const view = vscode.window.createTreeView(viewId, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [view, view.onDidChangeVisibility(({ visible }) => { if (visible) this.refresh(); }),
            history.onDidChange(() => this.refresh()), runtimes.onDidChange(() => this.refresh()),
            designer.onDidChangePreview(() => this.changed.fire()),
            vscode.commands.registerCommand('ferroui.diagnostics.refresh', () => this.refresh()),
            vscode.commands.registerCommand('ferroui.diagnostics.openSource', (item: Item) => this.openSource(item)),
        ];
    }

    async getChildren(item?: Item): Promise<Item[]> {
        if (!item) return [{ type: 'group', group: 'preview' }, { type: 'group', group: 'reload' },
            { type: 'group', group: 'binding' }, { type: 'group', group: 'layout' }];
        if (item.type !== 'group') return [];
        if (item.group === 'preview') return this.previewChecks();
        if (item.group === 'reload') {
            const recent = this.history.recent(50);
            return recent.length ? recent.map(record => ({ type: 'reload', record })) : [{ type: 'notice', label: 'Save a file while a managed app is running' }];
        }
        const endpoint = this.runtimes.get('managed') ?? this.runtimes.get('attached');
        if (!endpoint) return [{ type: 'notice', label: 'Start or attach a FerroUI application with MCP' }];
        try {
            if (!this.current) {
                const client = new RuntimeClient(endpoint);
                this.current = readRuntimeDiagnostics(client).finally(() => client.dispose());
            }
            const request = this.current;
            const { frames, logs } = await request;
            if (request !== this.current) return [];
            if (item.group === 'layout') return frames.map(stats => ({ type: 'layout', stats }));
            const binding = logs.entries.filter(entry => entry.area === 'Binding').slice(-100).reverse();
            return binding.length ? binding.map(log => ({ type: 'binding', log })) : [{ type: 'notice', label: 'No binding errors reported' }];
        } catch (error) { return [{ type: 'notice', label: errorMessage(error) }]; }
    }

    getTreeItem(item: Item): vscode.TreeItem {
        if (item.type === 'notice') return new vscode.TreeItem(item.label);
        if (item.type === 'group') {
            const title = { preview: 'Preview Checks', reload: 'Hot Reload History', binding: 'Binding Errors', layout: 'Runtime Layout' }[item.group];
            const row = new vscode.TreeItem(title, vscode.TreeItemCollapsibleState.Collapsed);
            row.id = item.group;
            return row;
        }
        if (item.type === 'preview') {
            const { finding } = item;
            const row = new vscode.TreeItem(finding.summary ?? finding.message);
            row.id = `preview:${item.panel.session}:${finding.node}:${finding.code}:${finding.property ?? ''}`;
            row.description = finding.name ?? finding.control;
            row.tooltip = `${row.description}\n${finding.message}`;
            row.accessibilityInformation = { label: `${row.description}. ${finding.message}` };
            row.iconPath = new vscode.ThemeIcon(finding.code === 'ineffectiveLayoutValue' ? 'info' : 'warning');
            row.command = { command: 'ferroui.diagnostics.openSource', title: 'Reveal Source', arguments: [item] };
            return row;
        }
        if (item.type === 'layout') {
            const { stats } = item;
            const row = new vscode.TreeItem(`Window ${stats.node_id}`);
            const timing = stats.layout_timing_available ?? stats.layout_pass_count > 0;
            row.description = `${timing ? `${stats.layout_ms.toFixed(2)} ms layout · ` : ''}${stats.layout_pass_count} passes · ${stats.rendered_scene_updates} scene updates`;
            row.iconPath = new vscode.ThemeIcon('graph');
            return row;
        }
        if (item.type === 'reload') {
            const { record } = item;
            const row = new vscode.TreeItem(path.basename(record.file));
            row.id = `reload:${record.id}`;
            const { frame } = record;
            const outcome = record.outcome && captions[record.outcome];
            const frameTime = frame?.elapsedMs === undefined ? undefined : `${frame.elapsedMs} ms to frame`;
            const frameOutcome = frame && frameCaptions[frame.status];
            const timing = frameTime ?? frameOutcome ?? `${record.acknowledgedMs ?? record.elapsedMs} ms`;
            row.description = outcome ? `${outcome} · ${timing}` : 'Waiting for the application';
            const resultTime = record.acknowledgedMs !== undefined ? `Application acknowledged in ${record.acknowledgedMs} ms`
                : record.elapsedMs === undefined ? undefined : `${outcome} in ${record.elapsedMs} ms`;
            const batch = frame?.batch !== undefined ? `Compositor batch ${frame.batch}` : undefined;
            row.tooltip = [record.file, record.pid ? `PID ${record.pid}` : undefined,
                record.generation ? `Process generation ${record.generation}` : undefined, resultTime, frameOutcome, frameTime, batch, record.message].filter(Boolean).join('\n');
            row.accessibilityInformation = { label: [row.label, row.description, frameOutcome, batch].filter(Boolean).join('. ') };
            row.iconPath = new vscode.ThemeIcon(record.outcome === 'rejected' || record.outcome === 'buildFailed' ? 'error' : 'history');
            row.command = { command: 'ferroui.diagnostics.openSource', title: 'Reveal Source', arguments: [item] };
            return row;
        }
        const message = runtimeLogText(item.log);
        const row = new vscode.TreeItem(message);
        row.tooltip = message;
        row.iconPath = new vscode.ThemeIcon('warning');
        if (item.log.source) row.command = { command: 'ferroui.diagnostics.openSource', title: 'Reveal Source', arguments: [item] };
        return row;
    }

    private async openSource(item: Item): Promise<void> {
        if (item?.type === 'preview') {
            const { panel, token } = item;
            const node = panel.nodes.find(node => node.id === item.finding.node);
            if (node && this.designer.activeDesign === panel) {
                const frame = { session: panel.session, token };
                await panel.select({ ...frame, node: node.id });
                await panel.revealSource(frame, item.finding.range ?? node.nameRange);
            }
        } else if (item?.type === 'reload') {
            const uri = vscode.Uri.file(item.record.file);
            if (await isWorkspaceFile(uri)) await vscode.commands.executeCommand('vscode.open', uri);
        } else if (item?.type === 'binding' && item.log.source) {
            const anchor = await this.source.resolve(item.log.source);
            if (anchor) await vscode.window.showTextDocument(anchor.document, { selection: new vscode.Range(anchor.position, anchor.position) });
        }
    }

    private async previewChecks(): Promise<Item[]> {
        const panel = this.designer.activeDesign;
        if (!panel) return [{ type: 'notice', label: 'Open a FerroUI designer' }];
        const token = panel.currentToken;
        if (token === undefined || panel.status) return [{ type: 'notice', label: panel.status ?? 'Waiting for preview' }];
        try {
            const result = await panel.studioQuery('designer_preview_checks', {}, undefined, AbortSignal.timeout(15_000)) as { findings: PreviewFinding[] };
            if (this.designer.activeDesign !== panel || !panel.canUse({ session: panel.session, token })) return [];
            if (!result.findings.length) return [{ type: 'notice', label: 'No findings in this preview' }];
            return result.findings.map(finding => ({ type: 'preview', panel, token, finding }));
        } catch (error) { return [{ type: 'notice', label: errorMessage(error) }]; }
    }

    private refresh(): void { this.current = undefined; this.changed.fire(); }
    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); this.changed.dispose(); }
}
