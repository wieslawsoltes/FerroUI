import { runTests } from '@vscode/test-electron';
import { constants } from 'node:fs';
import { access, cp, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareInstalledRuntime } from './installedRuntimeWorkspace.mjs';
import { installVsix } from './vscodeCli.mjs';
import { testVSCodeExecutable } from './vscodeDownload.mjs';

const extension = fileURLToPath(new URL('../', import.meta.url));
const toolkit = resolve(extension, '../..');
const [vsixPath] = process.argv.slice(2);
if (!vsixPath) throw new Error('Pass the VSIX built for the current workspace host.');
const vsix = resolve(vsixPath);
const manifest = JSON.parse(await readFile(join(extension, 'package.json'), 'utf8'));
const minimum = manifest.engines.vscode.replace(/^\^/, '');
const runtimeEnabled = process.env.FERROUI_TEST_INSTALLED_RUNTIME === '1';
const debuggerPath = process.env.FERROUI_TEST_INSTALLED_LLDB_DAP ? resolve(process.env.FERROUI_TEST_INSTALLED_LLDB_DAP) : undefined;
if (debuggerPath) {
    if (!runtimeEnabled) throw new Error('Set FERROUI_TEST_INSTALLED_RUNTIME=1 to run installed debugger acceptance.');
    await access(debuggerPath, constants.X_OK);
}

const failedVersions = new Set();
for (const [version, supplied] of [[minimum, process.env.FERROUI_TEST_VSCODE_MIN], ['stable', process.env.FERROUI_TEST_VSCODE]]) {
    let sandbox;
    let runtime;
    try {
        const executable = await testVSCodeExecutable(supplied, { version });
        sandbox = await mkdtemp(join(tmpdir(), 'ferroui-installed-'));
        // Cargo records canonical source paths in the test app's debug information.
        sandbox = await realpath(sandbox);
        const workspace = join(sandbox, 'workspace');
        const profile = join(sandbox, 'profile');
        const extensions = join(sandbox, 'extensions');
        const driver = join(sandbox, 'driver');
        const args = ['--user-data-dir', profile, '--extensions-dir', extensions];
        await mkdir(join(profile, 'User'), { recursive: true });
        await mkdir(driver);
        await cp(join(extension, 'test/fixtures'), workspace, { recursive: true });
        if (runtimeEnabled) runtime = await prepareInstalledRuntime(workspace, sandbox, toolkit, Boolean(debuggerPath));
        const contributes = debuggerPath ? {
            breakpoints: [{ language: 'rust' }],
            debuggers: [{ type: 'lldb-dap', label: 'LLDB', program: debuggerPath }],
        } : undefined;
        await writeFile(join(driver, 'package.json'), JSON.stringify({ name: 'ferroui-vsix-driver', publisher: 'ferroui-test',
            version: '0.0.0', engines: { vscode: manifest.engines.vscode }, contributes }));
        await writeFile(join(profile, 'User/settings.json'), JSON.stringify({
            'workbench.startupEditor': 'none', 'telemetry.telemetryLevel': 'off', ...runtime?.settings,
        }));
        await installVsix(executable, [...args, '--install-extension', vsix]);
        await runTests({ vscodeExecutablePath: executable, extensionDevelopmentPath: driver,
            extensionTestsPath: join(extension, 'dist/installed-test.js'),
            extensionTestsEnv: {
                FERROUI_TEST_INSTALLED_ROOT: extensions, ...runtime?.environment,
                FERROUI_TEST_HOSTED_SOURCE: process.env.FERROUI_TEST_HOSTED_SOURCE ?? join(toolkit, 'samples/TodoWorkbench'),
            },
            launchArgs: [workspace, ...args, '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust'] });
    } catch (error) {
        failedVersions.add(version);
        console.error(`The installed VS Code ${version} check failed.`, error);
    } finally {
        try { runtime?.clean(); }
        catch (error) {
            failedVersions.add(version);
            console.error(`The installed VS Code ${version} fixture could not be cleaned.`, error);
        }
        try { if (sandbox) await rm(sandbox, { recursive: true, force: true, maxRetries: 3 }); }
        catch (error) {
            failedVersions.add(version);
            console.error(`The installed VS Code ${version} profile and workspace could not be cleaned.`, error);
        }
    }
}
if (failedVersions.size) throw new Error(`Installed package checks failed for VS Code ${[...failedVersions].join(', ')}.`);
