import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface ImageSourceEdit {
    path: string;
    source: string;
    asset: string;
    expected: string;
    verifyPreview?: boolean;
}

export async function verifyProjectImageSource(fixture: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const projectAsset = 'Assets/Preview Image.png';
    for (const input of [
        { path: 'ImageSourceReview.xaml', source: '<Image xmlns="https://github.com/ferroui" Source="./Assets/Preview%20Image.png"/>' },
        { path: 'image_source_review.rs', source: `fn view() { rsx! { Image {
            source(::ferroui_view::embedded_image(/* credit 🎨 */ ::core::include_bytes!(r#"./Assets/Preview Image.png"#))),
        } } }` },
    ]) {
        await verifyImageSource(fixture, {
            ...input, asset: projectAsset, expected: input.source.replace('./Assets/', 'Assets/'),
        });
    }

    const directory = 'CargoImageSource';
    const packageRoot = vscode.Uri.joinPath(root, directory);
    const file = (name: string) => vscode.Uri.joinPath(packageRoot, name);
    await vscode.workspace.fs.createDirectory(file('src'));
    try {
        await vscode.workspace.fs.createDirectory(file('Assets'));
        const bytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(root, projectAsset));
        const replacement = Buffer.from(
            'iVBORw0KGgoAAAANSUhEUgAAAAYAAAADCAIAAAA/Y+msAAAAEUlEQVR4nGMwDpqKhhiIEgIALo0T1QA/I98AAAAASUVORK5CYII=', 'base64');
        await Promise.all([
            vscode.workspace.fs.writeFile(file('Cargo.toml'), Buffer.from('[package]\nname = "cargo-image-source-review"\nversion = "0.0.0"\nedition = "2021"\n')),
            vscode.workspace.fs.writeFile(file('src/lib.rs'), Buffer.from('mod card;\n')),
            vscode.workspace.fs.writeFile(file('Assets/Card.png'), bytes),
            vscode.workspace.fs.writeFile(file('Assets/New Photo.png'), replacement),
        ]);
        const source = `use ::ferroui_view::embedded_image as raster;
        fn view() { rsx! { Image {
            source(raster(::std::include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"), "/Assets/", /* credit 🎨 */ r#"Card"#, ".png",
            )))),
        } }; }`;
        await verifyImageSource(fixture, {
            path: `${directory}/src/card.rs`, source, asset: `${directory}/Assets/New Photo.png`,
            expected: source.replace('r#"Card"#', 'r#"New Photo"#'), verifyPreview: true,
        });
    } finally {
        await vscode.workspace.fs.delete(packageRoot, { recursive: true, useTrash: false });
    }
    console.log('Project image source changes preserved an imported RSX helper and the assembled Cargo path, rendered the new asset, and restored the original on Undo');
}

async function verifyImageSource({ invoke, content }: McpFixture, input: ImageSourceEdit): Promise<void> {
    const { path, source, asset, expected, verifyPreview } = input;
    const root = vscode.workspace.workspaceFolders![0].uri;
    const uri = vscode.Uri.joinPath(root, path);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    try {
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const ready = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(ready.status, 'Ready');
        const png = async () => {
            const result = await invoke('designer_preview_image', { session });
            const image = result.content.find(item => item.type === 'image')?.data;
            assert.ok(image, result.content[0]?.text);
            return image;
        };
        const before = verifyPreview ? await png() : undefined;
        const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const image = nodes.find(node => node.canSetImageSource);
        assert.ok(image, 'The accepted image must offer an authored source edit');
        const edit = { session, expected_revision: revision, node: image.id, project_asset: asset };
        content(await invoke('designer_set_image_source', edit));
        assert.equal(document.getText(), expected);
        assert.ok((await invoke('designer_set_image_source', edit)).isError, 'A replaced frame cannot change the source again');
        if (before) assert.notEqual(await png(), before, 'The accepted native frame should show the replacement image');
        const version = document.version;
        content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
        await until(() => document.version !== version);
        assert.equal(document.getText(), source);
        if (before) assert.equal(await png(), before, 'Undo should restore the original native preview');
    } finally {
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.label === `Design · ${path.split('/').at(-1)}` || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
        await vscode.window.tabGroups.close(tabs);
        await vscode.workspace.fs.delete(uri, { useTrash: false });
    }
}
