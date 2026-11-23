import { constants } from 'node:fs';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function browserDebugWorkspace(workspace, sandbox, site) {
    const toolkit = join(sandbox, 'browser-debug-toolkit');
    const scripts = join(toolkit, 'scripts');
    const host = join(workspace, 'wwwroot');
    await Promise.all([mkdir(scripts, { recursive: true }), mkdir(host, { recursive: true })]);
    await writeFile(join(workspace, 'Cargo.toml'), '[package]\nname = "live_inspector"\nversion = "0.1.0"\nedition = "2021"\n\n[[bin]]\nname = "live_inspector"\npath = "main.rs"\n');
    await writeFile(join(workspace, 'main.rs'), 'fn main() {}\n');
    const html = await readFile(join(site, 'index.html'), 'utf8');
    await writeFile(join(host, 'index.html'), html.replace('</body>', '<script type="module" src="./probe.js"></script></body>'));
    await writeFile(join(host, 'probe.js'), 'globalThis.dispatchCounter = 0;\nfunction markDispatched() {\n    globalThis.dispatchCounter += 1;\n}\nsetInterval(markDispatched, 400);\n');
    await cp(site, join(toolkit, 'site'), { recursive: true, mode: constants.COPYFILE_FICLONE });
    await writeFile(join(scripts, 'build-browser.sh'), '#!/usr/bin/env bash\nexec node "$(dirname "$0")/stage.mjs" "$@"\n');
    await writeFile(join(scripts, 'stage.mjs'), `
import { constants } from 'node:fs';
import { access, cp, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const out = args[args.indexOf('--out') + 1];
const messages = args[args.indexOf('--editor-messages') + 1];
const workspace = process.env.FERROUI_BROWSER_WORKSPACE;
if (await access(join(workspace, 'fail-build')).then(() => true, () => false)) {
    console.error('Controlled build failure for the debugger fixture.');
    process.exit(17);
}
const options = { recursive: true, mode: constants.COPYFILE_FICLONE };
await cp(fileURLToPath(new URL('../site/', import.meta.url)), out, options);
await cp(join(workspace, 'wwwroot'), out, options);
await writeFile(messages, '');
`);
    return toolkit;
}
