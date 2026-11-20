import * as path from 'node:path';
import * as vscode from 'vscode';
import type { BuiltProjects } from './builtProjects';
import { describeCargoTarget, discoverCargoTargets, type CargoTarget } from './cargoTarget';
import type { HotReload } from './hotReload';
import type { ProjectFile, ProjectInventory } from './projectInventory';
import { errorMessage } from './protocol';
import { localFolders } from './workspacePaths';

const viewId = 'ferroui.project';
const categories = ['Cargo', 'Views', 'Assets', 'Stage Sets', 'Scenarios'] as const;
type Category = typeof categories[number];
const kindCategory: Record<ProjectFile['kind'], Category> = {
    cargo: 'Cargo', rsx: 'Views', xaml: 'Views', asset: 'Assets', stages: 'Stage Sets', scenario: 'Scenarios',
};

type Item = { type: 'root'; folder: vscode.WorkspaceFolder }
    | { type: 'category'; folder: vscode.WorkspaceFolder; category: Category }
    | { type: 'file'; folder: vscode.WorkspaceFolder; file: ProjectFile }
    | { type: 'target'; target: CargoTarget }
    | { type: 'notice'; label: string };

export class ProjectView implements vscode.TreeDataProvider<Item>, vscode.Disposable {
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChangeTreeData = this.changed.event;
    private readonly subscriptions: vscode.Disposable[];

    constructor(private readonly inventory: ProjectInventory, private readonly projects: BuiltProjects, private readonly reload: HotReload) {
        this.subscriptions = [vscode.window.createTreeView(viewId, { treeDataProvider: this, showCollapseAll: true }),
            inventory.onDidChange(() => this.changed.fire()), projects.onDidChange(() => this.changed.fire()),
            vscode.workspace.onDidGrantWorkspaceTrust(() => this.changed.fire()),
            vscode.commands.registerCommand('ferroui.project.refresh', () => inventory.refresh()),
            vscode.commands.registerCommand('ferroui.project.findView', () => this.command(() => this.findView())),
            vscode.commands.registerCommand('ferroui.project.openFile', (item: Item) => this.command(() => this.openFile(item))),
            vscode.commands.registerCommand('ferroui.project.moveXamlFile', (item: Item) => this.command(async () => {
                if (item?.type !== 'file' || item.file.kind !== 'xaml') return;
                await vscode.commands.executeCommand('ferroui.moveXamlFile', await inventory.resolve(item.folder, item.file));
            })),
            vscode.commands.registerCommand('ferroui.project.runTarget', (item: Item) => this.command(() => this.runTarget(item))),
        ];
    }

    async getChildren(item?: Item): Promise<Item[]> {
        if (!item) return localFolders().map(folder => ({ type: 'root', folder }));
        if (item.type === 'root') return categories.map(category => ({ type: 'category', folder: item.folder, category }));
        try {
            if (item.type === 'category') {
                const { files, truncated } = await this.inventory.read(item.folder);
                const children: Item[] = files.filter(file => kindCategory[file.kind] === item.category)
                    .map(file => ({ type: 'file', folder: item.folder, file }));
                if (truncated) children.push({ type: 'notice', label: 'Showing the first indexed project files' });
                if (!children.length) children.push({ type: 'notice', label: `No ${item.category.toLowerCase()} found` });
                return children;
            }
            if (item.type === 'file' && item.file.kind === 'cargo') {
                const targets = await discoverCargoTargets({ folder: item.folder.uri.toString(), manifest_path: item.file.path });
                return targets.length ? targets.map(target => ({ type: 'target', target })) : [{ type: 'notice', label: 'No runnable Cargo binaries' }];
            }
            return [];
        } catch (error) { return [{ type: 'notice', label: errorMessage(error) }]; }
    }

    getTreeItem(item: Item): vscode.TreeItem {
        const collapsed = vscode.TreeItemCollapsibleState.Collapsed;
        if (item.type === 'notice') return new vscode.TreeItem(item.label);
        if (item.type === 'root') {
            const row = new vscode.TreeItem(item.folder.name, vscode.TreeItemCollapsibleState.Expanded);
            row.id = item.folder.uri.toString();
            row.iconPath = new vscode.ThemeIcon('root-folder');
            return row;
        }
        if (item.type === 'category') {
            const row = new vscode.TreeItem(item.category, collapsed);
            row.id = `${item.folder.uri}#${item.category}`;
            row.iconPath = new vscode.ThemeIcon('folder');
            return row;
        }
        if (item.type === 'target') {
            const row = new vscode.TreeItem(item.target.label);
            row.description = item.target.packageName;
            row.tooltip = `Cargo binary ${item.target.binary}${item.target.features.length ? ` · ${item.target.features.join(', ')}` : ''}`;
            row.iconPath = new vscode.ThemeIcon('debug-start');
            row.contextValue = 'ferrouiProjectTarget';
            return row;
        }
        const { file, folder } = item;
        const uri = vscode.Uri.joinPath(folder.uri, ...file.path.split('/'));
        const cargo = file.kind === 'cargo';
        const packageName = cargo ? this.projects.forDocument(uri)?.metadata.package : undefined;
        const label = packageName ?? (cargo ? file.path : path.posix.basename(file.path));
        const row = new vscode.TreeItem(label, cargo ? collapsed : vscode.TreeItemCollapsibleState.None);
        row.resourceUri = uri;
        row.id = uri.toString();
        row.description = file.path === label ? undefined : file.path;
        row.contextValue = file.kind === 'xaml' ? 'ferrouiProjectXaml' : 'ferrouiProjectFile';
        row.command = { command: 'ferroui.project.openFile', title: 'Open Project File', arguments: [item] };
        return row;
    }

    private async findView(): Promise<void> {
        const roots = await Promise.all(localFolders().map(async folder => ({ folder, snapshot: await this.inventory.read(folder) })));
        const choices = roots.flatMap(({ folder, snapshot }) => snapshot.files.filter(file => kindCategory[file.kind] === 'Views').map(file => ({
            label: path.posix.basename(file.path), description: file.path, detail: folder.name, file, folder,
        })));
        if (!choices.length) { void vscode.window.showInformationMessage('No FerroUI views found in the open workspace.'); return; }
        const selected = await vscode.window.showQuickPick(choices, { title: 'Find FerroUI View', matchOnDescription: true, matchOnDetail: true });
        if (selected) await this.openFile({ type: 'file', file: selected.file, folder: selected.folder });
    }

    private async openFile(item: Item): Promise<void> {
        if (item?.type !== 'file') return;
        const uri = await this.inventory.resolve(item.folder, item.file);
        const designer = item.file.kind === 'rsx' || item.file.kind === 'xaml';
        await vscode.commands.executeCommand(designer ? 'ferroui.openDesigner' : 'vscode.open', uri);
    }

    private async runTarget(item: Item): Promise<void> {
        if (item?.type === 'target') await this.reload.agentControl('start', { ...describeCargoTarget(item.target), features: item.target.features }, AbortSignal.timeout(25_000));
    }

    private async command(action: () => Promise<void>): Promise<void> {
        try { await action(); }
        catch (error) { void vscode.window.showErrorMessage(errorMessage(error)); }
    }

    dispose(): void {
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}
