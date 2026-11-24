import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function androidNativeWorkspace(workspace, toolkit, adbPath) {
    const dependencies = {
        'ferroui-android': ['src/Android/FerroUI.Android', ['live-inspector']],
        'ferroui-base': ['src/FerroUI.Base'],
        'ferroui-controls': ['src/FerroUI.Controls'],
        'ferroui-mcp': ['src/FerroUI.Mcp'],
        'ferroui-themes-simple': ['src/FerroUI.Themes.Simple'],
        'ferroui-view': ['src/FerroUI.View', ['xaml']],
    };
    const manifest = [
        '[package]', 'name = "android-inspector"', 'version = "0.1.0"', 'edition = "2024"',
        '[lib]', 'path = "lib.rs"', 'crate-type = ["cdylib"]', '[dependencies]',
        ...Object.entries(dependencies).map(([name, [path, features = []]]) =>
            `${name} = { path = ${JSON.stringify(join(toolkit, path))}, features = ${JSON.stringify(features)} }`),
        '',
    ].join('\n');
    const example = await readFile(join(toolkit, 'src/Android/FerroUI.Android/examples/android_inspector.rs'), 'utf8');
    const subtitle = 'TextBlock { text: "Review open cases for your team", foreground: "#405679" }';
    if (!example.includes(subtitle)) throw new Error('Find the Service Review subtitle to replace with the XAML fixture.');
    await mkdir(workspace);
    await Promise.all([
        copyFile(join(toolkit, 'Cargo.lock'), join(workspace, 'Cargo.lock')),
        writeFile(join(workspace, 'Cargo.toml'), manifest),
        writeFile(join(workspace, 'lib.rs'), example.replace(subtitle,
            '{Element::Existing(ferroui_view::use_xaml(include_str!("Message.xaml")))}')),
        writeFile(join(workspace, 'Message.xaml'), '<TextBlock xmlns="https://github.com/ferroui" Text="XAML inbox"/>\n'),
    ]);
    return {
        'ferroui.android.adbPath': adbPath,
        'ferroui.android.apps': [{ id: 'service-review', label: 'Managed Service Review', packageId: 'org.ferroui.androidmanagedreview',
            build: { package: 'android-inspector', toolchain: toolkit, debugger: true, reloadOnSave: true } }],
    };
}
