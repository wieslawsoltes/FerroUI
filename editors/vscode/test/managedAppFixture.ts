import { constants } from 'node:fs';
import { chmod, copyFile, link, mkdir, readFile, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';

const apps = [
    { package: 'account-desk', label: 'Account Desk', heading: 'Open customer records', filter: 'Filter customers', background: '#E4EEE8' },
    { package: 'routing-queue', label: 'Routing Queue', heading: 'Review incoming routes', filter: 'Filter routes', background: '#E6EDF8' },
] as const;

/** Reuse the native test app. Cargo still resolves the two real workspace packages; the test
 * driver supplies the prebuilt executable so the VS Code suite does not compile two UI graphs. */
export async function withManagedAppFixture(workspace: vscode.Uri, executable: string,
    run: (fixture: { root: string; files: string[]; labels: string[]; expected: string[] }) => Promise<void>): Promise<void> {
    const root = path.join(workspace.fsPath, 'Concurrent Apps');
    const write = async (name: string, source: string) => {
        const file = path.join(root, name);
        await mkdir(path.dirname(file), { recursive: true });
        await writeFile(file, source);
        return file;
    };
    await write('Cargo.toml', '[workspace]\nresolver = "2"\nmembers = ["account-desk", "routing-queue", "shared"]\n');
    await write('shared/Cargo.toml', '[package]\nname = "shared-app-data"\nversion = "0.1.0"\nedition = "2024"\n');
    await write('shared/src/lib.rs', 'pub const WORKSPACE: &str = "Northwind";\n');
    const xaml = await readFile(path.join(workspace.fsPath, 'LiveCustomerWindow.xaml'), 'utf8');
    const files: string[] = [];
    for (const app of apps) {
        await write(`${app.package}/Cargo.toml`, `[package]\nname = "${app.package}"\nversion = "0.1.0"\nedition = "2024"\n[dependencies]\nshared-app-data = { path = "../shared" }\n`);
        await write(`${app.package}/src/main.rs`, 'fn main() {}\n');
        const view = xaml.replace('Open customer records', app.heading).replace('Customer workbench', app.label)
            .replace('Filter customers', app.filter).replace('#E4EEE8', app.background);
        files.push(await write(`${app.package}/Window.xaml`, view));
    }
    const artifact = path.join(root, 'target', 'debug', 'native-app');
    await mkdir(path.dirname(artifact), { recursive: true });
    try { await link(executable, artifact); }
    catch { await copyFile(executable, artifact, constants.COPYFILE_FICLONE); }
    const cargo = await write('test-cargo', cargoDriver(root, artifact));
    await chmod(cargo, 0o700);
    const settings = vscode.workspace.getConfiguration('ferroui.hotReload', workspace);
    const previousCargo = settings.inspect<string>('cargoPath')?.globalValue;
    const previousTargets = settings.inspect<unknown[]>('targets')?.workspaceValue;
    try {
        await settings.update('cargoPath', cargo, vscode.ConfigurationTarget.Global);
        await settings.update('targets', apps.map((app, index) => ({
            label: app.label, manifestPath: 'Concurrent Apps/Cargo.toml', package: app.package,
            binary: app.package, args: ['--xaml-live', files[index], '--mcp', '--mcp-input'],
        })), vscode.ConfigurationTarget.Workspace);
        await run({ root, files, labels: apps.map(app => app.label), expected: apps.map(app => app.heading) });
    } finally {
        await settings.update('targets', previousTargets, vscode.ConfigurationTarget.Workspace);
        await settings.update('cargoPath', previousCargo, vscode.ConfigurationTarget.Global);
    }
}

function cargoDriver(root: string, artifact: string): string {
    return `#!/usr/bin/env node
const { execFileSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { join } = require('node:path');
const args = process.argv.slice(2);
const option = name => args[args.indexOf(name) + 1];
if (args[0] === 'metadata') {
    process.stdout.write(execFileSync('cargo', args));
} else if (args[0] === 'build') {
    const packageName = option('--package');
    const root = ${JSON.stringify(root)};
    if (existsSync(join(root, packageName, 'reject-build'))) {
        console.error('Build rejected for ' + packageName);
        process.exit(1);
    }
    const cargo = JSON.parse(execFileSync('cargo', ['metadata', '--manifest-path', option('--manifest-path'), '--no-deps', '--format-version', '1']));
    const selected = cargo.packages.find(pkg => pkg.name === packageName);
    const shared = cargo.packages.find(pkg => pkg.name === 'shared-app-data');
    for (const pkg of [shared, selected]) {
        const target = pkg.targets.find(target => target.name === option('--bin')) || pkg.targets[0];
        console.log(JSON.stringify({ reason: 'compiler-artifact', package_id: pkg.id, manifest_path: pkg.manifest_path,
            target, executable: pkg === selected ? ${JSON.stringify(artifact)} : null, features: [] }));
    }
}
`;
}
