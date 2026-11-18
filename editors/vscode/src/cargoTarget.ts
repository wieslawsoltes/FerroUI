import { execFile } from 'node:child_process';
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
    const targets = (await Promise.all((configured.length ? configured : [{}]).map(target => listTargets(folder, cargo, target)))).flat();
    if (!targets.length) throw new Error('No Cargo binary matches the FerroUI hot reload target.');
    if (targets.length === 1) return targets[0];
    if (!configured.length && document?.uri.scheme === 'file') {
        const local = targets.filter(target => isInside(path.dirname(target.manifest), document.uri.fsPath));
        if (local.length === 1) return local[0];
    }
    const picked = await vscode.window.showQuickPick(targets.map(target => ({
        label: target.label,
        description: target.packageName,
        detail: vscode.workspace.asRelativePath(target.manifest),
        target,
    })), { placeHolder: 'Select a Cargo binary with FerroUI hot reload enabled', matchOnDescription: true, matchOnDetail: true });
    return picked?.target;
}

async function listTargets(folder: vscode.WorkspaceFolder, cargo: string, target: ConfiguredTarget): Promise<CargoTarget[]> {
    const cwd = folder.uri.fsPath;
    const manifest = path.resolve(cwd, target.manifestPath || 'Cargo.toml');
    if (!isInside(cwd, manifest)) throw new Error('The Cargo manifest must be inside the selected workspace folder.');
    const { stdout } = await exec(cargo, ['metadata', '--manifest-path', manifest, '--no-deps', '--format-version', '1'], {
        cwd, windowsHide: true, timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
    });
    const metadata = JSON.parse(stdout) as { packages: CargoPackage[]; workspace_members: string[]; target_directory: string };
    return metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id) && (!target.package || target.package === pkg.name))
        .flatMap(pkg => pkg.targets.filter(bin => bin.kind.includes('bin') && (!target.binary || target.binary === bin.name)).map(bin => ({
            label: target.label ?? bin.name,
            folder, cargo, manifest: pkg.manifest_path, packageId: pkg.id, packageName: pkg.name, binary: bin.name,
            features: target.features ?? (Object.hasOwn(pkg.features, 'hot-reload') ? ['hot-reload'] : []),
            args: target.args ?? [], targetDirectory: metadata.target_directory,
        })));
}

export function isInside(root: string, file: string): boolean {
    const relative = path.relative(root, file);
    return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}
