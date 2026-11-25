import * as vscode from 'vscode';
import { iosAppProfiles, type IosAppProfile } from './iosAppProfiles';
import { deviceBuildProfile, type DeviceBuildProfile } from './iosBuildProfiles';
import { workspaceFolders } from './workspacePaths';

export interface IosDeviceApp extends IosAppProfile { build?: DeviceBuildProfile }

export function hasIosDeviceProfiles(): boolean {
    return workspaceFolders().some(folder => {
        const profiles = vscode.workspace.getConfiguration('ferroui.iosDevice', folder.uri).get<unknown>('apps');
        return Array.isArray(profiles) && profiles.length > 0;
    });
}

export function iosDeviceApps(folder?: string): IosDeviceApp[] {
    return iosAppProfiles('iosDevice', folder).map(({ build, ...app }) => {
        if (build !== undefined && app.bundlePath !== undefined) throw new Error(`Choose build or appPath for iOS device profile ${app.id}.`);
        return { ...app, build: deviceBuildProfile(build, app.id) };
    });
}
