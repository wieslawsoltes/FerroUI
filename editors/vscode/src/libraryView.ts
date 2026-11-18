import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { errorMessage } from './protocol';
import { action, StudioContext, type StudioItem } from './studioContext';
import { componentActions, componentKind, createTemplate, findDefinition, readLibrary, type Component } from './studioLibrary';

export const LIBRARY_VIEW = 'ferroui.library';
const run = 'ferroui.library.runAction';

export class LibraryView implements vscode.TreeDataProvider<StudioItem>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private readonly view: vscode.TreeView<StudioItem>;
    private readonly subscriptions: vscode.Disposable[];
    private panel?: DesignerPanel;
    private key?: string;

    constructor(visible: () => void) {
        this.view = vscode.window.createTreeView(LIBRARY_VIEW, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [this.view, this.changed, this.view.onDidChangeVisibility(visible),
            vscode.commands.registerCommand(run, async (item: StudioItem) => {
                try { await item.run?.(); }
                catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
            })];
    }

    get visible(): boolean { return this.view.visible; }

    update(panel: DesignerPanel | undefined, file: string): void {
        this.panel = panel;
        if (this.view.description !== file) this.view.description = file;
        const key = panel ? `${panel.session}:${panel.currentToken}:${panel.selectedIds.join(',')}` : '';
        if (key === this.key) return;
        this.key = key;
        this.view.message = !panel ? 'Open a FerroUI design to browse its definitions.' : panel.status;
        this.changed.fire();
    }

    getTreeItem(item: StudioItem): vscode.TreeItem {
        const row = new vscode.TreeItem(item.label, item.children ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None);
        row.id = item.id;
        row.description = item.description;
        if (item.icon) row.iconPath = new vscode.ThemeIcon(item.icon);
        if (item.run) row.command = { command: run, title: item.label, arguments: [item] };
        return row;
    }

    async getChildren(item?: StudioItem): Promise<StudioItem[]> {
        if (item) return await item.children?.() ?? [];
        const panel = this.panel;
        const token = panel?.currentToken;
        if (!panel || token === undefined || panel.status) return [];
        const context = new StudioContext(panel, token, () => this.panel === panel);
        const key = this.key;
        try {
            const { components, scope } = await readLibrary(context);
            const items: StudioItem[] = [];
            if (scope.definition) {
                items.push({ ...action('document', 'Return to document preview',
                    () => context.present('designer_definition', { action: 'exit' }), 'arrow-left'), description: scope.definition.title });
            }
            if (components.length) items.push(action('find', 'Find Definition…', () => findDefinition(context, components), 'search'));
            const owner = panel.nodes[0]?.id;
            if (context.writable && owner !== undefined) {
                items.push(action('create', 'New Template…', () => createTemplate(context, owner), 'add'));
            }
            const groups = new Map<string, Component[]>();
            for (const component of components) {
                const entries = groups.get(component.kind) ?? [];
                entries.push(component);
                groups.set(component.kind, entries);
            }
            return items.concat([...groups].map(([kind, entries]) => ({
                id: `kind:${kind}`, label: componentKind(kind), description: String(entries.length), icon: 'library',
                children: async () => entries.map(component => ({
                    id: `component:${component.node}`,
                    label: component.display_name,
                    description: [component.target_type, component.usages && `${component.usages} uses`].filter(Boolean).join(' · '),
                    children: async () => componentActions(context, component),
                })),
            })));
        } catch (error) {
            return key === this.key ? [{ id: 'library:error', label: errorMessage(error), icon: 'warning' }] : [];
        }
    }

    dispose(): void { this.subscriptions.forEach(subscription => subscription.dispose()); }
}
