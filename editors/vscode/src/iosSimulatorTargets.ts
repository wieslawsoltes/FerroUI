import { iosAppProfiles, type IosAppProfile, type IosAppRun } from './iosAppProfiles';
import { simulatorBuildProfile, type SimulatorBuildProfile } from './iosSimulatorBuildProfile';

export interface IosSimulator {
    id: string;
    name: string;
    system: string;
    booted: boolean;
}

export interface IosSimulatorApp extends IosAppProfile {
    build?: SimulatorBuildProfile;
}

export type IosSimulatorRun = IosAppRun;

export function iosSimulatorApps(folder?: string): IosSimulatorApp[] {
    return iosAppProfiles('iosSimulator', folder).map(({ build, ...app }) => {
        if (build !== undefined && app.bundlePath !== undefined) throw new Error(`Choose build or appPath for iOS simulator profile ${app.id}.`);
        return { ...app, build: simulatorBuildProfile(build, app.id) };
    });
}
