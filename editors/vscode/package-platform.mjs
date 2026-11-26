import { spawnSync } from 'node:child_process';
import { chmod, copyFile, mkdir, mkdtemp, rename, rm, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

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
const staging = await mkdtemp(join('bin', '.package-'));
try {
    // Give each executable a new inode so running development hosts keep their old signed file.
    for (const name of names) {
        const file = join(staging, name);
        await copyFile(join(directory, name), file);
        if (platform !== 'win32') await chmod(file, 0o755);
    }
    for (const name of names) await rename(join(staging, name), join('bin', name));
} finally {
    await rm(staging, { recursive: true, force: true });
}
const packager = fileURLToPath(new URL('package-vsix.mjs', import.meta.url));
const args = [packager, '--target', `${platform}-${arch}`];
if (output) args.push('--out', resolve(output));
const result = spawnSync(process.execPath, args, { stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
