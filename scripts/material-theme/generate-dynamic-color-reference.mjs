#!/usr/bin/env node
// Copyright 2026 FerroUI contributors. SPDX-License-Identifier: MIT.
// Execute the pinned, unmodified Google TypeScript implementation. The Rust
// implementation is deliberately not involved in producing expected ARGBs.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';

const COMMIT = 'bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3';
const VERIFIED_DATE = '2026-10-25';
const [sourceArgument, compilerArgument, outputArgument] = process.argv.slice(2);
if (!sourceArgument || !compilerArgument || !outputArgument) {
  throw new Error('Usage: node generate-dynamic-color-reference.mjs <pinned MCU typescript directory> <typescript.js compiler> <fixture directory>');
}
const source = path.resolve(sourceArgument);
const output = path.resolve(outputArgument);
const compiler = createRequire(import.meta.url)(path.resolve(compilerArgument));
const build = fs.mkdtempSync(path.join(os.tmpdir(), 'ferroui-mcu-reference-'));
const hashes = {};
function compile(directory = source) {
  for (const entry of fs.readdirSync(directory, {withFileTypes: true}).sort((a,b) => a.name.localeCompare(b.name))) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) { compile(file); continue; }
    if (!entry.name.endsWith('.ts') || entry.name.endsWith('.d.ts') || entry.name.endsWith('_test.ts')) continue;
    const relative = path.relative(source, file).replaceAll(path.sep, '/');
    const contents = fs.readFileSync(file, 'utf8');
    hashes[relative] = crypto.createHash('sha256').update(contents).digest('hex');
    let javascript = compiler.transpileModule(contents, {
      compilerOptions: {target: compiler.ScriptTarget.ES2022, module: compiler.ModuleKind.ES2022},
      fileName: file,
    }).outputText;
    // Upstream's package uses extensionless ESM imports in a few files. Resolve
    // those to the emitted .js files without changing any algorithm statements.
    javascript = javascript.replace(/(from\s+['"])(\.[^'"]+)(['"])/g,
      (match, before, specifier, after) => path.extname(specifier) ? match : `${before}${specifier}.js${after}`);
    const destination = path.join(build, relative.replace(/\.ts$/, '.js'));
    fs.mkdirSync(path.dirname(destination), {recursive: true});
    fs.writeFileSync(destination, javascript);
  }
}

