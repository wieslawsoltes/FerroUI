export interface AndroidBuildProfile {
    package?: string;
    toolchain: string;
    manifestPath: string;
    features: string[];
    debugger: boolean;
    reloadOnSave: boolean;
    debugKeystore?: string;
}

export function androidBuildProfile(value: unknown, packageId: string, id: string): AndroidBuildProfile | undefined {
    if (value === undefined) return;
    const { application, package: packageName, toolchain = '.', manifestPath = 'Cargo.toml', features = [], debugger: debug = false,
        reloadOnSave = false, debugKeystore } =
        (value ?? {}) as Record<string, unknown>;
    const bundled = application === 'inspector' && packageName === undefined && packageId === 'org.ferroui.androidinspector';
    const custom = application === undefined && typeof packageName === 'string' && /^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(packageName);
    if ((!bundled && !custom) || !buildPath(toolchain) || !buildPath(manifestPath) || typeof debug !== 'boolean' || typeof reloadOnSave !== 'boolean'
        || debugKeystore !== undefined && !buildPath(debugKeystore)
        || !Array.isArray(features) || !features.every(feature => typeof feature === 'string' && /^[A-Za-z0-9_][A-Za-z0-9_+./?-]*$/.test(feature))) {
        throw new Error(`Check build in Android profile ${id}. Choose a Cargo package or the inspector example with its package ID.`);
    }
    return { package: custom ? packageName as string : undefined, toolchain, manifestPath, features, debugger: debug, reloadOnSave, debugKeystore };
}

function buildPath(value: unknown): value is string {
    return typeof value === 'string' && value.trim().length > 0 && !/[\u0000-\u001f]/.test(value);
}
