import { execFile } from 'node:child_process';
import { realpath, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { isInside, localFolders, workspaceFolders } from './workspacePaths';

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
    rust_version?: string | null;
    features: Record<string, string[]>;
    dependencies?: { path?: string | null; kind?: string | null }[];
    targets: { name: string; kind: string[]; src_path: string }[];
}

interface CargoMetadata {
    packages: CargoPackage[];
    workspace_members: string[];
    target_directory: string;
    workspace_root?: string;
}

export interface CargoSources {
    workspaceManifest?: string;
    packageRoots: string[];
    dependencyRoots: string[];
}

export interface RustModuleProject {
    manifest: string;
    targets: string[];
    features?: string[];
}

export interface CargoTarget {
    label: string;
    folder: vscode.WorkspaceFolder;
    cargo: string;
    cargoDirectory: string;
    discoveryManifest: string;
    manifest: string;
    packageId: string;
    packageName: string;
    binary: string;
    rustVersion?: string;
    crateRoot?: string;
    libraryRoots?: string[];
    features: string[];
    args: string[];
    targetDirectory: string;
    sources?: CargoSources;
}

export interface CargoSelection {
    folder?: string;
    manifest_path?: string;
    package?: string;
    binary?: string;
    label?: string;
    features?: string[];
}

export async function selectCargoTarget(placeHolder = 'Select a Cargo binary with FerroUI hot reload enabled'): Promise<CargoTarget | undefined> {
    if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before running a FerroUI application.');
    const folders = localFolders();
    const document = vscode.window.activeTextEditor?.document;
    const owner = document && vscode.workspace.getWorkspaceFolder(document.uri);
    const folder = owner?.uri.scheme === 'file' ? owner : folders.length === 1 ? folders[0]
        : await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Select a FerroUI Cargo workspace' });
    if (!folder || folder.uri.scheme !== 'file') return undefined;
    let { targets, configured } = await targetsIn(folder);
    if (!targets.length) throw new Error('No Cargo binary matches the FerroUI hot reload target.');
    if (targets.length > 1 && !configured && document?.uri.scheme === 'file') {
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
    })), { placeHolder, matchOnDescription: true, matchOnDetail: true });
    return picked?.target;
}

export async function discoverCargoTargets(selection: CargoSelection): Promise<CargoTarget[]> {
    if (!vscode.workspace.isTrusted) throw new Error('Trust the workspace before discovering Cargo targets.');
    const folders = workspaceFolders(selection.folder);
    const document = vscode.window.activeTextEditor?.document;
    const absolute = selection.manifest_path && path.isAbsolute(selection.manifest_path);
    const source = absolute ? vscode.Uri.file(selection.manifest_path!) : document?.uri;
    const owner = source && vscode.workspace.getWorkspaceFolder(source);
    const folder = folders.length === 1 ? folders[0] : folders.find(item => item.uri.toString() === owner?.uri.toString());
    if (!folder) throw new Error('Specify a workspace folder from ferroui_workspace_files.');
    const { targets } = await targetsIn(folder, selection.manifest_path);
    return targets.filter(target => (selection.package === undefined || target.packageName === selection.package)
        && (selection.binary === undefined || target.binary === selection.binary)
        && (selection.label === undefined || target.label === selection.label)
        && (selection.features === undefined || sameFeatures(target.features, selection.features)));
}

function sameFeatures(current: string[], requested: string[]): boolean {
    return current.length === requested.length && requested.every(feature => current.includes(feature))
        && current.every(feature => requested.includes(feature));
}

export function describeCargoTarget(target: CargoTarget) {
    return { folder: target.folder.uri.toString(), manifest_path: path.relative(target.folder.uri.fsPath, target.discoveryManifest),
        package: target.packageName, binary: target.binary, label: target.label };
}

async function targetsIn(folder: vscode.WorkspaceFolder, manifest?: string) {
    const settings = vscode.workspace.getConfiguration('ferroui.hotReload', folder.uri);
    const cargo = settings.get<string>('cargoPath') || 'cargo';
    const root = folder.uri.fsPath;
    const configured = (settings.get<ConfiguredTarget[]>('targets') ?? []).filter(target => manifest === undefined
        || path.resolve(root, target.manifestPath || 'Cargo.toml') === path.resolve(root, manifest));
    const document = vscode.window.activeTextEditor?.document;
    const activeFile = document?.uri.scheme === 'file' ? document.uri.fsPath : undefined;
    const use = configured.length ? configured : [{ manifestPath: manifest ?? await nearestManifest(folder, activeFile) }];
    return { configured: configured.length !== 0, targets: (await Promise.all(use.map(target => listTargets(folder, cargo, target)))).flat() };
}

