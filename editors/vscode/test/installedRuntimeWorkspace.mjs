import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { copyFile, cp, mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function prepareInstalledRuntime(workspace, sandbox, toolkit, debuggerEnabled) {
    const fixture = join(workspace, 'installed-runtime');
    const receipts = join(sandbox, 'runtime-receipts');
    const targetDirectory = resolve(process.env.CARGO_TARGET_DIR ?? join(toolkit, 'target'));
    // The two VS Code versions share dependency artifacts, but each builds an app from a new workspace.
    const name = `ferroui-installed-runtime-${randomUUID().slice(0, 13)}`;
    const crates = {
        'ferroui-base': 'src/FerroUI.Base',
        'ferroui-controls': 'src/FerroUI.Controls',
        'ferroui-hot-reload': 'src/FerroUI.HotReload',
        'ferroui-inspect': 'src/FerroUI.Inspect',
        'ferroui-markup-xaml-loader': 'src/Markup/FerroUI.Markup.Xaml.Loader',
        'ferroui-mcp': 'src/FerroUI.Mcp',
        'ferroui-themes-simple': 'src/FerroUI.Themes.Simple',
    };
    const dependency = (crate, features = []) => `{ path = ${JSON.stringify(join(toolkit, crate))}, features = ${JSON.stringify(features)} }`;
    const manifest = [
        '[package]', `name = "${name}"`, 'version = "0.0.0"', 'edition = "2024"', 'publish = false',
        '[[bin]]', `name = "${name}"`, 'path = "main.rs"',
        '[dependencies]', 'serde_json = "1"',
        ...Object.entries(crates).map(([name, crate]) => `${name} = ${dependency(crate)}`),
        `ferroui-view = ${dependency('src/FerroUI.View', ['desktop', 'hot-reload'])}`,
        "[target.'cfg(target_os = \"linux\")'.dependencies]", 'libc = "0.2"',
        ...(debuggerEnabled ? [`[profile.dev.package.${name}]`, 'debug = 2'] : []),
        '',
    ].join('\n');
    const environment = { CARGO_TARGET_DIR: targetDirectory, FERROUI_TEST_RUNTIME_RECEIPTS: receipts };
    const cargo = join(sandbox, process.platform === 'win32' ? 'cargo-receipt.exe' : 'cargo-receipt');
    const settings = {
        'ferroui.hotReload.cargoPath': cargo,
        'ferroui.hotReload.targets': [{ label: 'Release Review', manifestPath: 'installed-runtime/Cargo.toml', package: name, binary: name }],
    };
    if (debuggerEnabled) {
        const configuration = { name: 'Debug Release Review', type: 'lldb-dap', request: 'attach',
            pid: '${command:ferroui.hotReload.processId}', stopOnEntry: false,
            // The fixture's own symbols are local. A Linux image may configure network lookup for system libraries.
            ...(process.platform === 'linux' ? { initCommands: ['settings set symbols.enable-external-lookup false'] } : {}),
        };
        settings.launch = { configurations: [configuration] };
        environment.FERROUI_TEST_RUNTIME_DEBUG = configuration.name;
    }
    await cp(new URL('./native/installed-runtime/', import.meta.url), fixture, { recursive: true });
    await Promise.all([
        mkdir(receipts, { mode: 0o700 }),
        copyFile(join(toolkit, 'Cargo.lock'), join(fixture, 'Cargo.lock')),
        writeFile(join(fixture, 'Cargo.toml'), manifest),
    ]);
    const cargoReceipt = fileURLToPath(new URL('./native/cargo_receipt.rs', import.meta.url));
    execFileSync('rustc', ['--edition=2024', cargoReceipt, '-o', cargo], { stdio: 'inherit', windowsHide: true });
    // Warm the same Cargo target the installed extension will launch. The package tests
    // can then measure a saved edit and replacement without a cold dependency build.
    const runCargo = (action, args) => execFileSync('cargo', [action, '--manifest-path', join(fixture, 'Cargo.toml'), ...args], {
        cwd: fixture, env: { ...process.env, ...environment }, stdio: 'inherit', windowsHide: true,
    });
    const clean = () => runCargo('clean', ['-p', name]);
    try {
        runCargo('build', ['--bin', name]);
    } catch (error) {
        try { clean(); } catch (cleanupError) { console.error('Installed runtime fixture cleanup failed.', cleanupError); }
        throw error;
    }
    return { settings, environment, clean };
}
