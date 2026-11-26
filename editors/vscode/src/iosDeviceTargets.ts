import * as vscode from 'vscode';
import { iosAppProfiles, type IosAppProfile, type IosAppRun } from './iosAppProfiles';
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

export function iosDeviceApp(request: IosAppRun): IosDeviceApp {
    const matches = iosDeviceApps(request.folder).filter(app => app.id === request.profile);
    if (matches.length !== 1) throw new Error('Choose an unambiguous iOS device launch profile.');
    return matches[0];
}
