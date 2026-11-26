import { execFileSync } from 'node:child_process';
import { constants } from 'node:fs';
import { appendFile, copyFile, cp, readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const stage = process.env.FERROUI_IOS_MANAGED_TEST;
const workspace = process.env.FERROUI_IOS_WORKSPACE;
const args = process.argv.slice(2);
const argument = option => args[args.indexOf(option) + 1];
const binary = argument('--bin');
if (binary !== 'managed-simulator' || argument('--package') !== 'managed-simulator') throw new Error('Unexpected simulator Cargo target.');

const source = await readFile(join(workspace, 'app/main.rs'), 'utf8');
await appendFile(join(stage, 'builds'), source);
const hold = join(stage, 'hold');
while (await stat(hold).then(() => true, () => false)) await delay(25);
if (source.includes('fail-build')) {
    console.error('controlled simulator compiler error');
    process.exit(17);
}

const destination = join(argument('--out'), binary + '.app');
await cp(process.env.FERROUI_IOS_TEST_BUNDLE, destination, { recursive: true, mode: constants.COPYFILE_FICLONE });
if (source.includes('fail-startup')) {
    const executable = execFileSync('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(destination, 'Info.plist')],
        { encoding: 'utf8' }).trim();
    await copyFile(process.env.FERROUI_IOS_TEST_BROKEN, join(destination, executable));
    execFileSync('/usr/bin/codesign', ['--force', '--sign', '-', '--timestamp=none', destination]);
}
console.log('staged simulator bundle');
