import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { StudioContext, type StudioItem } from './studioContext';
import { behaviorItems } from './studioBehaviors';
import { contentSections } from './studioContent';
import { findDefinition } from './studioLibrary';
import { focusItems } from './studioFocus';
import { chooseGridTrack, gridItems } from './studioGrid';
import { layerItems } from './studioLayers';
import { stateItems } from './studioStates';
import type { StateTransitionRoute } from './stateTransitionProtocol';
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

    constructor(visible: () => void, private readonly previewTransition: (context: StudioContext, route: StateTransitionRoute) => Promise<void>) {
        this.view = vscode.window.createTreeView(viewId, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [this.view, this.changed, this.view.onDidChangeVisibility(visible),
            vscode.commands.registerCommand('ferroui.showStudio', () => vscode.commands.executeCommand(`${viewId}.focus`)),
            vscode.commands.registerCommand('ferroui.studio.findDefinition', async () => {
                try {
                    const context = this.context();
                    if (!context) throw new Error('Open a FerroUI design before finding a definition.');
                    await findDefinition(context);
                } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
            vscode.commands.registerCommand('ferroui.studio.gridTracks', async () => {
                try {
                    const context = this.context();
                    if (!context) throw new Error('Open a FerroUI design before editing its Grid tracks.');
                    await chooseGridTrack(context);
                } catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            }),
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
        const context = this.context();
        if (!context) return [];
        return [
            ...contentSections(context),
            { id: 'layers', label: 'Layers', icon: 'layers', children: () => layerItems(context) },
            { id: 'grid', label: 'Grid Tracks', icon: 'layout', children: () => gridItems(context) },
            { id: 'focus', label: 'Focus and Labels', icon: 'list-ordered', children: () => focusItems(context) },
            { id: 'timeline', label: 'Timeline', icon: 'history', children: () => timelineItems(context) },
            { id: 'states', label: 'States', icon: 'symbol-enum', children: () => stateItems(context, this.previewTransition) },
            { id: 'behaviors', label: 'Behaviors', icon: 'run', children: () => behaviorItems(context) },
        ];
    }

    private context(): StudioContext | undefined {
        const panel = this.panel;
        const token = panel?.currentToken;
        if (panel && token !== undefined) return new StudioContext(panel, token, () => this.panel === panel);
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
