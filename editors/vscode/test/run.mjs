import { runTests } from '@vscode/test-electron';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const extension = fileURLToPath(new URL('../', import.meta.url));
const sandbox = await mkdtemp(join(tmpdir(), 'ferroui-vscode-'));
const workspace = join(sandbox, 'workspace');
const profile = join(sandbox, 'profile');
const bin = resolve(process.env.FERROUI_TEST_BIN_DIR ?? join(extension, '../../target/debug'));
const executable = name => join(bin, name + (process.platform === 'win32' ? '.exe' : ''));
try {
    await cp(join(extension, 'test/fixtures'), workspace, { recursive: true });
    const animation = join(workspace, 'animation_view.rs');
    // Seed the CRLF fixture before VS Code watches or loads its text model.
    await writeFile(animation, (await readFile(animation, 'utf8')).replace(/\r?\n/g, '\r\n'));
    await mkdir(join(workspace, 'Assets'));
    await writeFile(join(workspace, 'Assets', 'Preview Image.png'), Buffer.from(
        'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==', 'base64'));
    await mkdir(join(profile, 'User'), { recursive: true });
    await writeFile(join(profile, 'User/settings.json'), JSON.stringify({
        'ferroui.languageServer.path': executable('ferroui-xaml-lsp'),
        'ferroui.preview.path': executable('ferroui-designer-preview'),
        'workbench.startupEditor': 'none',
        'telemetry.telemetryLevel': 'off',
        'files.autoSave': 'off',
        'files.enableTrash': false,
    }));
    await runTests({
        extensionDevelopmentPath: extension,
        extensionTestsPath: join(extension, 'dist/test.js'),
        vscodeExecutablePath: process.env.FERROUI_TEST_VSCODE,
        version: process.env.FERROUI_TEST_VSCODE_VERSION,
        launchArgs: [workspace, '--user-data-dir', profile, '--extensions-dir', join(sandbox, 'extensions'),
            '--skip-welcome', '--skip-release-notes', '--disable-workspace-trust'],
    });
} finally {
    await rm(sandbox, { recursive: true, force: true, maxRetries: 3 });
}
