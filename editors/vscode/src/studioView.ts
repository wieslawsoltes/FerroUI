import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { StudioContext, type StudioItem } from './studioContext';
import { behaviorItems } from './studioBehaviors';
import { contentSections } from './studioContent';
import { focusItems } from './studioFocus';
import { layerItems } from './studioLayers';
import { stateItems } from './studioStates';
import { timelineItems } from './studioTimeline';

const viewId = 'ferroui.studio';
const runId = 'ferroui.studio.runAction';

export class StudioView implements vscode.TreeDataProvider<StudioItem>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private readonly view: vscode.TreeView<StudioItem>;
    private readonly subscriptions: vscode.Disposable[];
    private key?: string;
    private panel?: DesignerPanel;

    constructor(visible: () => void) {
        this.view = vscode.window.createTreeView(viewId, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [this.view, this.changed, this.view.onDidChangeVisibility(visible),
            vscode.commands.registerCommand('ferroui.showStudio', () => vscode.commands.executeCommand(`${viewId}.focus`)),
            vscode.commands.registerCommand(runId, async (item: StudioItem) => {
                try { await item.run?.(); }
                catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
        ];
    }

    get visible(): boolean { return this.view.visible; }

    update(panel: DesignerPanel | undefined, description: string): void {
        this.panel = panel;
        this.view.description = description;
        const key = panel ? `${panel.session}:${panel.currentToken}:${panel.selectedIds.join(',')}` : '';
        if (this.key === key) return;
        this.key = key;
        this.view.message = !panel ? 'Open a FerroUI designer.' : panel.currentToken === undefined ? (panel.status ?? 'Rendering the design…') : undefined;
        this.changed.fire();
    }

    getTreeItem(item: StudioItem): vscode.TreeItem {
        const row = new vscode.TreeItem(item.label, item.children ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None);
        row.id = item.id;
        row.description = item.description;
        if (item.icon) row.iconPath = new vscode.ThemeIcon(item.icon);
        if (item.run) row.command = { command: runId, title: item.label, arguments: [item] };
        return row;
    }

    async getChildren(item?: StudioItem): Promise<StudioItem[]> {
        if (item) {
            try { return await item.children?.() ?? []; }
            catch (error) { return [{ id: `${item.id}:error`, label: errorMessage(error), icon: 'warning' }]; }
        }
        const panel = this.panel;
        const token = panel?.currentToken;
        if (!panel || token === undefined) return [];
        const context = new StudioContext(panel, token, () => this.panel === panel);
        return [
            ...contentSections(context),
            { id: 'layers', label: 'Layers', icon: 'layers', children: () => layerItems(context) },
            { id: 'focus', label: 'Focus and Labels', icon: 'list-ordered', children: () => focusItems(context) },
            { id: 'timeline', label: 'Timeline', icon: 'history', children: () => timelineItems(context) },
            { id: 'states', label: 'States', icon: 'symbol-enum', children: () => stateItems(context) },
            { id: 'behaviors', label: 'Behaviors', icon: 'run', children: () => behaviorItems(context) },
        ];
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
