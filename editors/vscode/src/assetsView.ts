import * as vscode from 'vscode';
import type { DesignerPanel } from './designerPanel';
import { DockView } from './dockView';
import { ProjectAssets } from './projectAssets';
import { errorMessage, type AssetsState, type DragOrigin, type FrameTarget, type FromAssets, type ProjectAsset, type ToAssets } from './protocol';
import { assetsHtml } from './view';

export const ASSETS_VIEW = 'ferroui.assets';

interface Query {
    key: string;
    assets?: ProjectAsset[];
    error?: string;
}

export class AssetsView {
    readonly view: DockView<AssetsState, FromAssets, ToAssets>;
    private readonly subscription: vscode.Disposable;
    private folder?: vscode.WorkspaceFolder;
    private cache?: Query;

    constructor(root: vscode.Uri, readonly library: ProjectAssets, private readonly changed: () => void, receive: (message: FromAssets) => void) {
        this.view = new DockView(root, assetsHtml, changed, receive);
        this.subscription = library.onDidChange(() => { this.cache = undefined; changed(); });
    }

    update(panel?: DesignerPanel, frame?: FrameTarget, canInsert = false): void {
        const folder = panel ? vscode.workspace.getWorkspaceFolder(panel.document.uri) : vscode.workspace.workspaceFolders?.[0];
        this.folder = folder;
        if (!folder || folder.uri.scheme !== 'file') {
            this.view.show({ assets: [], canInsert: false, message: 'Open a local FerroUI project to browse its files.' }, '');
            return;
        }
        if (!this.view.visible) return;
        const canPlace = panel?.language === 'xaml';
        const key = `${folder.uri}:${panel?.document.uri ?? ''}:${canPlace}`;
        if (this.cache?.key !== key) {
            const query: Query = { key };
            this.cache = query;
            void this.library.list(folder, panel?.document.uri, canPlace).then(assets => { query.assets = assets; },
                error => { query.error = errorMessage(error); }).finally(() => {
                if (this.cache === query) this.changed();
            });
        }
        this.view.show({
            frame: canPlace && panel?.snapshot?.image ? frame : undefined,
            assets: this.cache.assets ?? [], canInsert, folder: folder.uri.toString(),
            message: this.cache.error ?? (this.cache.assets ? undefined : 'Finding project assets…'),
        }, folder.name);
    }

    ownsImage(origin: Extract<DragOrigin, { type: 'image' }>): boolean {
        return this.cache?.assets?.some(asset => asset.id === origin.asset && asset.source === origin.source) ?? false;
    }

    async browse(action: 'preview' | 'copy' | 'open', id: string): Promise<void> {
        const folder = this.folder;
        if (!folder) return;
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
            await vscode.commands.executeCommand('vscode.open', asset.uri);
        }
    }

    dispose(): void {
        this.subscription.dispose();
        this.view.dispose();
    }
}
