import { mkdir, mkdtemp, realpath, rm, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { inCargoBuildSlot } from './cargoBuildGate';
import { simulatorCargoTarget, type CargoTarget } from './cargoTarget';
import type { IosSimulatorApp } from './iosSimulatorTargets';
import { runMobilePackager } from './mobilePackager';
import type { ProcessOutput } from './reloadLog';
import { workspaceFolders } from './workspacePaths';

export interface SimulatorBundle {
    bundlePath: string;
    dispose(): Promise<void>;
}

export async function buildSimulatorApp(app: IosSimulatorApp, output: ProcessOutput, signal: AbortSignal,
    starting: (target: CargoTarget) => void): Promise<SimulatorBundle> {
    const build = app.build;
    if (!build) throw new Error('Choose an iOS simulator source-build profile.');
    const [folder] = workspaceFolders(app.folder);
    const toolkit = await realpath(path.resolve(folder.uri.fsPath, build.toolchain));
    const script = path.join(toolkit, 'scripts', 'ios', 'bundle.sh');
    if (!await stat(script).then(info => info.isFile(), () => false)) {
        throw new Error('Choose a FerroUI toolchain with scripts/ios/bundle.sh.');
    }
    const cargo = await simulatorCargoTarget(folder, {
        label: app.label, manifestPath: build.manifestPath, package: build.package, binary: build.executable, features: build.features,
    }, build.kind, signal);
    const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
    const target = build.target ?? (process.arch === 'arm64' ? 'aarch64-apple-ios-sim' : 'x86_64-apple-ios');
    return inCargoBuildSlot(cargo.targetDirectory, signal, async () => {
        starting(cargo);
        await mkdir(cargo.targetDirectory, { recursive: true });
        const directory = await mkdtemp(path.join(cargo.targetDirectory, '.ferroui-simulator-'));
        const dispose = () => rm(directory, { recursive: true, force: true, maxRetries: 3 });
        try {
            const args = [script, '--package', cargo.packageName, build.kind === 'bin' ? '--bin' : '--example', cargo.binary,
                '--target', target, '--profile', 'dev', '--bundle-id', app.bundleId, '--out', directory];
            if (build.features.length) args.push('--features', build.features.join(','));
            if (build.noDefaultFeatures) args.push('--no-default-features');
            await runMobilePackager('iOS simulator', args, workspace,
                { ...process.env, FERROUI_IOS_WORKSPACE: workspace, CARGO_TARGET_DIR: cargo.targetDirectory }, output, signal);
            const bundlePath = path.join(directory, cargo.binary + '.app');
            if (!await stat(bundlePath).then(info => info.isDirectory(), () => false)) throw new Error('The iOS build did not produce its simulator bundle.');
            signal.throwIfAborted();
            return { bundlePath, dispose };
        } catch (error) { await dispose(); throw error; }
    });
}
