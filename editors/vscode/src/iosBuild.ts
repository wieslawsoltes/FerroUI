import { mkdir, mkdtemp, realpath, rm, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { inCargoBuildSlot } from './cargoBuildGate';
import { iosCargoTarget, type CargoTarget } from './cargoTarget';
import type { IosAppProfile } from './iosAppProfiles';
import type { IosCargoBuildProfile, IosRustTarget } from './iosBuildProfiles';
import { runMobilePackager } from './mobilePackager';
import type { ProcessOutput } from './reloadLog';
import { workspaceFolders } from './workspacePaths';

export interface IosBundle {
    bundlePath: string;
    programPath: string;
    dispose(): Promise<void>;
}

export async function buildIosBundle(app: IosAppProfile, build: IosCargoBuildProfile, target: IosRustTarget, output: ProcessOutput,
    signal: AbortSignal, starting: (target: CargoTarget) => void): Promise<IosBundle> {
    const platform = target === 'aarch64-apple-ios' ? 'iOS device' : 'iOS simulator';
    const [folder] = workspaceFolders(app.folder);
    const toolkit = await realpath(path.resolve(folder.uri.fsPath, build.toolchain));
    const script = path.join(toolkit, 'scripts', 'ios', 'bundle.sh');
    if (!await stat(script).then(info => info.isFile(), () => false)) {
        throw new Error('Choose a FerroUI toolchain with scripts/ios/bundle.sh.');
    }
    const cargo = await iosCargoTarget(folder, {
        label: app.label, manifestPath: build.manifestPath, package: build.package, binary: build.executable, features: build.features,
    }, build.kind, signal);
    const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
    return inCargoBuildSlot(cargo.targetDirectory, signal, async () => {
        starting(cargo);
        await mkdir(cargo.targetDirectory, { recursive: true });
        const prefix = target === 'aarch64-apple-ios' ? '.ferroui-ios-device-' : '.ferroui-simulator-';
        const directory = await mkdtemp(path.join(cargo.targetDirectory, prefix));
        const dispose = () => rm(directory, { recursive: true, force: true, maxRetries: 3 });
        try {
            const args = [script, '--package', cargo.packageName, build.kind === 'bin' ? '--bin' : '--example', cargo.binary,
                '--target', target, '--profile', 'dev', '--bundle-id', app.bundleId, '--out', directory];
            if (build.features.length) args.push('--features', build.features.join(','));
            if (build.noDefaultFeatures) args.push('--no-default-features');
            await runMobilePackager(platform, args, workspace,
                { ...process.env, FERROUI_IOS_WORKSPACE: workspace, CARGO_TARGET_DIR: cargo.targetDirectory }, output, signal);
            const bundlePath = path.join(directory, cargo.binary + '.app');
            const programPath = path.join(bundlePath, cargo.binary);
            if (!await stat(programPath).then(info => info.isFile(), () => false)) throw new Error(`The ${platform} build did not produce its app bundle.`);
            signal.throwIfAborted();
            return { bundlePath, programPath, dispose };
        } catch (error) { await dispose(); throw error; }
    });
}
