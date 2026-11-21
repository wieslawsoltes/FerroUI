import { lstat, realpath, stat } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';

export function isInside(root: string, file: string): boolean {
    const relative = path.relative(root, file);
    return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

export function localFolders(): vscode.WorkspaceFolder[] {
    return vscode.workspace.workspaceFolders?.filter(folder => folder.uri.scheme === 'file') ?? [];
}

export function workspaceFolders(reference?: string): vscode.WorkspaceFolder[] {
    const folders = localFolders();
    if (reference === undefined) return folders;
    const matched = folders.filter(folder => folder.name === reference || folder.uri.toString() === reference);
    if (matched.length !== 1) throw new Error('Choose an unambiguous workspace folder from ferroui_workspace_files.');
    return matched;
}

export async function isWorkspaceFile(uri: vscode.Uri): Promise<boolean> {
    if (uri.scheme !== 'file') return false;
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    if (!folder || folder.uri.scheme !== 'file') return false;
    try {
        const [root, file, info] = await Promise.all([realpath(folder.uri.fsPath), realpath(uri.fsPath), stat(uri.fsPath)]);
        return info.isFile() && isInside(root, file);
    } catch { return false; }
}

/** Map a native application's physical source path to the workspace's editor URI. */
export async function workspaceSourceUri(uri: vscode.Uri): Promise<vscode.Uri | undefined> {
    if (uri.scheme !== 'file') return;
    if (await isWorkspaceFile(uri)) return uri;
    try {
        const [file, info] = await Promise.all([realpath(uri.fsPath), stat(uri.fsPath)]);
        if (!info.isFile()) return;
        const mapped = await Promise.all(localFolders().map(async folder => {
            const root = await realpath(folder.uri.fsPath).catch(() => undefined);
            if (root && isInside(root, file)) return vscode.Uri.file(path.join(folder.uri.fsPath, path.relative(root, file)));
        }));
        const matches = mapped.filter(uri => uri !== undefined);
        if (matches.length === 1) return matches[0];
    } catch { return; }
}

export async function resolveWorkspaceFile(input: string, reference?: string): Promise<vscode.Uri> {
    const folders = workspaceFolders(reference);
    const paths = new Set(folders.flatMap(folder => {
        const file = path.resolve(folder.uri.fsPath, input);
        return isInside(folder.uri.fsPath, file) ? [file] : [];
    }));
    const candidates = [...paths].map(file => vscode.Uri.file(file));
    const matches = (await Promise.all(candidates.map(async uri => await isWorkspaceFile(uri) ? uri : undefined)))
        .filter((uri): uri is vscode.Uri => uri !== undefined);
    if (matches.length !== 1) throw new Error('Provide an unambiguous file path inside the current workspace.');
    return matches[0];
}

export async function newWorkspaceFile(input: string, reference?: string): Promise<vscode.Uri> {
    const folders = workspaceFolders(reference).filter(folder => isInside(folder.uri.fsPath, path.resolve(folder.uri.fsPath, input)));
    if (folders.length !== 1) throw new Error('Choose a destination in one workspace folder.');
    const root = folders[0].uri.fsPath;
    const file = path.resolve(root, input);
    const physicalRoot = await realpath(root);
    if (file === root || await exists(file)) throw new Error('The destination already exists.');
    // Validate the closest existing directory. A nested file can have new parents,
    // including a path below an existing symlink.
    let parent = path.dirname(file);
    while (!await exists(parent)) parent = path.dirname(parent);
    if (!(await stat(parent)).isDirectory() || !isInside(physicalRoot, await realpath(parent))) {
        throw new Error('The destination directory must resolve inside the workspace.');
    }
    return vscode.Uri.file(file);
}

async function exists(file: string): Promise<boolean> {
    try {
        await lstat(file);
        return true;
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
        return false;
    }
}
