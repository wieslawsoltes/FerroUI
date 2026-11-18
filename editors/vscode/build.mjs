import { build } from 'esbuild';
import { readFile, readdir, writeFile } from 'node:fs/promises';

const [host] = await Promise.all([
    build({
        entryPoints: { extension: 'src/extension.ts', test: 'test/suite.ts' },
        outdir: 'dist',
        bundle: true,
        platform: 'node',
        target: 'node20',
        external: ['vscode'],
        metafile: true,
        sourcemap: true,
    }),
    build({
        entryPoints: ['webview/main.ts'],
        outfile: 'dist/designer.js',
        bundle: true,
        platform: 'browser',
        target: 'chrome128',
        sourcemap: true,
    }),
]);

const output = Object.entries(host.metafile.outputs)
    .find(([file]) => file.replaceAll('\\', '/') === 'dist/extension.js')[1];
const modules = Object.keys(output.inputs)
    .map(file => file.replaceAll('\\', '/').match(/(?:^|\/)node_modules\/((?:@[^/]+\/)?[^/]+)/)?.[1])
    .filter(Boolean);
const licenses = await Promise.all([...new Set(modules)].sort().map(async name => {
    const dir = `node_modules/${name}`;
    const files = await readdir(dir);
    const license = files.find(file => /^licen[sc]e(?:\.|$)/i.test(file));
    if (!license) throw new Error(`Missing bundled license for ${name}`);
    return `${name}\n${(await readFile(`${dir}/${license}`, 'utf8')).trim()}`;
}));
await writeFile('dist/THIRD-PARTY-NOTICES.txt', `${licenses.join('\n\n')}\n`);
