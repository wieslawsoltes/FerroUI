import { execFileSync } from 'node:child_process';
import { copyFile, cp, mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

export async function prepareInstalledRuntime(workspace, sandbox, toolkit) {
    const fixture = join(workspace, 'installed-runtime');
    const receipts = join(sandbox, 'runtime-receipts');
    const targetDirectory = resolve(process.env.CARGO_TARGET_DIR ?? join(toolkit, 'target'));
    const name = 'ferroui-installed-runtime-test';
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
        '',
    ].join('\n');
    const environment = { CARGO_TARGET_DIR: targetDirectory, FERROUI_TEST_RUNTIME_RECEIPTS: receipts };
    await cp(new URL('./native/installed-runtime/', import.meta.url), fixture, { recursive: true });
    await Promise.all([
        mkdir(receipts, { mode: 0o700 }),
        copyFile(join(toolkit, 'Cargo.lock'), join(fixture, 'Cargo.lock')),
        writeFile(join(fixture, 'Cargo.toml'), manifest),
    ]);
    // Warm the same Cargo target the installed extension will launch. The package tests
    // can then measure a saved edit and replacement without a cold dependency build.
    execFileSync('cargo', ['build', '--manifest-path', join(fixture, 'Cargo.toml'), '--bin', name], {
        cwd: fixture, env: { ...process.env, ...environment }, stdio: 'inherit', windowsHide: true,
    });
    return {
        settings: { 'ferroui.hotReload.targets': [{ label: 'Release Review', manifestPath: 'installed-runtime/Cargo.toml', package: name, binary: name }] },
        environment,
    };
}
