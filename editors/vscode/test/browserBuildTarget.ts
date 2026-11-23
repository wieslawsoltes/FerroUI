import { strict as assert } from 'node:assert';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { resolveBrowserBuild } from '../src/browserBuild';
import { fileKey } from '../src/workspacePaths';

export async function verifyBrowserBuildTarget(workspace: vscode.Uri): Promise<void> {
    const fixture = vscode.Uri.joinPath(workspace, 'Browser Cargo targets');
    const browserCrate = 'src/Browser/FerroUI.Browser';
    const toolkit = `toolkit/${browserCrate}`;
    const example = `${toolkit}/examples/live_inspector`;
    const files = [
        ['toolkit/Cargo.toml', `[workspace]\nresolver = "2"\nmembers = ["${browserCrate}"]\n`],
        ['toolkit/scripts/build-browser.sh', '#!/bin/bash\n'],
        [`${toolkit}/Cargo.toml`, '[package]\nname = "ferroui-browser"\nversion = "0.1.0"\nedition = "2021"\n'],
        [`${toolkit}/src/lib.rs`, ''],
        [`${example}/main.rs`, 'fn main() {}\n'],
        ['app/Cargo.toml', '[package]\nname = "live_inspector"\nversion = "0.1.0"\nedition = "2021"\n'],
        ['app/src/main.rs', 'fn main() {}\n'],
    ];
    for (const [name, content] of files) {
        const uri = vscode.Uri.joinPath(fixture, name);
        await vscode.workspace.fs.createDirectory(vscode.Uri.file(path.dirname(uri.fsPath)));
        await vscode.workspace.fs.writeFile(uri, Buffer.from(content));
    }
    const resolve = (manifest: string) => resolveBrowserBuild({
        id: 'live-inspector', folder: workspace.toString(), label: 'Live Inspector', width: 960, height: 620,
        build: { application: 'live_inspector', toolchain: vscode.Uri.joinPath(fixture, 'toolkit').fsPath,
            manifestPath: vscode.Uri.joinPath(fixture, manifest).fsPath, features: [], threads: false },
    }, AbortSignal.timeout(20_000));
    const [app, bundled] = await Promise.all([resolve('app/Cargo.toml'), resolve('toolkit/Cargo.toml')]);
    assert.equal(app.cargo.kind, 'bin');
    assert.equal(app.cargo.packageName, 'live_inspector');
    assert.equal(app.hostPage, fileKey(vscode.Uri.joinPath(fixture, 'app/wwwroot').fsPath));
    assert.equal(bundled.cargo.kind, 'example');
    assert.equal(bundled.cargo.packageName, 'ferroui-browser');
    assert.equal(bundled.hostPage, fileKey(vscode.Uri.joinPath(fixture, example, 'wwwroot').fsPath));
    console.log('VS Code resolved colliding browser targets through their owning Cargo workspace');
}
