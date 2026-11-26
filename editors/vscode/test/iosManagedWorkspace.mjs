import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function iosManagedWorkspace(workspace, sandbox, bundle) {
    const toolkit = join(sandbox, 'toolkit');
    const scripts = join(toolkit, 'scripts', 'ios');
    const broken = join(sandbox, 'missing-inspector');
    const bundleId = execFileSync('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', join(bundle, 'Info.plist')],
        { encoding: 'utf8' }).trim();
    const target = process.arch === 'arm64' ? 'arm64-apple-ios14.0-simulator' : 'x86_64-apple-ios14.0-simulator';
    execFileSync('/usr/bin/xcrun', ['--sdk', 'iphonesimulator', 'clang', '-target', target, '-x', 'c', '-', '-o', broken],
        { input: 'int main(void) { return 7; }' });
    await Promise.all([scripts, join(workspace, 'app'), join(workspace, 'notes')].map(directory => mkdir(directory, { recursive: true })));
    await Promise.all([
        writeFile(join(scripts, 'bundle.sh'), '#!/bin/bash\nexec "$FERROUI_IOS_TEST_NODE" "$FERROUI_IOS_TEST_PACKAGER" "$@"\n'),
        writeFile(join(workspace, 'Cargo.toml'), '[workspace]\nresolver = "3"\nmembers = ["app","notes"]\n'),
        writeFile(join(workspace, 'app/Cargo.toml'), '[package]\nname = "managed-simulator"\nversion = "0.1.0"\n[[bin]]\nname = "managed-simulator"\npath = "main.rs"\n'),
        writeFile(join(workspace, 'notes/Cargo.toml'), '[package]\nname = "fixture-notes"\nversion = "0.1.0"\n[lib]\npath = "lib.rs"\n'),
        writeFile(join(workspace, 'app/main.rs'), '// Application build fixture\n'),
        writeFile(join(workspace, 'app/View.xaml'), '<TextBlock Text="Initial state"/>\n'),
        writeFile(join(workspace, 'notes/lib.rs'), '// Separate Cargo crate\n'),
    ]);
    return {
        settings: { 'ferroui.iosSimulator.apps': [{
            id: 'managed', label: 'Contact Review', bundleId, args: ['--mcp', '--mcp-input'],
            build: { package: 'managed-simulator', binary: 'managed-simulator', toolchain: toolkit, reloadOnSave: true },
        }] },
        env: {
            FERROUI_IOS_MANAGED_TEST: sandbox,
            FERROUI_IOS_TEST_NODE: process.execPath,
            FERROUI_IOS_TEST_PACKAGER: fileURLToPath(new URL('./iosManagedPackager.mjs', import.meta.url)),
            FERROUI_IOS_TEST_BUNDLE: bundle,
            FERROUI_IOS_TEST_BROKEN: broken,
            CARGO_TARGET_DIR: join(workspace, 'target'),
        },
    };
}
