import { build } from 'esbuild';

await Promise.all([
    build({
        entryPoints: { extension: 'src/extension.ts', test: 'test/suite.ts' },
        outdir: 'dist',
        bundle: true,
        platform: 'node',
        target: 'node20',
        external: ['vscode'],
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
