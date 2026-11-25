import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import type { CargoTarget } from './cargoTarget';
import { buildIosBundle, type IosBundle } from './iosBuild';
import type { IosDevice } from './iosDeviceBridge';
import { prepareDeviceSigning, signDeviceBundle } from './iosDeviceSigning';
import type { IosDeviceApp } from './iosDeviceTargets';
import type { ProcessOutput } from './reloadLog';
import { workspaceFolders } from './workspacePaths';

export async function buildIosDeviceApp(app: IosDeviceApp, device: IosDevice, output: ProcessOutput, signal: AbortSignal,
    starting: (target: CargoTarget) => void): Promise<IosBundle> {
    const build = app.build;
    if (!build) throw new Error('Choose an iOS device source-build profile.');
    const [folder] = workspaceFolders(app.folder);
    const scratch = await mkdtemp(path.join(tmpdir(), 'ferroui-ios-signing-'));
    try {
        const signing = await prepareDeviceSigning(build.signing, folder.uri.fsPath, scratch, app.bundleId, device.udid, signal);
        const bundle = await buildIosBundle(app, build, 'aarch64-apple-ios', output, signal, starting);
        try {
            output.appendLine(`Signing ${app.label} for ${device.name}`);
            await signDeviceBundle(bundle.bundlePath, signing, signal);
            return bundle;
        } catch (error) {
            await bundle.dispose();
            throw error;
        }
    } finally { await rm(scratch, { recursive: true, force: true }); }
}
