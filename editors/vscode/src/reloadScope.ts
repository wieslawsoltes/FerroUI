import { realpathSync } from 'node:fs';
import * as path from 'node:path';
import type { CargoTarget } from './cargoTarget';
import { isInside, localFolders } from './workspacePaths';

export const reloadWatchPattern = '**/{*.[rR][sS],*.[xX][aA][mM][lL],Cargo.toml}';

export function fileKey(file: string): string {
    let real: string;
    try { real = realpathSync.native(file); } catch { real = path.resolve(file); }
    return process.platform === 'win32' ? real.toLowerCase() : real;
}

/** Route source saves by Cargo ownership. A nested package is a separate owner even if the
 * selected package is at the workspace root. Runtime notifications can register exact includes. */
export class ReloadScope {
    readonly targetDirectory: string;
    private readonly workspace: string[];
    private readonly entryPoints: Set<string>;
    private readonly packageRoot: string;
    private readonly declared: string[];
    private readonly targetSources: string[];
    private readonly includes = new Set<string>();
    private roots: string[];
    private packages: Set<string>;

    constructor(target: CargoTarget) {
        this.targetDirectory = fileKey(target.targetDirectory);
        this.workspace = [...new Set([target.folder, ...localFolders()].map(folder => fileKey(folder.uri.fsPath)))];
        this.packageRoot = fileKey(path.dirname(target.manifest));
        this.targetSources = [target.crateRoot, ...(target.libraryRoots ?? [])]
            .filter((file): file is string => Boolean(file)).map(file => path.dirname(fileKey(file)))
            .filter(root => this.inWorkspace(root) && !isInside(this.packageRoot, root));
        this.entryPoints = new Set([target.manifest, target.discoveryManifest, target.sources?.workspaceManifest]
            .filter((file): file is string => Boolean(file)).map(fileKey));
        this.declared = (target.sources?.packageRoots ?? []).map(fileKey);
        const owners = [this.packageRoot, ...(target.sources?.dependencyRoots ?? []).map(fileKey).filter(root => this.inWorkspace(root))];
        this.packages = new Set(owners);
        this.roots = [...new Set([...this.declared, ...owners])].sort((a, b) => b.length - a.length);
    }

    built(manifests: readonly string[]): void {
        const paths = new Set(manifests.map(fileKey));
        const own = fileKey(path.join(this.packageRoot, 'Cargo.toml'));
        if (!paths.has(own)) return;
        const owners = [...paths].map(file => path.dirname(file)).filter(directory => this.inWorkspace(directory));
        this.packages = new Set(owners);
        this.roots = [...new Set([...this.declared, ...owners])].sort((a, b) => b.length - a.length);
    }

    observes(file: string): boolean {
        if (!this.inWorkspace(file)) return false;
        if (this.entryPoints.has(file) || this.includes.has(file)) return true;
        const owner = this.roots.find(root => isInside(root, file));
        return owner !== undefined ? this.packages.has(owner) : this.targetSources.some(root => isInside(root, file));
    }

    observeRuntime(file: string): boolean {
        if (!this.inWorkspace(file)) return false;
        if (!this.observes(file)) this.includes.add(file);
        return true;
    }

    private inWorkspace(file: string): boolean {
        if (isInside(this.targetDirectory, file)) return false;
        return this.workspace.some(root => isInside(root, file)
            && !path.relative(root, file).split(path.sep).some(part => ['.git', '.vscode', 'target', 'node_modules'].includes(part)));
    }
}
