import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function writeNativeMobileApp(workspace, toolkit, { name, kind, platform, example, subtitle, profile = [] }) {
    const sourceFile = kind === 'bin' ? 'main.rs' : 'lib.rs';
    const target = kind === 'bin' ? ['[[bin]]', `name = "${name}"`] : ['[lib]', 'crate-type = ["cdylib"]'];
    const dependencies = {
        ...platform,
        'ferroui-base': ['src/FerroUI.Base'],
        'ferroui-controls': ['src/FerroUI.Controls'],
        'ferroui-mcp': ['src/FerroUI.Mcp'],
        'ferroui-themes-simple': ['src/FerroUI.Themes.Simple'],
        'ferroui-view': ['src/FerroUI.View', ['xaml']],
    };
    const manifest = [
        '[package]', `name = "${name}"`, 'version = "0.1.0"', 'edition = "2024"',
        ...target, `path = "${sourceFile}"`, '[dependencies]',
        ...Object.entries(dependencies).map(([name, [path, features = [], defaults = true]]) =>
            `${name} = { path = ${JSON.stringify(join(toolkit, path))}, features = ${JSON.stringify(features)}, default-features = ${defaults} }`),
        ...profile,
        '',
    ].join('\n');
    const source = await readFile(join(toolkit, example), 'utf8');
    if (!source.includes(subtitle)) throw new Error(`Find the subtitle to replace with the XAML fixture in ${example}.`);
    await mkdir(workspace);
    await Promise.all([
        copyFile(join(toolkit, 'Cargo.lock'), join(workspace, 'Cargo.lock')),
        writeFile(join(workspace, 'Cargo.toml'), manifest),
        writeFile(join(workspace, sourceFile), source.replace(subtitle,
            '{Element::Existing(ferroui_view::use_xaml(include_str!("Message.xaml")))}')),
        writeFile(join(workspace, 'Message.xaml'), '<TextBlock xmlns="https://github.com/ferroui" Text="XAML inbox"/>\n'),
    ]);
}
