import { execFile } from 'node:child_process';
import { realpath, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';

const exec = promisify(execFile);

interface ConfiguredTarget {
    label?: string;
    manifestPath?: string;
    package?: string;
    binary?: string;
    features?: string[];
    args?: string[];
}

interface CargoPackage {
    id: string;
    name: string;
    manifest_path: string;
    features: Record<string, string[]>;
    targets: { name: string; kind: string[] }[];
}

export interface CargoTarget {
    label: string;
    folder: vscode.WorkspaceFolder;
    cargo: string;
    cargoDirectory: string;
    manifest: string;
    packageId: string;
    packageName: string;
    binary: string;
    features: string[];
    args: string[];
    targetDirectory: string;
}

export async function selectCargoTarget(): Promise<CargoTarget | undefined> {
    if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before running a FerroUI application.');
    const folders = vscode.workspace.workspaceFolders?.filter(folder => folder.uri.scheme === 'file') ?? [];
    const document = vscode.window.activeTextEditor?.document;
    const owner = document && vscode.workspace.getWorkspaceFolder(document.uri);
    const folder = owner?.uri.scheme === 'file' ? owner : folders.length === 1 ? folders[0]
        : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Select a FerroUI Cargo workspace' });
    if (!folder || folder.uri.scheme !== 'file') return undefined;
    const settings = vscode.workspace.getConfiguration('ferroui.hotReload', folder.uri);
    const cargo = settings.get<string>('cargoPath') || 'cargo';
    const configured = settings.get<ConfiguredTarget[]>('targets') ?? [];
    const automatic = configured.length ? undefined : await nearestManifest(folder,
        document?.uri.scheme === 'file' ? document.uri.fsPath : undefined);
    let targets = (await Promise.all((configured.length ? configured : [{ manifestPath: automatic }])
        .map(target => listTargets(folder, cargo, target)))).flat();
    if (!targets.length) throw new Error('No Cargo binary matches the FerroUI hot reload target.');
    if (targets.length > 1 && !configured.length && document?.uri.scheme === 'file') {
        const file = await realpath(document.uri.fsPath).catch(() => document.uri.fsPath);
        const local = targets.filter(target => isInside(path.dirname(target.manifest), file));
        const nearest = local.sort((a, b) => b.manifest.length - a.manifest.length)[0];
        if (nearest) targets = local.filter(target => target.manifest === nearest.manifest);
    }
    if (targets.length === 1) return targets[0];
    const picked = await vscode.window.showQuickPick(targets.map(target => ({
        label: target.label,
        description: target.packageName,
        detail: vscode.workspace.asRelativePath(target.manifest),
        target,
    })), { placeHolder: 'Select a Cargo binary with FerroUI hot reload enabled', matchOnDescription: true, matchOnDetail: true });
    return picked?.target;
}

async function listTargets(folder: vscode.WorkspaceFolder, cargo: string, target: ConfiguredTarget): Promise<CargoTarget[]> {
    const root = folder.uri.fsPath;
    const manifest = path.resolve(root, target.manifestPath || 'Cargo.toml');
    if (!isInside(root, manifest)) throw new Error('The Cargo manifest must be inside the selected workspace folder.');
    const [physicalRoot, physicalManifest] = await Promise.all([realpath(root), realpath(manifest).catch(error => {
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
            throw new Error(`No Cargo manifest found at ${path.relative(root, manifest)}. Open a file in a Cargo project or set ferroui.hotReload.targets.`);
        }
        throw error;
    })]);
    if (!isInside(physicalRoot, physicalManifest)) throw new Error('The Cargo manifest must resolve inside this workspace.');
    const cwd = path.dirname(manifest);
    const { stdout } = await exec(cargo, ['metadata', '--manifest-path', manifest, '--no-deps', '--format-version', '1'], {
        cwd, windowsHide: true, timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
    });
    const metadata = JSON.parse(stdout) as { packages: CargoPackage[]; workspace_members: string[]; target_directory: string };
    return metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id)
        && (isInside(root, pkg.manifest_path) || isInside(physicalRoot, pkg.manifest_path))
        && (!target.package || target.package === pkg.name))
        .flatMap(pkg => pkg.targets.filter(bin => bin.kind.includes('bin') && (!target.binary || target.binary === bin.name)).map(bin => ({
            label: target.label ?? bin.name,
            folder, cargo, cargoDirectory: cwd, manifest: pkg.manifest_path, packageId: pkg.id, packageName: pkg.name, binary: bin.name,
            features: target.features ?? (Object.hasOwn(pkg.features, 'hot-reload') ? ['hot-reload'] : []),
            args: target.args ?? [], targetDirectory: metadata.target_directory,
        })));
}

async function nearestManifest(folder: vscode.WorkspaceFolder, file?: string): Promise<string | undefined> {
    const root = folder.uri.fsPath;
    let directory = file && isInside(root, file) ? path.dirname(file) : root;
    while (isInside(root, directory)) {
        const manifest = path.join(directory, 'Cargo.toml');
        if (await stat(manifest).then(info => info.isFile(), () => false)) return path.relative(root, manifest);
        const parent = path.dirname(directory);
        if (directory === root || directory === parent) break;
        directory = parent;
    }
    return undefined;
}

export function isInside(root: string, file: string): boolean {
    const relative = path.relative(root, file);
    return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}
