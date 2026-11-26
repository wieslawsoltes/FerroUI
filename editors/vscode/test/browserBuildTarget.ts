import { strict as assert } from 'node:assert';
import * as path from 'node:path';
import * as vscode from 'vscode';
import type { AppSessions } from '../src/appSessions';
import { resolveBrowserBuild } from '../src/browserBuild';
import type { BrowserBuildProfile } from '../src/browserTargets';
import type { AppRuntime } from '../src/runtimeTargets';
import { fileKey } from '../src/workspacePaths';
import { until } from './wait';

const appManifest = '[package]\nname = "live_inspector"\nversion = "0.1.0"\nedition = "2021"\n';

export async function verifyBrowserBuildTarget(workspace: vscode.Uri): Promise<void> {
    const fixture = vscode.Uri.joinPath(workspace, 'Browser Cargo targets');
    const browserCrate = 'src/Browser/FerroUI.Browser';
    const toolkit = `toolkit/${browserCrate}`;
    const example = `${toolkit}/examples/live_inspector`;
    const files = [
        ['toolkit/Cargo.toml', `[workspace]\nresolver = "2"\nmembers = ["${browserCrate}"]\n`],
        ['toolkit/scripts/build-browser.sh', '#!/bin/bash\necho "fixture reached the browser build"\nexit 23\n'],
        [`${toolkit}/Cargo.toml`, '[package]\nname = "ferroui-browser"\nversion = "0.1.0"\nedition = "2021"\n'],
        [`${toolkit}/src/lib.rs`, ''],
        [`${example}/main.rs`, 'fn main() {}\n'],
        ['app/Cargo.toml', appManifest],
        ['app/src/main.rs', 'fn main() {}\n'],
    ];
    for (const [name, content] of files) {
        const uri = vscode.Uri.joinPath(fixture, name);
        await vscode.workspace.fs.createDirectory(vscode.Uri.file(path.dirname(uri.fsPath)));
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
    }
    const build = (manifest: string): BrowserBuildProfile => ({
        application: 'live_inspector', toolchain: vscode.Uri.joinPath(fixture, 'toolkit').fsPath,
        manifestPath: vscode.Uri.joinPath(fixture, manifest).fsPath, features: [], threads: false, debugSymbols: false,
    });
    const resolve = (manifest: string) => resolveBrowserBuild({
        id: 'live-inspector', folder: workspace.toString(), label: 'Live Inspector', width: 960, height: 620,
        build: build(manifest),
    }, AbortSignal.timeout(20_000));
    const [app, bundled] = await Promise.all([resolve('app/Cargo.toml'), resolve('toolkit/Cargo.toml')]);
    assert.equal(app.cargo.kind, 'bin');
    assert.equal(app.cargo.packageName, 'live_inspector');
    assert.equal(app.hostPage, fileKey(vscode.Uri.joinPath(fixture, 'app/wwwroot').fsPath));
    assert.equal(bundled.cargo.kind, 'example');
    assert.equal(bundled.cargo.packageName, 'ferroui-browser');
    assert.equal(bundled.hostPage, fileKey(vscode.Uri.joinPath(fixture, example, 'wwwroot').fsPath));
    console.log('VS Code resolved colliding browser targets through their owning Cargo workspace');
    await verifyInitialManifestRepair(workspace, build('app/Cargo.toml'));
}

async function verifyInitialManifestRepair(workspace: vscode.Uri, build: BrowserBuildProfile): Promise<void> {
    const manifest = vscode.Uri.file(build.manifestPath);
    await vscode.workspace.fs.writeFile(manifest, Buffer.from('[package\n'));
    const { appSessions } = vscode.extensions.getExtension<{ appSessions: AppSessions }>('FerroUI.ferroui')!.exports;
    const settings = vscode.workspace.getConfiguration('ferroui.browser', workspace);
    const previous = settings.inspect('apps')?.workspaceValue;
    const profile = 'repair-browser-manifest';
    await settings.update('apps', [{ id: profile, build }], vscode.ConfigurationTarget.Workspace);
    let runtime: AppRuntime | undefined;
    try {
        const pending = await appSessions.browser.open({ profile, folder: workspace.toString() }, 'run', AbortSignal.timeout(5000));
        runtime = pending.runtime;
        await until(() => appSessions.list().runtimes.some(session => session.runtime === runtime && session.status === 'Browser launch failed'));
        const editor = await vscode.window.showTextDocument(manifest);
        const entire = new vscode.Range(0, 0, editor.document.lineCount, 0);
        assert.ok(await editor.edit(edit => edit.replace(entire, appManifest)));
        assert.ok(await editor.document.save());
        await until(() => appSessions.browser.recentOutput(pending.runtime, 30).output.includes('fixture reached the browser build'));
        assert.ok(appSessions.list().runtimes.some(session => session.runtime === runtime));
        console.log('Saving a repaired Cargo manifest retried the original browser session');
    } finally {
        try { if (runtime) await appSessions.browser.stop(runtime); }
        finally { await settings.update('apps', previous, vscode.ConfigurationTarget.Workspace); }
    }
}
