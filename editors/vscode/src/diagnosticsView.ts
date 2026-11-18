import * as path from 'node:path';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import type { ReloadHistory, ReloadRecord } from './reloadHistory';
import { RuntimeClient } from './runtimeClient';
import type { RuntimeConnections } from './runtimeConnections';
import { readRuntimeDiagnostics, runtimeLogText, type RuntimeFrameStats, type RuntimeLog } from './runtimeDiagnostics';
import type { RuntimeSourceEditor } from './runtimeSource';
import { isWorkspaceFile } from './workspacePaths';

const viewId = 'ferroui.diagnostics';
const captions = { applied: 'Reloaded in place', rejected: 'Edit rejected', rebuilt: 'Candidate activated', buildFailed: 'Candidate failed',
    superseded: 'Superseded by a newer save', stopped: 'Session stopped' };
type Item = { type: 'group'; group: 'reload' | 'binding' | 'layout' }
    | { type: 'notice'; label: string }
    | { type: 'reload'; record: ReloadRecord }
    | { type: 'binding'; log: RuntimeLog }
    | { type: 'layout'; stats: RuntimeFrameStats };

export class DiagnosticsView implements vscode.TreeDataProvider<Item>, vscode.Disposable {
    private readonly subscriptions: vscode.Disposable[];
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private current?: Promise<Awaited<ReturnType<typeof readRuntimeDiagnostics>>>;

    constructor(private readonly history: ReloadHistory, private readonly runtimes: RuntimeConnections, private readonly source: RuntimeSourceEditor) {
        const view = vscode.window.createTreeView(viewId, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [view, view.onDidChangeVisibility(({ visible }) => { if (visible) this.refresh(); }),
            history.onDidChange(() => this.refresh()), runtimes.onDidChange(() => this.refresh()),
            vscode.commands.registerCommand('ferroui.diagnostics.refresh', () => this.refresh()),
            vscode.commands.registerCommand('ferroui.diagnostics.openSource', (item: Item) => this.openSource(item)),
        ];
    }

    async getChildren(item?: Item): Promise<Item[]> {
        if (!item) return [{ type: 'group', group: 'reload' }, { type: 'group', group: 'binding' }, { type: 'group', group: 'layout' }];
        if (item.type !== 'group') return [];
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
            const title = { reload: 'Hot Reload History', binding: 'Binding Errors', layout: 'Runtime Layout' }[item.group];
            return new vscode.TreeItem(title, vscode.TreeItemCollapsibleState.Collapsed);
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
            row.description = record.outcome ? `${captions[record.outcome]} · ${record.elapsedMs} ms` : 'Waiting for the application';
            row.tooltip = [record.file, record.pid ? `PID ${record.pid}` : undefined, record.message].filter(Boolean).join('\n');
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
        if (item?.type === 'reload') {
            const uri = vscode.Uri.file(item.record.file);
            if (await isWorkspaceFile(uri)) await vscode.commands.executeCommand('vscode.open', uri);
        } else if (item?.type === 'binding' && item.log.source) {
            const anchor = await this.source.resolve(item.log.source);
            if (anchor) await vscode.window.showTextDocument(anchor.document, { selection: new vscode.Range(anchor.position, anchor.position) });
        }
    }

    private refresh(): void { this.current = undefined; this.changed.fire(); }
    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); this.changed.dispose(); }
}
