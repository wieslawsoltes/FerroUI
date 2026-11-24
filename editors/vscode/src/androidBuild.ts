import { spawn } from 'node:child_process';
import { constants } from 'node:fs';
import { copyFile, mkdir, mkdtemp, realpath, rename, rm, stat } from 'node:fs/promises';
import * as path from 'node:path';
import { createInterface } from 'node:readline';
import type { AndroidBridge } from './androidBridge';
import type { AndroidApp } from './androidTargets';
import { inCargoBuildSlot } from './cargoBuildGate';
import { androidCargoTarget, type CargoTarget } from './cargoTarget';
import { OwnedProcess } from './ownedProcess';
import type { ProcessOutput } from './reloadLog';
import { fileKey, workspaceFolders } from './workspacePaths';

export interface AndroidArtifact {
    apkPath: string;
    symbolFiles: string[];
    dispose(): Promise<void>;
}

const targets = { 'arm64-v8a': 'aarch64-linux-android', 'x86_64': 'x86_64-linux-android' };
type AndroidAbi = keyof typeof targets;

export async function buildAndroidApp(app: AndroidApp, bridge: AndroidBridge, output: ProcessOutput,
    signal: AbortSignal, starting: (target: CargoTarget) => void): Promise<AndroidArtifact> {
    const build = app.build;
    if (!build) throw new Error('Choose an Android source-build profile.');
    const [folder] = workspaceFolders(app.folder);
    const toolkit = await realpath(path.resolve(folder.uri.fsPath, build.toolchain));
    const script = path.join(toolkit, 'scripts', 'android', 'apk.sh');
    if (!await stat(script).then(info => info.isFile(), () => false)) {
        throw new Error('Choose a FerroUI toolchain with scripts/android/apk.sh.');
    }
    const packageName = build.package ?? 'ferroui-android';
    const library = build.package?.replaceAll('-', '_') ?? 'android_inspector';
    const [cargo, variants] = await Promise.all([
        androidCargoTarget(folder, {
            label: app.label, manifestPath: build.manifestPath, package: packageName, binary: library,
            features: build.package ? build.features : ['inspector-example', ...build.features],
        }, build.package ? 'cdylib' : 'example', signal),
        bridge.command(['shell', 'getprop', 'ro.product.cpu.abilist'], signal),
    ]);
    const abi = variants.split(',').map(entry => entry.trim()).find((entry): entry is AndroidAbi => Object.hasOwn(targets, entry));
    if (!abi) throw new Error('Select an Android device supporting arm64-v8a or x86_64.');

    const workspace = path.dirname(cargo.sources?.workspaceManifest ?? cargo.discoveryManifest);
    if (!build.package && fileKey(workspace) !== fileKey(toolkit)) {
        throw new Error('Open the FerroUI Cargo workspace to build the bundled Android inspector.');
    }
    const configuredKey = build.debugKeystore ?? process.env.FERROUI_ANDROID_DEBUG_KEYSTORE;
    const keystore = configuredKey ? path.resolve(folder.uri.fsPath, configuredKey)
        : path.join(cargo.targetDirectory, 'android', 'debug.keystore');
    return inCargoBuildSlot(cargo.targetDirectory, signal, async () => {
        starting(cargo);
        await mkdir(cargo.targetDirectory, { recursive: true });
        const directory = await mkdtemp(path.join(cargo.targetDirectory, '.ferroui-android-'));
        const dispose = () => rm(directory, { recursive: true, force: true, maxRetries: 3 });
        try {
            const staging = path.join(directory, 'package');
            const args = [script, build.package ? 'package' : 'inspector', '--abi', abi, '--out', staging];
            if (build.package) args.push('--package', build.package, '--application-id', app.packageId,
                '--permission', 'android.permission.INTERNET');
            if (build.features.length) args.push('--features', build.features.join(','));
            if (build.debugger) args.push('--debugger');
            await packageApp(args, workspace, {
                ...process.env, FERROUI_ANDROID_WORKSPACE: workspace, CARGO_TARGET_DIR: cargo.targetDirectory,
                FERROUI_ANDROID_DEBUG_KEYSTORE: keystore,
            }, output, signal);
            const apkPath = path.join(directory, 'app.apk');
            await rename(path.join(staging, `${build.package ?? 'inspector'}-${abi}-debug.apk`), apkPath);
            const symbolFiles: string[] = [];
            if (build.debugger) {
                const name = `lib${library}.so`;
                const source = path.join(cargo.targetDirectory, targets[abi], 'debug', build.package ? '' : 'examples', name);
                const destination = path.join(directory, name);
                await copyFile(source, destination, constants.COPYFILE_FICLONE);
                symbolFiles.push(destination);
            }
            await rm(staging, { recursive: true, force: true, maxRetries: 3 });
            signal.throwIfAborted();
            return { apkPath, symbolFiles, dispose };
        } catch (error) { await dispose(); throw error; }
    });
}

async function packageApp(args: string[], workspace: string, env: NodeJS.ProcessEnv, output: ProcessOutput, signal: AbortSignal): Promise<void> {
    signal.throwIfAborted();
    const child = spawn('bash', args, {
        cwd: workspace, detached: true, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'], env,
    });
    const compiler = new OwnedProcess(child);
    const stop = () => { void compiler.stop(); };
    signal.addEventListener('abort', stop, { once: true });
    for (const stream of [child.stdout, child.stderr]) {
        createInterface({ input: stream }).on('line', line => output.appendLine(line));
    }
    let exit: number | null;
    try { exit = await compiler.closed; }
    finally { signal.removeEventListener('abort', stop); }
    signal.throwIfAborted();
    if (exit !== 0) throw new Error(`The Android build failed${exit === null ? '' : ` with exit code ${exit}`}. See Show App Output.`);
}
