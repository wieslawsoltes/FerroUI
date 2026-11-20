import * as path from 'node:path';
import * as vscode from 'vscode';
import { AssetImport } from './assetImport';
import type { AssetMoves } from './assetMoves';
import type { TokenRequest } from './designTokenProtocol';
import { NativePreview } from './nativePreview';
import type { ProjectAssets } from './projectAssets';
import { isWorkspaceFile, workspaceFolders } from './workspacePaths';

interface NativeProjectFile { path: string; kind: 'cargo' | 'rsx' | 'xaml' | 'stages' | 'scenario'; invocations?: number }
export interface NativeProjectInventory { files: NativeProjectFile[]; truncated: boolean }
export type ProjectFile = NativeProjectFile | { path: string; kind: 'asset'; assetKind: string };
export interface ProjectInventorySnapshot { files: ProjectFile[]; truncated: boolean }
const ignoredDirectories = new Set(['target', '.git', 'node_modules', '.idea', '.vs']);

export class ProjectInventory implements vscode.Disposable {
    readonly assetImport = new AssetImport();
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly subscriptions: vscode.Disposable[];
    private readonly roots = new Map<string, Promise<NativeProjectInventory>>();
    private client?: Promise<NativePreview>;
    private timer?: ReturnType<typeof setTimeout>;

    constructor(private readonly context: vscode.ExtensionContext, private readonly output: vscode.OutputChannel,
        private readonly assets: ProjectAssets, readonly assetMoves: AssetMoves) {
        const watcher = vscode.workspace.createFileSystemWatcher('**/{Cargo.toml,*.rs,*.xaml,*.XAML,*.ferrostages,*.ferroui-scenario.json}');
        const invalidate = (uri: vscode.Uri) => {
            const folder = vscode.workspace.getWorkspaceFolder(uri);
            if (!folder || path.relative(folder.uri.fsPath, uri.fsPath).split(path.sep).some(part => ignoredDirectories.has(part))) return;
            this.roots.delete(folder.uri.toString());
            this.schedule();
        };
        this.subscriptions = [watcher, watcher.onDidCreate(invalidate), watcher.onDidDelete(invalidate), watcher.onDidChange(invalidate),
            vscode.workspace.onDidChangeWorkspaceFolders(() => this.refresh()),
            vscode.workspace.onDidChangeConfiguration(change => {
                if (change.affectsConfiguration('ferroui.preview.path')) { this.stopClient(); this.refresh(); }
            }), assets.onDidChange(() => this.schedule())];
    }

    async read(folder: vscode.WorkspaceFolder): Promise<ProjectInventorySnapshot> {
        const [inventory, assets] = await Promise.all([this.native(folder), this.assets.list(folder)]);
        return { truncated: inventory.truncated, files: [
            ...inventory.files,
            ...assets.filter(asset => asset.kind !== 'XAML').map((asset): ProjectFile => ({ path: asset.path, kind: 'asset', assetKind: asset.kind })),
        ] };
    }

    async list(reference: string | undefined, kind: ProjectFile['kind'] | undefined, offset: number, limit: number, signal: AbortSignal) {
        const snapshots = await Promise.all(workspaceFolders(reference).map(async folder => ({ folder, snapshot: await this.read(folder) })));
        signal.throwIfAborted();
        const files = snapshots.flatMap(({ folder, snapshot }) => snapshot.files.filter(file => kind === undefined || file.kind === kind)
            .map(file => ({ ...file, folder: folder.uri.toString() })));
        return { files: files.slice(offset, offset + limit), total: files.length, offset,
            truncated: snapshots.some(({ snapshot }) => snapshot.truncated) };
    }

    async resolve(folder: vscode.WorkspaceFolder, file: ProjectFile): Promise<vscode.Uri> {
        if (file.kind === 'asset') return (await this.assets.resolve(folder, file.path)).uri;
        const uri = vscode.Uri.joinPath(folder.uri, ...file.path.split('/'));
        if (!await isWorkspaceFile(uri)) throw new Error('The project file is unavailable inside this workspace.');
        return uri;
    }

    async designTokens(request: TokenRequest, signal: AbortSignal) {
        signal.throwIfAborted();
        if (Buffer.byteLength(request.source) > 1024 * 1024) throw new Error('The token handoff source exceeds 1 MiB.');
        const client = await this.getClient();
        signal.throwIfAborted();
        const plan = await client.designTokens(request);
        signal.throwIfAborted();
        return plan;
    }

    refresh(): void { this.roots.clear(); this.schedule(); }

    private schedule(): void {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => this.changed.fire(), 180);
    }

    private native(folder: vscode.WorkspaceFolder): Promise<NativeProjectInventory> {
        const key = folder.uri.toString();
        let visit = this.roots.get(key);
        if (visit) return visit;
        visit = this.getClient().then(client => client.projectInventory(folder.uri.fsPath));
        this.roots.set(key, visit);
        void visit.catch(() => { if (this.roots.get(key) === visit) this.roots.delete(key); });
        return visit;
    }

    private async getClient(): Promise<NativePreview> {
        const previous = this.client;
        if (previous && (await previous).closed && this.client === previous) this.client = undefined;
        if (!this.client) {
            const client = NativePreview.start(this.context, this.output);
            this.client = client;
            void client.catch(() => { if (this.client === client) this.client = undefined; });
        }
        return this.client;
    }

    private stopClient(): void {
        void this.client?.then(client => client.dispose(), () => {});
        this.client = undefined;
    }

    dispose(): void {
        this.stopClient();
        clearTimeout(this.timer);
        this.subscriptions.forEach(subscription => subscription.dispose());
        this.changed.dispose();
    }
}
