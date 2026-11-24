import * as path from 'node:path';
import type { AndroidBridge } from './androidBridge';
import { buildAndroidApp, type AndroidArtifact } from './androidBuild';
import { openAndroidProcess, type AndroidProcess } from './androidProcess';
import type { AndroidApp } from './androidTargets';
import { MobileDevelopment } from './mobileDevelopment';
import type { ReloadHistory } from './reloadHistory';
import type { ProcessOutput } from './reloadLog';
import type { RuntimeConnections } from './runtimeConnections';
import type { AppRuntime } from './runtimeTargets';

export class AndroidDevelopment extends MobileDevelopment<AndroidArtifact, AndroidProcess> {
    constructor(private readonly app: AndroidApp, bridge: AndroidBridge, connections: RuntimeConnections,
        runtime: AppRuntime, history: ReloadHistory, output: ProcessOutput, signal: AbortSignal, beforeInstall: () => Promise<void>) {
        super({
            platform: 'Android',
            label: app.label,
            reloadOnSave: app.build?.reloadOnSave ?? false,
            build: (active, starting) => buildAndroidApp(app, bridge, output, active, starting),
            install: (artifact, active) => openAndroidProcess({ ...app, apkPath: artifact.apkPath }, bridge, 'run', active),
            owns: (process, active) => bridge.matches(app.packageId, process.receipt, active),
            running: async active => Boolean(await bridge.pid(app.packageId, active)),
            beforeInstall,
            publish: (status, process) => connections.updateAndroid(runtime, status, process ? {
                pid: process.pid, debuggable: Boolean(process.receipt.debuggerPath), endpoint: process.forward.endpoint,
            } : process),
        }, runtime, history, output, signal);
    }

    get symbols(): string[] {
        const built = this.artifact?.symbolFiles ?? [];
        const names = new Set(built.map(file => path.basename(file)));
        return [...built, ...this.app.symbolFiles.filter(file => !names.has(path.basename(file)))];
    }
}
