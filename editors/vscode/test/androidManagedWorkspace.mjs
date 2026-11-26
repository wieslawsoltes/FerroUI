import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function androidManagedWorkspace(workspace, sandbox) {
    if (process.platform === 'win32') throw new Error('The managed Android fixture requires a macOS or Linux host.');
    const toolkit = join(sandbox, 'toolkit');
    const scripts = join(toolkit, 'scripts', 'android');
    const adb = join(sandbox, 'adb');
    const [adbSource, packageSource] = await Promise.all([
        readFile(new URL('./androidManaged/adb.mjs', import.meta.url), 'utf8'),
        readFile(new URL('./androidManaged/package.mjs', import.meta.url), 'utf8'),
        ...[scripts, join(workspace, 'app'), join(workspace, 'notes')].map(directory => mkdir(directory, { recursive: true })),
    ]);
    await Promise.all([
        writeFile(adb, '#!/bin/bash\nexec "$FERROUI_MANAGED_TEST_NODE" "$FERROUI_MANAGED_TEST_DIR/adb.mjs" "$@"\n', { mode: 0o755 }),
        writeFile(join(sandbox, 'adb.mjs'), adbSource),
        writeFile(join(sandbox, 'package.mjs'), packageSource),
        writeFile(join(scripts, 'apk.sh'), '#!/bin/bash\nexec "$FERROUI_MANAGED_TEST_NODE" "$FERROUI_MANAGED_TEST_DIR/package.mjs" "$@"\n'),
        writeFile(join(workspace, 'Cargo.toml'), '[workspace]\nresolver = "3"\nmembers = ["app", "notes"]\n'),
        writeFile(join(workspace, 'app/Cargo.toml'), '[package]\nname = "managed-fixture"\nversion = "0.1.0"\n[lib]\npath = "lib.rs"\ncrate-type = ["cdylib"]\n'),
        writeFile(join(workspace, 'notes/Cargo.toml'), '[package]\nname = "fixture-notes"\nversion = "0.1.0"\n[lib]\npath = "lib.rs"\n'),
        writeFile(join(workspace, 'app/lib.rs'), '// cargo application fixture\n'),
        writeFile(join(workspace, 'notes/lib.rs'), '// separate Cargo package\n'),
        writeFile(join(workspace, 'app/View.xaml'), '<TextBlock Text="Initial XAML"/>\n'),
    ]);
    return {
        settings: {
            'ferroui.android.adbPath': adb,
            'ferroui.android.apps': [{
                id: 'managed', label: 'Managed Android Fixture', packageId: 'org.ferroui.managedfixture',
                build: { package: 'managed-fixture', toolchain: toolkit, reloadOnSave: true },
            }],
        },
        env: {
            FERROUI_MANAGED_TEST_DIR: sandbox,
            FERROUI_MANAGED_TEST_NODE: process.execPath,
            CARGO_TARGET_DIR: join(workspace, 'target'),
        },
    };
}
