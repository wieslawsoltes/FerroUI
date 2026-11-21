import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const extension = new URL('./', import.meta.url);
const cwd = fileURLToPath(extension);
const { repository } = JSON.parse(await readFile(new URL('package.json', extension), 'utf8'));
const revision = spawnSync('git', ['rev-parse', '--verify', 'HEAD'], { cwd, encoding: 'utf8' });
const ref = revision.status === 0 ? revision.stdout.trim() : 'HEAD';
const source = repository.url.replace(/\.git$/, '');
const packager = fileURLToPath(new URL('node_modules/@vscode/vsce/vsce', extension));
const args = [packager, 'package', '--no-dependencies',
    '--baseContentUrl', `${source}/blob/${ref}/${repository.directory}`,
    '--baseImagesUrl', `${source}/raw/${ref}/${repository.directory}`,
    ...process.argv.slice(2)];
const result = spawnSync(process.execPath, args, { cwd, stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