async function listTargets(folder: vscode.WorkspaceFolder, cargo: string, target: ConfiguredTarget): Promise<CargoTarget[]> {
    const root = folder.uri.fsPath;
    const manifest = path.resolve(root, target.manifestPath || 'Cargo.toml');
    const { metadata, physicalRoot } = await readMetadata(folder, cargo, manifest);
    const cwd = path.dirname(manifest);
    return metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id)
        && (isInside(root, pkg.manifest_path) || isInside(physicalRoot, pkg.manifest_path))
        && (!target.package || target.package === pkg.name))
        .flatMap(pkg => pkg.targets.filter(bin => bin.kind.includes('bin') && (!target.binary || target.binary === bin.name)).map(bin => ({
            label: target.label ?? bin.name,
            folder, cargo, cargoDirectory: cwd, discoveryManifest: manifest, manifest: pkg.manifest_path,
            packageId: pkg.id, packageName: pkg.name, binary: bin.name, crateRoot: bin.src_path,
            rustVersion: pkg.rust_version ?? undefined,
            libraryRoots: pkg.targets.filter(target => target.kind.some(kind => libraryKinds.has(kind))).map(target => target.src_path),
            features: target.features ?? (Object.hasOwn(pkg.features, 'hot-reload') ? ['hot-reload'] : []),
            args: target.args ?? [], targetDirectory: metadata.target_directory,
            sources: cargoSources(metadata, pkg),
        })));
}

function cargoSources(metadata: CargoMetadata, selected: CargoPackage): CargoSources {
    const packages = new Map(metadata.packages.map(pkg => [path.dirname(pkg.manifest_path), pkg]));
    const dependencyRoots = new Set<string>();
    const visit = (pkg: CargoPackage) => {
        for (const dependency of pkg.dependencies ?? []) {
            if (!dependency.path || dependency.kind === 'dev' || dependencyRoots.has(dependency.path)) continue;
            dependencyRoots.add(dependency.path);
            const local = packages.get(dependency.path);
            if (local) visit(local);
        }
    };
    visit(selected);
    return { workspaceManifest: metadata.workspace_root && path.join(metadata.workspace_root, 'Cargo.toml'),
        packageRoots: [...packages.keys()], dependencyRoots: [...dependencyRoots] };
}

const libraryKinds = new Set(['lib', 'rlib', 'dylib', 'cdylib', 'staticlib', 'proc-macro']);

export async function rustModuleProject(uri: vscode.Uri): Promise<RustModuleProject | undefined> {
    if (!vscode.workspace.isTrusted || uri.scheme !== 'file') return;
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (folder?.uri.scheme !== 'file') return;
    const relative = await nearestManifest(folder, uri.fsPath);
    if (relative === undefined) return;
    const manifest = path.resolve(folder.uri.fsPath, relative);
    const cargo = vscode.workspace.getConfiguration('ferroui.hotReload', folder.uri).get<string>('cargoPath') || 'cargo';
    const { metadata, physicalManifest } = await readMetadata(folder, cargo, manifest, true);
    const owner = metadata.packages.find(pkg => metadata.workspace_members.includes(pkg.id)
        && (pkg.manifest_path === physicalManifest || pkg.manifest_path === manifest));
    if (owner) return { manifest: owner.manifest_path, targets: [...new Set(owner.targets.map(target => target.src_path))] };
}

async function readMetadata(folder: vscode.WorkspaceFolder, cargo: string, manifest: string, offline = false) {
    const root = folder.uri.fsPath;
    if (!isInside(root, manifest)) throw new Error('The Cargo manifest must be inside the selected workspace folder.');
    const [physicalRoot, physicalManifest] = await Promise.all([realpath(root), realpath(manifest).catch(error => {
        if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
            throw new Error(`No Cargo manifest found at ${path.relative(root, manifest)}. Open a file in a Cargo project or set ferroui.hotReload.targets.`);
        }
        throw error;
    })]);
    if (!isInside(physicalRoot, physicalManifest)) throw new Error('The Cargo manifest must resolve inside this workspace.');
    const options = offline ? ['--locked', '--offline'] : [];
    const { stdout } = await exec(cargo, ['metadata', '--manifest-path', manifest, '--no-deps', '--format-version', '1', ...options], {
        cwd: path.dirname(manifest), windowsHide: true, timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
    });
    return { metadata: JSON.parse(stdout) as CargoMetadata, physicalRoot, physicalManifest };
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
