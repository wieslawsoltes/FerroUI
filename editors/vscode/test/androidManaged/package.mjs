import { existsSync, readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
const stage = process.env.FERROUI_MANAGED_TEST_DIR;
const args = process.argv.slice(2);
const root = process.env.FERROUI_ANDROID_WORKSPACE;
const packageName = args[args.indexOf('--package') + 1];
const abi = args[args.indexOf('--abi') + 1];
const output = args[args.indexOf('--out') + 1];
if (packageName !== 'managed-fixture' || abi !== 'arm64-v8a' || !process.env.FERROUI_ANDROID_DEBUG_KEYSTORE?.endsWith('/android/debug.keystore')) {
    process.exit(33);
}
const source = readFileSync(join(root, 'app/lib.rs'), 'utf8');
while (existsSync(join(stage, 'hold'))) await delay(25);
if (source.includes('fail-build')) {
    console.error('controlled compiler error');
    process.exit(17);
}
mkdirSync(output, { recursive: true });
writeFileSync(join(output, packageName + '-' + abi + '-debug.apk'), source.includes('bad-startup') ? 'bad' : 'good');
console.log('staged Android package');
