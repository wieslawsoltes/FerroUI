import { build } from 'esbuild';
import { copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';

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
        entryPoints: { designer: 'webview/main.ts', properties: 'webview/properties.ts', outline: 'webview/outline.ts',
            toolbox: 'webview/toolbox.ts', details: 'webview/details.ts', assets: 'webview/assets.ts', runtime: 'webview/runtime.ts',
            hosted: 'webview/hosted.ts', stages: 'webview/stages.ts' },
        outdir: 'dist',
        bundle: true,
        platform: 'browser',
        target: 'chrome128',
        sourcemap: true,
    }),
]);

const codicons = 'node_modules/@vscode/codicons';
await mkdir('dist/icons', { recursive: true });
await Promise.all([
    ...['codicon.css', 'codicon.ttf'].map(file => copyFile(`${codicons}/dist/${file}`, `dist/icons/${file}`)),
    ...['preview.svg', 'list-tree.svg', 'settings-gear.svg', 'extensions.svg', 'symbol-event.svg', 'library.svg', 'file-media.svg']
        .map(file => copyFile(`${codicons}/src/icons/${file}`, `dist/icons/${file}`)),
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
licenses.push(`Microsoft VS Code Codicons (https://github.com/microsoft/vscode-codicons)\n${(await readFile(`${codicons}/LICENSE`, 'utf8')).trim()}\n\n${(await readFile(`${codicons}/LICENSE-CODE`, 'utf8')).trim()}`);
await writeFile('dist/THIRD-PARTY-NOTICES.txt', `${licenses.join('\n\n')}\n`);
