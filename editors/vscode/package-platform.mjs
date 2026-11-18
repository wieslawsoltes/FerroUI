import { spawnSync } from 'node:child_process';
import { chmod, copyFile, mkdir, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [directory, output] = process.argv.slice(2);
const { platform, arch } = process;
if (!directory || !['darwin', 'linux', 'win32'].includes(platform) || !['x64', 'arm64'].includes(arch)) {
    throw new Error('Pass the directory containing both native binaries for this macOS, Linux or Windows machine.');
}
const names = ['ferroui-xaml-lsp', 'ferroui-designer-preview'].map(name => name + (platform === 'win32' ? '.exe' : ''));
for (const name of names) {
    const info = await stat(join(directory, name));
    if (!info.isFile()) throw new Error(`Expected a native executable at ${join(directory, name)}`);
}
await mkdir('bin', { recursive: true });
for (const name of names) {
    const destination = join('bin', name);
    await copyFile(join(directory, name), destination);
    if (platform !== 'win32') await chmod(destination, 0o755);
}
const packager = resolve('node_modules/@vscode/vsce/vsce');
const args = [packager, 'package', '--no-dependencies', '--target', `${platform}-${arch}`];
if (output) args.push('--out', output);
const result = spawnSync(process.execPath, args, { stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
