import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyProjectImageSource({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const projectAsset = 'Assets/Preview Image.png';
    for (const fixture of [
        { path: 'ImageSourceReview.xaml', source: '<Image xmlns="https://github.com/ferroui" Source="./Assets/Preview%20Image.png"/>' },
        { path: 'image_source_review.rs', source: `fn view() { rsx! { Image {
            source: ::ferroui_view::embedded_image(/* credit 🎨 */ ::core::include_bytes!(r#"./Assets/Preview Image.png"#)),
        } } }` },
    ]) {
        const uri = vscode.Uri.joinPath(root, fixture.path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(fixture.source));
        const document = await vscode.workspace.openTextDocument(uri);
        try {
            const { session } = content<{ session: number }>(await invoke('designer_open', { path: fixture.path }));
            content(await invoke('designer_wait', { session }));
            const { revision, nodes } = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
            const image = nodes.find(node => node.canSetImageSource);
            assert.ok(image, 'The accepted image must offer an authored source edit');
            const edit = { session, expected_revision: revision, node: image.id, project_asset: projectAsset };
            content(await invoke('designer_set_image_source', edit));
            assert.equal(document.getText(), fixture.source.replace('./Assets/', 'Assets/'));
            assert.ok((await invoke('designer_set_image_source', edit)).isError, 'A replaced frame cannot change the source again');
            const version = document.version;
            content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
            await until(() => document.version !== version);
            assert.equal(document.getText(), fixture.source);
        } finally {
            const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
                tab.label === `Design · ${fixture.path}` || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
            await vscode.window.tabGroups.close(tabs);
            await vscode.workspace.fs.delete(uri, { useTrash: false });
        }
    }
    console.log('Project image source changes preserved authored XAML and RSX and one-step Undo');
}
