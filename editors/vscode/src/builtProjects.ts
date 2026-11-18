import * as path from 'node:path';
import * as vscode from 'vscode';
import { errorMessage } from './protocol';
import type { ProjectMetadata } from './reloadProcess';
import { isInside } from './workspacePaths';

const stateKey = 'ferroui.builtProjectMetadata.v1';

export interface BuiltProject {
    readonly metadata: ProjectMetadata;
    readonly workspaceRoot: string;
}

export class BuiltProjects implements vscode.Disposable {
    private readonly projects = new Map<string, BuiltProject>();
    private readonly changed = new vscode.EventEmitter<void>();
    readonly onDidChange = this.changed.event;
    private readonly folders: vscode.Disposable;

    constructor(private readonly state: vscode.Memento, private readonly output: vscode.OutputChannel) {
        const saved = state.get<ProjectMetadata[]>(stateKey, []);
        if (Array.isArray(saved)) for (const metadata of saved) this.store(metadata);
        this.folders = vscode.workspace.onDidChangeWorkspaceFolders(() => {
            for (const [manifest] of this.projects) {
                if (!vscode.workspace.getWorkspaceFolder(vscode.Uri.file(manifest))) this.projects.delete(manifest);
            }
            this.changed.fire();
        });
    }

    get metadata(): ProjectMetadata[] {
        return [...this.projects.values()].map(project => project.metadata);
    }

    update(metadata: ProjectMetadata): void {
        if (!this.store(metadata)) return;
        this.changed.fire();
        void Promise.resolve(this.state.update(stateKey, this.metadata)).catch(error => {
            this.output.appendLine(`Project metadata cache: ${errorMessage(error)}`);
        });
    }

    forDocument(uri: vscode.Uri): BuiltProject | undefined {
        if (uri.scheme !== 'file' || !vscode.workspace.isTrusted) return;
        const workspace = vscode.workspace.getWorkspaceFolder(uri)?.uri.fsPath;
        let best: BuiltProject | undefined;
        for (const project of this.projects.values()) {
            const root = path.dirname(project.metadata.manifest);
            if (project.workspaceRoot === workspace && isInside(root, uri.fsPath)
                && (!best || root.length > path.dirname(best.metadata.manifest).length)) best = project;
        }
        return best;
    }

    private store(metadata: ProjectMetadata): boolean {
        if (!vscode.workspace.isTrusted || typeof metadata?.manifest !== 'string' || !path.isAbsolute(metadata.manifest)
            || !Array.isArray(metadata.directories) || metadata.directories.some(directory => typeof directory !== 'string')) return false;
        const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(metadata.manifest));
        if (!folder || folder.uri.scheme !== 'file') return false;
        this.projects.set(metadata.manifest, { metadata, workspaceRoot: folder.uri.fsPath });
        return true;
    }

    dispose(): void {
        this.folders.dispose();
        this.changed.dispose();
    }
}
