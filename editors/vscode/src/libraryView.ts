import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { findLibraryDefinition, type DefinitionChoice } from './librarySearch';
import type { ProjectDefinition, ProjectLibrary } from './projectLibrary';
import { errorMessage } from './protocol';
import { action, StudioContext, type StudioItem } from './studioContext';
import { componentActions, componentKind, createTemplate, previewDefinition, readLibrary, type Component } from './studioLibrary';
import { localFolders } from './workspacePaths';

export const LIBRARY_VIEW = 'ferroui.library';
const run = 'ferroui.library.runAction';
const pageSize = 100;
const kindIcons: Record<string, string> = {
    UserControl: 'symbol-class',
    CustomControl: 'symbol-class',
    ControlTemplate: 'symbol-structure',
    DataTemplate: 'symbol-interface',
    ControlTheme: 'color-mode',
    Style: 'paintcan',
    Resource: 'symbol-property',
    ResourceDictionary: 'folder-library',
};

export type DefinitionDestination = 'preview' | 'source';

export class LibraryView implements vscode.TreeDataProvider<StudioItem>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private readonly view: vscode.TreeView<StudioItem>;
    private readonly subscriptions: vscode.Disposable[];
    private readonly pages = new Map<string, Promise<StudioItem[]>>();
    private panel?: DesignerPanel;
    private key?: string;
    private refresh?: ReturnType<typeof setTimeout>;

    constructor(visible: () => void, private readonly editStyle: (panel: DesignerPanel, node: number) => Promise<void>,
        private readonly project: ProjectLibrary, private readonly open: (entry: ProjectDefinition, destination: DefinitionDestination) => Promise<void>) {
        this.view = vscode.window.createTreeView(LIBRARY_VIEW, { treeDataProvider: this, showCollapseAll: true });
        this.subscriptions = [this.view, this.changed, this.view.onDidChangeVisibility(visible),
            this.project.onDidChange(() => this.invalidateProject()),
            vscode.workspace.onDidChangeWorkspaceFolders(() => this.invalidateProject()),
            vscode.commands.registerCommand(run, (item: StudioItem) => this.report(item.run?.())),
            vscode.commands.registerCommand('ferroui.studio.findDefinition', () => this.report(this.findDefinition())),
        ];
    }

    get visible(): boolean { return this.view.visible; }

    update(panel: DesignerPanel | undefined, file: string): void {
        this.panel = panel;
        if (this.view.description !== file) this.view.description = file;
        this.updateMessage();
        const key = panel ? `${panel.session}:${panel.currentToken}:${panel.selectedIds.join(',')}` : '';
        if (key === this.key) return;
        this.key = key;
        this.changed.fire();
    }

    getTreeItem(item: StudioItem): vscode.TreeItem {
        const state = item.id === 'current' ? vscode.TreeItemCollapsibleState.Expanded
            : item.children ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None;
        const row = new vscode.TreeItem(item.label, state);
        row.id = item.id;
        row.description = item.description;
        if (item.icon) row.iconPath = new vscode.ThemeIcon(item.icon);
        if (item.run) row.command = { command: run, title: item.label, arguments: [item] };
        return row;
    }

    async getChildren(item?: StudioItem): Promise<StudioItem[]> {
        if (item) return await item.children?.() ?? [];
        const folders = localFolders();
        const sections: StudioItem[] = [];
        if (this.panel) sections.push({ id: 'current', label: 'Current document', icon: 'file-code', children: async () => {
            const context = this.context();
            return context ? await this.documentItems(context) : [{ id: 'current:status', label: this.panel?.status ?? 'Rendering the design…' }];
        } });
        if (folders.length) sections.push({
            id: 'project', label: 'Project', icon: 'folder-library', children: async () => folders.length === 1
                ? await this.projectItems(folders[0].uri.toString())
                : folders.map(folder => ({ id: `folder:${folder.uri}`, label: folder.name, icon: 'folder',
                    children: () => this.projectItems(folder.uri.toString()) })),
        });
        return sections;
    }

    private context(): StudioContext | undefined {
        const panel = this.panel;
        const token = panel?.currentToken;
        if (panel && token !== undefined && !panel.status) return new StudioContext(panel, token, () => this.panel === panel);
    }

    private async documentItems(context: StudioContext): Promise<StudioItem[]> {
        const panel = context.panel;
        const key = this.key;
        try {
            const { components, scope } = await readLibrary(context);
            const items: StudioItem[] = [];
            if (scope.definition) {
                items.push({ ...action('document', 'Return to document preview',
                    () => context.present('designer_definition', { action: 'exit' }), 'arrow-left'), description: scope.definition.title });
            }
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
                id: `kind:${kind}`, label: componentKind(kind), description: String(entries.length), icon: kindIcons[kind] ?? 'library',
                children: async () => entries.map(component => ({
                    id: `component:${component.node}`,
                    label: component.display_name,
                    description: [component.target_type, component.usages && `${component.usages} ${component.usages === 1 ? 'use' : 'uses'}`]
                        .filter(Boolean).join(' · '),
                    children: async () => componentActions(context, component, async () => {
                        context.ensure();
                        await this.editStyle(panel, component.node);
                    }),
                })),
            })));
        } catch (error) {
            return key === this.key ? [{ id: 'library:error', label: errorMessage(error), icon: 'warning' }] : [];
        }
    }

    private projectItems(folder: string, offset = 0): Promise<StudioItem[]> {
        const key = `${folder}:${offset}`;
        let page = this.pages.get(key);
        if (!page) {
            page = this.readProjectItems(folder, offset);
            this.pages.set(key, page);
        }
        return page;
    }

    private async readProjectItems(folder: string, offset: number): Promise<StudioItem[]> {
        try {
            const page = await this.project.list(folder, '', offset, pageSize, AbortSignal.timeout(15_000));
            const rows: StudioItem[] = page.entries.map(entry => {
                const { definition, path } = entry;
                const id = `project:${folder}:${path}:${definition.range.start.line}:${definition.range.start.character}`;
                return { id, label: definition.name, description: path, icon: kindIcons[definition.kind],
                    children: async () => [
                        action(`${id}:preview`, 'Preview definition', () => this.open(entry, 'preview'), 'preview'),
                        action(`${id}:source`, 'Reveal definition in source', () => this.open(entry, 'source'), 'go-to-file'),
                    ] };
            });
            if (!rows.length) return [{ id: `project:${folder}:${offset}:empty`, label: 'No reusable XAML definitions' }];
            if (offset + rows.length < page.total) rows.push({ id: `more:${folder}:${offset}`, label: 'More definitions', icon: 'ellipsis',
                children: () => this.projectItems(folder, offset + pageSize) });
            return rows;
        } catch (error) {
            return [{ id: `project:${folder}:${offset}:error`, label: errorMessage(error), icon: 'warning' }];
        }
    }

    private async findDefinition(): Promise<void> {
        const context = this.context();
        const current: DefinitionChoice[] = [];
        if (context) {
            const library = await readLibrary(context).catch(() => undefined);
            for (const component of library?.components ?? []) current.push({ label: component.display_name,
                description: [componentKind(component.kind), component.target_type].filter(Boolean).join(' · '), alwaysShow: true,
                open: () => previewDefinition(context, component.node) });
        }
        const selected = await findLibraryDefinition(this.project, current, entry => this.open(entry, 'preview'));
        await selected?.open?.();
    }

    private async report(work?: Promise<void>): Promise<void> {
        try { await work; }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    private invalidateProject(): void {
        this.updateMessage();
        clearTimeout(this.refresh);
        this.pages.clear();
        if (this.view.visible) this.refresh = setTimeout(() => this.changed.fire(), 150);
    }

    private updateMessage(): void {
        const message = !this.panel && !localFolders().length ? 'Open a workspace folder or a FerroUI design.' : undefined;
        if (message !== this.view.message) this.view.message = message;
    }

    dispose(): void {
        clearTimeout(this.refresh);
        this.subscriptions.forEach(subscription => subscription.dispose());
    }
}
