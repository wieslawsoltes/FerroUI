import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import * as path from 'node:path';
import * as vscode from 'vscode';

const apps = [
    { package: 'account-desk', label: 'Account Desk', heading: 'Open customer records', filter: 'Filter customers', background: '#E4EEE8' },
    { package: 'routing-queue', label: 'Routing Queue', heading: 'Review incoming routes', filter: 'Filter routes', background: '#E6EDF8' },
] as const;

/** Compile two small Cargo targets that exec the prebuilt native app under their own PIDs. */
export async function withManagedAppFixture(workspace: vscode.Uri,
    run: (fixture: { root: string; files: string[]; labels: string[]; expected: string[] }) => Promise<void>): Promise<void> {
    const root = await mkdtemp(path.join(workspace.fsPath, 'Concurrent Apps '));
    const write = async (name: string, source: string) => {
        const file = path.join(root, name);
        await mkdir(path.dirname(file), { recursive: true });
        await writeFile(file, source);
        return file;
    };
    const settings = vscode.workspace.getConfiguration('ferroui.hotReload', workspace);
    const previousTargets = settings.inspect<unknown[]>('targets')?.workspaceValue;
    const previousOutput = process.env.CARGO_TARGET_DIR;
    try {
        const manifest = await write('Cargo.toml', '[workspace]\nresolver = "2"\nmembers = ["account-desk", "routing-queue", "shared"]\n');
        await write('shared/Cargo.toml', '[package]\nname = "shared-app-data"\nversion = "0.1.0"\nedition = "2024"\n');
        await write('shared/src/lib.rs', 'pub const WORKSPACE: &str = "Northwind";\n');
        const xaml = await readFile(path.join(workspace.fsPath, 'LiveCustomerWindow.xaml'), 'utf8');
        const files: string[] = [];
        for (const app of apps) {
            await write(`${app.package}/Cargo.toml`, `[package]\nname = "${app.package}"\nversion = "0.1.0"\nedition = "2024"\n[dependencies]\nshared-app-data = { path = "../shared" }\n`);
            await write(`${app.package}/src/main.rs`, launcher);
            await write(`${app.package}/build.rs`, buildScript);
            const view = xaml.replace('Open customer records', app.heading).replace('Customer workbench', app.label)
                .replace('Filter customers', app.filter).replace('#E4EEE8', app.background);
            files.push(await write(`${app.package}/Window.xaml`, view));
        }
        process.env.CARGO_TARGET_DIR = path.join(root, 'target');
        await settings.update('targets', apps.map((app, index) => ({
            label: app.label, manifestPath: path.relative(workspace.fsPath, manifest), package: app.package,
            binary: app.package, args: ['--xaml-live', files[index], '--mcp', '--mcp-input'],
        })), vscode.ConfigurationTarget.Workspace);
        await run({ root, files, labels: apps.map(app => app.label), expected: apps.map(app => app.heading) });
    } finally {
        if (previousOutput === undefined) delete process.env.CARGO_TARGET_DIR;
        else process.env.CARGO_TARGET_DIR = previousOutput;
        try { await settings.update('targets', previousTargets, vscode.ConfigurationTarget.Workspace); }
        finally { await rm(root, { recursive: true, force: true, maxRetries: 3 }); }
    }
}

const launcher = `use std::{os::unix::process::CommandExt, process::Command};

fn main() {
    let error = Command::new(env!("FERROUI_TEST_HOSTED_APP"))
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("Could not open {}: {error}", shared_app_data::WORKSPACE);
    std::process::exit(1);
}
`;

const buildScript = `use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=reject-build");
    assert!(!Path::new("reject-build").exists(), "Fixture requested a build failure");
}
`;
