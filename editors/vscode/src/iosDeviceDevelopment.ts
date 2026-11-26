import type { IosBundle } from './iosBuild';
import type { IosDevice, IosDeviceBridge } from './iosDeviceBridge';
import { buildIosDeviceApp } from './iosDeviceBuild';
import { launchIosDeviceApp, type IosDeviceProcess } from './iosDeviceProcess';
import type { IosDeviceApp } from './iosDeviceTargets';
import { MobileDevelopment } from './mobileDevelopment';
import type { ReloadDebugger } from './reloadDebugger';
import type { ReloadHistory } from './reloadHistory';
import type { ProcessOutput } from './reloadLog';
import type { RuntimeConnections } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';

interface DeviceDeployment {
    app: IosDeviceApp;
    device: IosDevice;
    bridge: IosDeviceBridge;
    connections: Pick<RuntimeConnections, 'revision' | 'updateIosDevice'>;
    runtime: AppRuntime;
    history: ReloadHistory;
    output: ProcessOutput;
    cancellation: AbortSignal;
    debugger(): Pick<ReloadDebugger, 'activated' | 'detach' | 'detachForReplacement'> | undefined;
}

export function iosDeviceDevelopment(options: DeviceDeployment,
    buildApp = buildIosDeviceApp): MobileDevelopment<IosBundle, IosDeviceProcess> {
    const { app, device, bridge, connections, runtime, history, output, cancellation } = options;
    const processes = (signal: AbortSignal) => bridge.appProcesses(device, app.bundleId, signal);
    const development = new MobileDevelopment<IosBundle, IosDeviceProcess>({
        platform: 'iOS device',
        label: app.label,
        reloadOnSave: app.build?.reloadOnSave ?? false,
        installTimeoutMs: 95_000,
        build: (signal, starting) => buildApp(app, device, output, signal, starting),
        owns: async (process, signal) => !process.console.hasExited && (await processes(signal)).includes(process.pid),
        running: async signal => (await processes(signal)).length > 0,
        beforeInstall: async () => { await options.debugger()?.detachForReplacement(); },
        install: async (bundle, signal) => {
            const previous = development.current;
            if (previous) {
                connections.updateIosDevice(runtime, 'Deploying iOS device app', null);
                await previous.close(true);
            }
            signal.throwIfAborted();
            await bridge.install(device, { ...app, bundlePath: bundle.bundlePath }, signal);
            const process = await launchIosDeviceApp(bridge.executable, device, app, output, signal);
            return { ...process, programPath: bundle.programPath };
        },
        publish: (status, process) => {
            connections.updateIosDevice(runtime, status, process);
            if (!process) return;
            const ended = () => {
                if (development.current !== process || development.deploying || cancellation.aborted) return;
                void options.debugger()?.detach();
                void development.exited(process).catch(error => output.appendLine(String(error)));
            };
            void process.console.closed.then(ended, ended);
        },
        resumeDebugger: process => {
            const generation = connections.revision(runtime);
            if (generation !== undefined) options.debugger()?.activated(process.pid, undefined, generation);
        },
    }, runtime, history, output, cancellation);
    return development;
}
