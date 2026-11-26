import * as path from 'node:path';
import * as vscode from 'vscode';
import { workspaceFolders } from './workspacePaths';

export interface IosAppProfile {
    id: string;
    bundleId: string;
    label: string;
    folder: string;
    args: string[];
    bundlePath?: string;
}

export interface IosAppRun { device: string; profile: string; folder?: string }

export function iosAppProfiles(kind: 'iosSimulator' | 'iosDevice', folder?: string): (IosAppProfile & { build?: unknown })[] {
    const target = kind === 'iosSimulator' ? 'iOS simulator' : 'iOS device';
    return workspaceFolders(folder).flatMap(workspace => {
        const configured = vscode.workspace.getConfiguration(`ferroui.${kind}`, workspace.uri).get<unknown[]>('apps', []);
        if (!Array.isArray(configured)) return [];
        const apps = configured.map(value => appProfile(value, workspace, target));
        if (new Set(apps.map(app => app.id)).size !== apps.length) {
            throw new Error(`Give the ${target} profiles in ${workspace.name} unique IDs.`);
        }
        return apps;
    });
}

function appProfile(value: unknown, folder: vscode.WorkspaceFolder, target: string): IosAppProfile & { build?: unknown } {
    const { id, bundleId, label, args = [], appPath, build } = (value ?? {}) as Record<string, unknown>;
    if (typeof id !== 'string' || !/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(id)
        || typeof bundleId !== 'string' || bundleId.length > 255 || !/^[a-zA-Z0-9][a-zA-Z0-9-]*(?:\.[a-zA-Z0-9][a-zA-Z0-9-]*)+$/.test(bundleId)
        || label !== undefined && (typeof label !== 'string' || !label.trim() || label.length > 96)
        || !Array.isArray(args) || args.length > 32 || args.some(arg => typeof arg !== 'string' || arg.length > 4096 || arg.includes('\0'))
        || appPath !== undefined && (typeof appPath !== 'string' || path.extname(appPath).toLowerCase() !== '.app' || appPath.includes('\0'))) {
        throw new Error(`Check the ${target} launch profile in ${folder.name}. Use an ID, an iOS bundle ID and an optional .app path.`);
    }
    return { id, bundleId, label: typeof label === 'string' ? label.trim() : bundleId, folder: folder.uri.toString(), args,
        bundlePath: typeof appPath === 'string' ? path.resolve(folder.uri.fsPath, appPath) : undefined, build };
}
