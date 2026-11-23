import * as path from 'node:path';
import * as vscode from 'vscode';
import { workspaceFolders } from './workspacePaths';

export interface AndroidDevice {
    id: string;
    name: string;
    status: string;
    emulator: boolean;
}

export interface AndroidApp {
    id: string;
    folder: string;
    packageId: string;
    activity: string;
    label: string;
    apkPath?: string;
}

export interface AndroidRequest {
    device: string;
    profile: string;
    folder?: string;
}

export type AndroidMode = 'run' | 'attach';

export function hasAndroidProfiles(): boolean {
    return workspaceFolders().some(folder => androidSettings(folder).length > 0);
}

export function androidProfiles(folder?: string): AndroidApp[] {
    return workspaceFolders(folder).flatMap(workspace => {
        const profiles = androidSettings(workspace).map(value => androidApp(value, workspace));
        if (new Set(profiles.map(profile => profile.id)).size !== profiles.length) {
            throw new Error(`Give the Android app profiles in ${workspace.name} unique IDs.`);
        }
        return profiles;
    });
}

function androidSettings(folder: vscode.WorkspaceFolder): unknown[] {
    const raw = vscode.workspace.getConfiguration('ferroui.android', folder.uri).get('apps');
    return Array.isArray(raw) ? raw : [];
}

function androidApp(value: unknown, folder: vscode.WorkspaceFolder): AndroidApp {
    const { id, packageId, activity = 'org.ferroui.android.FerroMainActivity', label, apkPath } = (value ?? {}) as Record<string, unknown>;
    if (typeof id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/.test(id)
        || typeof packageId !== 'string' || packageId.length > 255 || !/^[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+$/.test(packageId)
        || typeof activity !== 'string' || activity.length > 255 || !/^\.?[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*$/.test(activity)
        || label !== undefined && (typeof label !== 'string' || !label.trim() || label.length > 96)
        || apkPath !== undefined && (typeof apkPath !== 'string' || path.extname(apkPath).toLowerCase() !== '.apk' || apkPath.includes('\0'))) {
        throw new Error(`Check the Android app profile in ${folder.name}. Use an ID, package name, activity, and optional APK path.`);
    }
    return { id, folder: folder.uri.toString(), packageId, activity, label: typeof label === 'string' ? label.trim() : packageId,
        apkPath: typeof apkPath === 'string' ? path.resolve(folder.uri.fsPath, apkPath) : undefined };
}