try {
  compile();
  fs.writeFileSync(path.join(build, 'package.json'), '{"type":"module"}\n');
  const {DynamicScheme} = await import(pathToFileURL(path.join(build, 'dynamiccolor/dynamic_scheme.js')));
  const {Hct} = await import(pathToFileURL(path.join(build, 'hct/hct.js')));
  const {Variant} = await import(pathToFileURL(path.join(build, 'dynamiccolor/variant.js')));
  if (DynamicScheme.DEFAULT_SPEC_VERSION !== '2021' || DynamicScheme.DEFAULT_PLATFORM !== 'phone') {
    throw new Error('Upstream defaults changed; update the reviewed source contract before regenerating.');
  }
  const roles = [
    'Background', 'Error', 'ErrorContainer', 'InverseOnSurface', 'InversePrimary', 'InverseSurface',
    'OnBackground', 'OnError', 'OnErrorContainer', 'OnPrimary', 'OnPrimaryContainer', 'OnPrimaryFixed',
    'OnPrimaryFixedVariant', 'OnSecondary', 'OnSecondaryContainer', 'OnSecondaryFixed',
    'OnSecondaryFixedVariant', 'OnSurface', 'OnSurfaceVariant', 'OnTertiary', 'OnTertiaryContainer',
    'OnTertiaryFixed', 'OnTertiaryFixedVariant', 'Outline', 'OutlineVariant', 'Primary',
    'PrimaryContainer', 'PrimaryFixed', 'PrimaryFixedDim', 'Scrim', 'Secondary', 'SecondaryContainer',
    'SecondaryFixed', 'SecondaryFixedDim', 'Surface', 'SurfaceBright', 'SurfaceContainer',
    'SurfaceContainerHigh', 'SurfaceContainerHighest', 'SurfaceContainerLow', 'SurfaceContainerLowest',
    'SurfaceDim', 'SurfaceVariant', 'Tertiary', 'TertiaryContainer', 'TertiaryFixed', 'TertiaryFixedDim',
    'SurfaceTint',
  ];
  const styles = ['MONOCHROME', 'NEUTRAL', 'TONAL_SPOT', 'VIBRANT', 'EXPRESSIVE', 'FIDELITY', 'CONTENT', 'RAINBOW', 'FRUIT_SALAD'];
  const seeds = [0xff6750a4, 0xff00639b, 0xff4285f4, 0xff000000, 0xffffffff, 0xffff0000, 0xff00ff00, 0xff0000ff,
    0xffffff00, 0xff00ffff, 0xffff00ff, 0xffff9800, 0xff795548, 0xff009688, 0xff222222, 0xff8e7b55];
  const contrasts = [-1, -0.5, 0, 0.25, 0.5, 1];
  const hex = value => (value >>> 0).toString(16).padStart(8, '0');
  const rows = [`spec,seed,style,dark,contrast,${roles.join(',')}`];
  let schemes = 0;
  for (const specVersion of ['2021', '2025']) for (const seed of seeds) for (const style of styles) {
    for (const isDark of [false, true]) for (const contrastLevel of contrasts) {
      const scheme = new DynamicScheme({sourceColorHct: Hct.fromInt(seed), variant: Variant[style], isDark,
        contrastLevel, specVersion, platform: 'phone'});
      const colors = roles.map(role => hex(scheme[role[0].toLowerCase() + role.slice(1)]));
      if (colors.some(value => !value.startsWith('ff'))) throw new Error('Reference returned a non-opaque role');
      rows.push([specVersion, hex(seed), style, Number(isDark), contrastLevel, ...colors].join(','));
      // For all nine single-seed styles exposed by FerroUI, 2026 currently
      // falls back exactly to 2025 or 2021. Verify rather than assume that.
      if (specVersion === '2025') {
        const current = new DynamicScheme({sourceColorHct: Hct.fromInt(seed), variant: Variant[style], isDark,
          contrastLevel, specVersion: '2026', platform: 'phone'});
        for (const [index, role] of roles.entries()) {
          if (hex(current[role[0].toLowerCase() + role.slice(1)]) !== colors[index]) {
            throw new Error(`2026 no longer falls back for ${style}/${role}`);
          }
        }
      }
      schemes++;
    }
  }
  fs.mkdirSync(output, {recursive: true});
  const csv = rows.join('\n') + '\n';
  fs.writeFileSync(path.join(output, 'dynamic-color-phone.csv'), csv);
  fs.writeFileSync(path.join(output, 'dynamic-color-provenance.json'), JSON.stringify({
    upstreamRepository: 'https://github.com/material-foundation/material-color-utilities',
    upstreamCommit: COMMIT,
    sourceVerifiedOn: VERIFIED_DATE,
    algorithm: 'Google TypeScript, phone platform, explicit 2021/2025 specs; verified 2026 fallback for all nine supported styles',
    compilerVersion: compiler.version,
    seedCount: seeds.length, styleCount: styles.length, darkVariants: 2, contrasts, specs: ['2021', '2025'],
    schemeCount: schemes, argbCount: schemes * roles.length, roles,
    csvSha256: crypto.createHash('sha256').update(csv).digest('hex'),
    sourceSha256: hashes,
  }, null, 2) + '\n');
  console.log(`Generated ${schemes} schemes and ${schemes * roles.length} independent ARGB values.`);
} finally {
  fs.rmSync(build, {recursive: true, force: true});
}
