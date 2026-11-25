const simulatorRustTargets = ['aarch64-apple-ios-sim', 'x86_64-apple-ios'] as const;
type SimulatorRustTarget = typeof simulatorRustTargets[number];
export type IosRustTarget = SimulatorRustTarget | 'aarch64-apple-ios';

export interface IosCargoBuildProfile {
    package: string;
    kind: 'bin' | 'example';
    executable: string;
    toolchain: string;
    manifestPath: string;
    features: string[];
    noDefaultFeatures: boolean;
    reloadOnSave: boolean;
}

export interface SimulatorBuildProfile extends IosCargoBuildProfile { target?: SimulatorRustTarget }
export interface IosSigningProfile { identity: string; provisioningProfile: string; entitlements?: string }
export interface DeviceBuildProfile extends IosCargoBuildProfile { signing: IosSigningProfile }

export function simulatorBuildProfile(value: unknown, id: string): SimulatorBuildProfile | undefined {
    if (value === undefined) return;
    const target = (value as { target?: unknown } | null)?.target;
    if (target !== undefined && !simulatorRustTargets.some(item => item === target)) {
        throw new Error(`Choose a simulator Rust target in iOS simulator profile ${id}.`);
    }
    return { ...iosCargoBuildProfile(value, id, 'iOS simulator'), target: target as SimulatorRustTarget | undefined };
}

export function deviceBuildProfile(value: unknown, id: string): DeviceBuildProfile | undefined {
    if (value === undefined) return;
    const build = iosCargoBuildProfile(value, id, 'iOS device');
    const { signing, target } = value as { signing?: unknown; target?: unknown };
    if (target !== undefined && target !== 'aarch64-apple-ios') {
        throw new Error(`Use aarch64-apple-ios for iOS device profile ${id}.`);
    }
    const { identity, provisioningProfile, entitlements } = (signing ?? {}) as Record<string, unknown>;
    if (typeof identity !== 'string' || !/^[0-9a-f]{40}$/i.test(identity) || !buildPath(provisioningProfile)
        || entitlements !== undefined && !buildPath(entitlements)) {
        throw new Error(`Configure the Apple Development certificate fingerprint and provisioning profile for iOS device profile ${id}.`);
    }
    return { ...build, signing: { identity: identity.toUpperCase(), provisioningProfile, entitlements } };
}

function iosCargoBuildProfile(value: unknown, id: string, platform: string): IosCargoBuildProfile {
    const { application, package: packageName, binary, example, toolchain = '.', manifestPath = 'Cargo.toml', features = [],
        noDefaultFeatures = false, reloadOnSave = false } = (value ?? {}) as Record<string, unknown>;
    const invalid = () => new Error(`Check build in ${platform} profile ${id}. Choose Contact Review or a Cargo package and one executable.`);
    if (!buildPath(toolchain) || !buildPath(manifestPath) || !Array.isArray(features)
        || !features.every(feature => typeof feature === 'string' && /^[A-Za-z0-9_][A-Za-z0-9_+./?-]*$/.test(feature))
        || typeof noDefaultFeatures !== 'boolean' || typeof reloadOnSave !== 'boolean') throw invalid();
    const options = { toolchain, manifestPath, reloadOnSave };
    if (application === 'contact-review' && packageName === undefined && binary === undefined && example === undefined) {
        return { ...options, package: 'ferroui-ios', kind: 'example', executable: 'simulator_inspector',
            features: [...new Set(['quartz-metal', 'inspector-example', ...features])], noDefaultFeatures: true };
    }
    if (application !== undefined || !cargoName(packageName)) throw invalid();
    if (cargoName(binary) && example === undefined) {
        return { ...options, package: packageName, kind: 'bin', executable: binary, features, noDefaultFeatures };
    }
    if (cargoName(example) && binary === undefined) {
        return { ...options, package: packageName, kind: 'example', executable: example, features, noDefaultFeatures };
    }
    throw invalid();
}

function cargoName(value: unknown): value is string {
    return typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(value);
}

function buildPath(value: unknown): value is string {
    return typeof value === 'string' && value.trim().length > 0 && !/[\u0000-\u001f]/.test(value);
}
