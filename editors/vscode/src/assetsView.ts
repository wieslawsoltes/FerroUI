import * as path from 'node:path';
import * as vscode from 'vscode';
import { assetFormat } from './assetFormats';
import type { BuiltProject } from './builtProjects';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { ProjectAssets, rasterThumbnail } from './projectAssets';
import { errorMessage, imageSourceProperties, type AssetsState, type DragOrigin, type FrameTarget, type FromAssets, type ProjectAsset, type ToAssets } from './protocol';
import { assetsHtml } from './view';

export const ASSETS_VIEW = 'ferroui.assets';

interface Query {
    key: string;
    project?: BuiltProject;
    assets?: ProjectAsset[];
    error?: string;
}

export class AssetsView {
    readonly view: DockView<AssetsState, FromAssets, ToAssets>;
    private readonly subscription: vscode.Disposable;
    private folder?: vscode.WorkspaceFolder;
    private panel?: DesignerPanel;
    private cache?: Query;

    constructor(root: vscode.Uri, readonly library: ProjectAssets, private readonly changed: () => void, receive: (message: FromAssets) => void) {
        this.view = new DockView(root, assetsHtml, changed, receive);
        this.subscription = library.onDidChange(() => { this.cache = undefined; changed(); });
    }

    update(panel?: DesignerPanel, frame?: FrameTarget, canInsert = false): void {
        const folder = panel ? vscode.workspace.getWorkspaceFolder(panel.document.uri) : vscode.workspace.workspaceFolders?.[0];
        this.folder = folder;
        this.panel = panel;
        if (!folder || folder.uri.scheme !== 'file') {
            this.view.show({ assets: [], canInsert: false, message: 'Open a local FerroUI project to browse its files.' }, '');
            return;
        }
        if (!this.view.visible) return;
        const canPlace = panel !== undefined;
        const key = `${folder.uri}:${panel?.document.uri ?? ''}:${canPlace}`;
        if (this.cache?.key !== key || this.cache.project !== panel?.publishedAssetProject) {
            const query: Query = { key, project: panel?.publishedAssetProject };
            this.cache = query;
            void this.list(folder, panel).then(assets => { query.assets = assets; },
                error => { query.error = errorMessage(error); }).finally(() => {
                if (this.cache === query) this.changed();
            });
        }
        const accepted = panel?.snapshot?.image ? frame : undefined;
        const selected = panel?.selectedNode;
        this.view.show({
            frame: accepted,
            imageTarget: frame && selected && panel?.selectedIds.length === 1 && panel.canUse(frame)
                && imageSourceProperties(selected).length > 0 ? { ...frame, node: selected.id, selection: [selected.id] } : undefined,
            assets: this.cache.assets ?? [], canInsert, folder: folder.uri.toString(),
            message: this.cache.error ?? (this.cache.assets ? undefined : 'Finding project assets…'),
        }, folder.name);
    }

    ownsImage(origin: Extract<DragOrigin, { type: 'image' }>): boolean {
        return this.cache?.assets?.some(asset => asset.id === origin.asset && asset.source === origin.source) ?? false;
    }

    async list(folder: vscode.WorkspaceFolder, panel?: DesignerPanel): Promise<ProjectAsset[]> {
        const [files, published] = await Promise.all([
            this.library.list(folder, panel?.document.uri, Boolean(panel)), panel?.publishedAssets() ?? [],
        ]);
        const packaged = published.flatMap<ProjectAsset>(({ address, assembly, key, sourceRoot }) => {
            const format = assetFormat(key);
            if (!format) return [];
            return [{ id: address, label: path.posix.basename(key), path: `${assembly} · ${key}`,
                kind: format.kind, relative: address, published: true, thumbnail: Boolean(format.mime),
                movable: format.kind !== 'XAML' && Boolean(sourceRoot && vscode.workspace.getWorkspaceFolder(vscode.Uri.file(sourceRoot))),
                source: format.mime ? address : undefined }];
        });
        return [...packaged, ...files];
    }

    async imageSource(panel: DesignerPanel, id: string): Promise<string> {
        if (!isPublished(id)) return this.library.imageSource(panel.document.uri, id);
        if (!(await panel.publishedAsset(id)).raster) throw new Error('Choose a raster image from the current Assets view.');
        return id;
    }

    async browse(action: 'preview' | 'copy' | 'open' | 'move', id: string): Promise<void> {
        const folder = this.folder;
        if (!folder) return;
        if (isPublished(id)) {
            const panel = this.panel;
            if (!panel) throw new Error('Use the published asset from its owning Cargo design.');
            const input = await panel.publishedAsset(id);
            const file = vscode.Uri.parse(input.uri, true);
            if (this.panel !== panel) return;
            if (action === 'preview') {
                const uri = await rasterThumbnail(file, input.raster ? assetFormat(file.fsPath)?.mime : undefined);
                if (this.panel === panel) this.view.post({ type: 'thumbnail', id, uri });
            } else if (action === 'copy') {
                await vscode.env.clipboard.writeText(id);
                this.view.post({ type: 'assetStatus', message: 'Copied packaged address.' });
            } else if (action === 'move') await vscode.commands.executeCommand('ferroui.moveAsset', file);
            else await vscode.commands.executeCommand('vscode.open', file);
            return;
        }
        if (action === 'preview') {
            const uri = await this.library.thumbnail(folder, id);
            if (this.folder?.uri.toString() === folder.uri.toString()) this.view.post({ type: 'thumbnail', id, uri });
        } else if (action === 'copy') {
            await this.library.resolve(folder, id);
            const relative = this.cache?.assets?.find(asset => asset.id === id)?.relative ?? id;
            await vscode.env.clipboard.writeText(relative);
            this.view.post({ type: 'assetStatus', message: 'Copied relative path.' });
        } else {
            const asset = await this.library.resolve(folder, id);
            if (action === 'move') {
                const source = vscode.Uri.joinPath(folder.uri, ...asset.path.split('/'));
                const command = asset.kind === 'XAML' ? 'ferroui.moveXamlFile' : 'ferroui.moveAsset';
                await vscode.commands.executeCommand(command, source);
            } else await vscode.commands.executeCommand('vscode.open', asset.uri);
        }
    }

    dispose(): void {
        this.subscription.dispose();
        this.view.dispose();
    }
}

function isPublished(id: string): boolean { return /^ferres:\/\//i.test(id); }
