import { strict as assert } from 'node:assert';
import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import * as vscode from 'vscode';
import type { IosAppProfile } from '../src/iosAppProfiles';
import { deviceBuildProfile } from '../src/iosBuildProfiles';
import { IosDeviceBridge, type IosDevice } from '../src/iosDeviceBridge';
import { iosDeviceDevelopment } from '../src/iosDeviceDevelopment';
import type { IosDeviceProcess } from '../src/iosDeviceProcess';
import { ReloadHistory } from '../src/reloadHistory';
import { createIosDeviceFixture, deviceProfile } from './iosDeviceFixture';
import { until } from './wait';

class DeviceFixtureBridge extends IosDeviceBridge {
    constructor(executable: string, private readonly installed: string) { super(executable); }

    override async install(device: IosDevice, app: IosAppProfile, signal: AbortSignal) {
        assert.deepEqual(await this.appProcesses(device, app.bundleId, signal), [], 'The candidate never installs over an independent process');
        const revision = await readFile(join(app.bundlePath!, 'revision'), 'utf8');
        if (revision === 'install-failed') throw new Error('Fixture installation failed');
        await writeFile(this.installed, revision);
    }
}

export async function verifyIosDeviceDevelopment(): Promise<void> {
    if (process.platform === 'win32') return;
    const root = await mkdtemp(join(tmpdir(), 'ferroui-ios-device-deployment-'));
    const fixture = await createIosDeviceFixture(root);
    const bridge = new DeviceFixtureBridge(fixture.executable, fixture.installed);
    const controller = new AbortController();
    const history = new ReloadHistory();
    const [device] = await bridge.devices(controller.signal);
    const folder = { uri: vscode.Uri.file(root), name: 'Contact Review', index: 0 };
    const manifest = join(root, 'Cargo.toml');
    const app = { ...deviceProfile, folder: folder.uri.toString(), build: deviceBuildProfile({
        application: 'contact-review', reloadOnSave: true, signing: { identity: 'a'.repeat(40), provisioningProfile: 'fixtures.mobileprovision' },
    }, deviceProfile.id) };
    const runtime = 'ios-device:deployment-fixture';
    let status = '';
    let accepted: Pick<IosDeviceProcess, 'pid' | 'endpoint'> | undefined;
    let generation = 0;
    let revision = 'first';
    let builds = 0;
    let waitToBuild: Promise<void> | undefined;
    let releaseBuild = () => {};
    let detaches = 0;
    const debuggerGenerations: number[] = [];
    const output = { append() {}, appendLine() {} };
    const artifacts = async () => (await readdir(root)).filter(name => name.startsWith('bundle-')).sort();
    const installed = () => readFile(fixture.installed, 'utf8');
    const development = iosDeviceDevelopment({
        app, device, bridge, runtime, output, history, cancellation: controller.signal,
        connections: {
            revision: () => accepted ? generation : undefined,
            updateIosDevice: (_, message, process) => {
                status = message;
                if (process !== undefined) accepted = process ?? undefined;
                if (process) generation++;
            },
        },
        debugger: () => ({
            async detach() {},
            async detachForReplacement() { detaches++; },
            activated(pid, _, active) {
                assert.equal(pid, 9421);
                assert.equal(active, generation);
                debuggerGenerations.push(generation);
            },
        }),
    }, async (_, __, ___, signal, starting) => {
        const captured = revision;
        builds++;
        starting({
            folder, manifest, discoveryManifest: manifest, label: app.label, cargo: 'cargo', cargoDirectory: root,
            packageId: 'contact-review', packageName: 'contact-review', binary: 'contact-review', features: [], args: [], targetDirectory: join(root, 'target'),
        });
        await waitToBuild;
        signal.throwIfAborted();
        if (captured === 'signing-failed') throw new Error('Fixture could not sign the candidate');
        const directory = await mkdtemp(join(root, 'bundle-'));
        const bundlePath = join(directory, 'ContactReview.app');
        const programPath = join(bundlePath, 'ContactReview');
        await mkdir(bundlePath);
        await writeFile(join(bundlePath, 'revision'), captured);
        return { bundlePath, programPath, dispose: () => rm(directory, { recursive: true, force: true }) };
    });

    try {
        development.rebuild();
        await until(() => generation === 1);
        assert.equal(await installed(), 'first');
        const first = development.current!;
        revision = 'signing-failed';
        development.observe(join(root, 'View.xaml'), () => '<Button />');
        await until(() => status === 'iOS device build failed');
        assert.equal(development.current, first);
        assert.equal(generation, 1);
        assert.equal(detaches, 1);

        waitToBuild = new Promise<void>(resolve => { releaseBuild = resolve; });
        revision = 'superseded';
        development.observe(join(root, 'src', 'main.rs'), () => 'fn stale() {}');
        await until(() => builds === 3);
        assert.equal(accepted, first);
        revision = 'latest';
        development.observe(join(root, 'src', 'main.rs'), () => 'fn latest() {}');
        waitToBuild = undefined;
        releaseBuild();
        await until(() => generation === 2);
        assert.equal(await installed(), 'latest');
        assert.equal(development.current?.pid, first.pid);
        assert.notEqual(development.current, first);
        const matchedProgram = development.current?.programPath;
        await until(async () => (await artifacts()).length === 1);

        revision = 'bad-frame';
        development.observe(join(root, 'View.xaml'), () => '<Border />');
        await until(() => status.includes('restored previous build'));
        assert.equal(await installed(), 'latest');
        assert.equal(generation, 3);
        assert.deepEqual(debuggerGenerations, [1, 2, 3]);
        assert.equal(development.current?.programPath, matchedProgram);
        assert.deepEqual(history.recent(10, runtime).map(item => item.outcome),
            ['buildFailed', 'rebuilt', 'superseded', 'buildFailed']);

        revision = 'install-failed';
        development.rebuild();
        await until(() => generation === 4 && status.includes('restored previous build'));
        assert.equal(await installed(), 'latest');
        assert.equal(development.current?.programPath, matchedProgram);
        assert.deepEqual(debuggerGenerations, [1, 2, 3, 4]);

        await writeFile(fixture.running, '5043');
        development.rebuild();
        await until(() => status === 'iOS device build failed');
        assert.equal(accepted, undefined);
        assert.equal(await readFile(fixture.running, 'utf8'), '5043');
        assert.equal(await installed(), 'latest');
        revision = 'would-overwrite';
        development.rebuild();
        await until(() => builds === 8 && status === 'iOS device build failed');
        assert.equal(await installed(), 'latest');
        assert.equal(await readFile(fixture.running, 'utf8'), '5043');
    } finally {
        releaseBuild();
        controller.abort();
        try {
            await development.close(true);
            assert.deepEqual(await artifacts(), []);
        } finally {
            history.dispose();
            await rm(root, { recursive: true, force: true });
        }
    }
}
