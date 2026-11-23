import { execFile } from 'node:child_process';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { promisify } from 'node:util';

const exec = promisify(execFile);

/** Build the same example as a standalone app so the extension-host test can save real files. */
export async function browserBuildWorkspace(workspace, toolkit) {
    const demo = join(toolkit, 'src/Browser/FerroUI.Browser/examples/live_inspector');
    await cp(demo, workspace, { recursive: true });
    const script = join(workspace, 'wwwroot/main.js');
    await writeFile(script, (await readFile(script, 'utf8')).replace('./live_inspector.js', './dispatch-check.js'));
    const toolkitManifest = await readFile(join(toolkit, 'Cargo.toml'), 'utf8');
    const version = /^wasm-bindgen = "(=.+)"$/m.exec(toolkitManifest)?.[1];
    if (!version) throw new Error('Read the wasm-bindgen pin from the FerroUI toolchain.');
    const crates = {
        'ferroui-base': ['src/FerroUI.Base'],
        'ferroui-browser': ['src/Browser/FerroUI.Browser', 'inspector-example'],
        'ferroui-controls': ['src/FerroUI.Controls', 'testing'],
        'ferroui-fonts-inter': ['src/FerroUI.Fonts.Inter'],
        'ferroui-themes-simple': ['src/FerroUI.Themes.Simple'],
        'ferroui-view': ['src/FerroUI.View', 'xaml'],
    };
    const quotePath = directory => JSON.stringify(join(toolkit, directory).replaceAll('\\', '/'));
    const dependencies = Object.entries(crates).map(([name, [directory, feature]]) =>
        `${name} = { path = ${quotePath(directory)}${feature ? `, features = ["${feature}"]` : ''} }`).join('\n');
    await writeFile(join(workspace, 'Cargo.toml'), `[package]
name = "dispatch-check"
version = "0.1.0"
edition = "2024"

[workspace]
resolver = "3"
members = ["notes"]

[[bin]]
name = "dispatch-check"
path = "main.rs"

[dependencies]
${dependencies}
wasm-bindgen = ${JSON.stringify(version)}

[patch.crates-io]
skia-bindings = { path = ${quotePath('external/rust-skia-bindings')} }
`);
    await mkdir(join(workspace, 'notes/src'), { recursive: true });
    await writeFile(join(workspace, 'notes/Cargo.toml'), '[package]\nname = "dispatch-notes"\nversion = "0.1.0"\nedition = "2024"\n');
    await writeFile(join(workspace, 'notes/src/lib.rs'), 'pub const STATUS: &str = "Draft notes";\n');
    await cp(join(toolkit, 'Cargo.lock'), join(workspace, 'Cargo.lock'));
    await exec('cargo', ['update', '--workspace', '--offline'], { cwd: workspace, timeout: 45_000 });
}
