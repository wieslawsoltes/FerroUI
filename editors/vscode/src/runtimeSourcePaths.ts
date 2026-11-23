import * as path from 'node:path';
import * as vscode from 'vscode';
import type { RuntimeSource } from './runtimeProtocol';
import { resolveWorkspaceFile, workspaceSourceUri } from './workspacePaths';

/** Resolve compiler paths on the workspace host when the app runs on a device. */
export async function runtimeSourceUri(source: RuntimeSource): Promise<vscode.Uri | undefined> {
    if (typeof source.uri === 'string') {
        try { return await workspaceSourceUri(vscode.Uri.parse(source.uri, true)); }
        catch { return; }
    }
    if (source.language !== 'rust' || typeof source.revision !== 'string' || !/^[a-f0-9]{16}$/i.test(source.revision)) return;
    const file = compilerPath(source.workspacePath);
    if (!file) return;
    if (path.isAbsolute(file)) return workspaceSourceUri(vscode.Uri.file(file));

    const manifest = compilerPath(source.manifestPath);
    if (manifest && path.isAbsolute(manifest)) {
        let directory = manifest;
        for (;;) {
            const candidate = await workspaceSourceUri(vscode.Uri.file(path.resolve(directory, file)));
            if (candidate) return candidate;
            const parent = path.dirname(directory);
            if (parent === directory) break;
            directory = parent;
        }
    }
    return resolveWorkspaceFile(file).catch(() => undefined);
}

function compilerPath(value: unknown): string | undefined {
    if (typeof value !== 'string' || !value || value.includes('\0') || value.length > 4_096) return;
    return value.replaceAll(/[\\/]/g, path.sep);
}
