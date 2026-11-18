import { downloadAndUnzipVSCode, resolveCliPathFromVSCodeExecutablePath, runTests } from '@vscode/test-electron';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const extension = fileURLToPath(new URL('../', import.meta.url));
const [vsixPath] = process.argv.slice(2);
if (!vsixPath) throw new Error('Pass the VSIX built for the current workspace host.');
const vsix = resolve(vsixPath);
const manifest = JSON.parse(await readFile(join(extension, 'package.json'), 'utf8'));
const minimum = manifest.engines.vscode.replace(/^\^/, '');

for (const [version, supplied] of [[minimum, process.env.FERROUI_TEST_VSCODE_MIN], ['stable', process.env.FERROUI_TEST_VSCODE]]) {
    const executable = supplied ?? await downloadAndUnzipVSCode(version);
    const sandbox = await mkdtemp(join(tmpdir(), 'ferroui-installed-'));
    const workspace = join(sandbox, 'workspace');
    const profile = join(sandbox, 'profile');
    const extensions = join(sandbox, 'extensions');
    const driver = join(sandbox, 'driver');
    const args = ['--user-data-dir', profile, '--extensions-dir', extensions];
    try {
        await Promise.all([mkdir(join(profile, 'User'), { recursive: true }), mkdir(driver),
            cp(join(extension, 'test/fixtures'), workspace, { recursive: true })]);
        await writeFile(join(driver, 'package.json'), JSON.stringify({ name: 'ferroui-vsix-driver', publisher: 'ferroui-test',
            version: '0.0.0', engines: { vscode: manifest.engines.vscode } }));
        await writeFile(join(profile, 'User/settings.json'), JSON.stringify({ 'workbench.startupEditor': 'none', 'telemetry.telemetryLevel': 'off' }));
        install(executable, [...args, '--install-extension', vsix]);
        await runTests({ vscodeExecutablePath: executable, extensionDevelopmentPath: driver,
            extensionTestsPath: join(extension, 'dist/installed-test.js'), extensionTestsEnv: { FERROUI_TEST_INSTALLED_ROOT: extensions },
            launchArgs: [workspace, ...args, '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust'] });
    } finally { await rm(sandbox, { recursive: true, force: true, maxRetries: 3 }); }
}

function install(executable, args) {
    const windows = process.platform === 'win32';
    const cli = windows ? join(dirname(executable), 'resources', 'app', 'out', 'cli.js') : resolveCliPathFromVSCodeExecutablePath(executable);
    const options = { stdio: 'inherit', timeout: 60_000, windowsHide: true };
    // Run the Windows CLI with the same Electron executable as its launcher.
    const result = windows ? spawnSync(executable, [cli, ...args], { ...options, env: { ...process.env, ELECTRON_RUN_AS_NODE: '1' } })
        : spawnSync(cli, args, options);
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`VS Code could not install the VSIX (exit ${result.status}).`);
}
