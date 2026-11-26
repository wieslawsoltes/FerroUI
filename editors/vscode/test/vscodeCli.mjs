import { resolveCliPathFromVSCodeExecutablePath } from '@vscode/test-electron';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';

export function windowsCliEntry(launcher) {
    // The archive's launcher names the active resource tree. New Windows builds
    // put that tree in a versioned directory while older builds use the root.
    const path = launcher.match(/"%~dp0\.\.\\((?:[\w.-]+\\)?resources\\app\\out\\cli\.js)"\s+%\*/i)?.[1];
    if (!path || path.split('\\').some(segment => segment === '.' || segment === '..')) {
        throw new Error('The Windows VS Code launcher does not identify its CLI entry point.');
    }
    return path.split('\\');
}

export async function installVsix(executable, args) {
    const cli = resolveCliPathFromVSCodeExecutablePath(executable);
    const options = { stdio: 'inherit', timeout: 60_000, windowsHide: true };
    let result;
    if (process.platform === 'win32') {
        const entry = join(dirname(executable), ...windowsCliEntry(await readFile(cli, 'utf8')));
        // Mirror the launcher while keeping paths and flags out of a command shell.
        result = spawnSync(executable, [entry, ...args], { ...options, env: { ...process.env, ELECTRON_RUN_AS_NODE: '1', VSCODE_DEV: '' } });
    } else {
        result = spawnSync(cli, args, options);
    }
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`VS Code could not install the VSIX (exit ${result.status}).`);
}
