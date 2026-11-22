import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { FileMovePlan } from '../src/fileMoveProtocol';
import type { ProjectAsset } from '../src/protocol';
import type { StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyPublishedAssetMove({ invoke, content }: McpFixture): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const directory = 'Published Asset Move';
    const root = vscode.Uri.joinPath(workspace, directory);
    const file = (relative: string) => vscode.Uri.joinPath(root, relative);
    const source = 'Assets/Logo Ł.png';
    const destination = 'Assets/Archive/Logo #2.png';
    const address = 'ferres://published_asset_move/Assets/Logo%20%C5%81.png';
    const nextAddress = 'ferres://published_asset_move/Assets/Archive/Logo%20%232.png';
    const xaml = `<Image xmlns="https://github.com/ferroui" Source="${address}" Width="80" Height="80" />`;
    const rust = `fn view() { rsx! { Image { source(::ferroui_view::asset_image(/* original 🎨 */ r#"${address}"#)) } }; }\r\n`;
    const bytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(workspace, 'Assets', 'Preview Image.png'));
    const write = (name: string, text: string) => vscode.workspace.fs.writeFile(file(name), Buffer.from(text));
    await vscode.workspace.fs.createDirectory(file('Assets/Archive'));
    await Promise.all([
        write('Cargo.toml', '[package]\nname="published-asset-move"\nversion="0.1.0"\nedition="2021"\n[[bin]]\nname="published-asset-move"\npath="main.rs"\n'),
        write('main.rs', 'fn main() {}\n'), write('View.xaml', xaml), write('view.rs', rust),
        write('build.rs', buildScript(file('Cargo.toml').fsPath, source)), vscode.workspace.fs.writeFile(file(source), bytes),
    ]);
    const editor = await vscode.window.showTextDocument(file('View.xaml'), { viewColumn: vscode.ViewColumn.One, preview: false });
    try {
        await vscode.commands.executeCommand('ferroui.buildProjectSchema');
        const design = content<{ session: number }>(await invoke('designer_open', { path: `${directory}/View.xaml` }));
        const args = { session: design.session };
        const assets = async () => content<{ assets: ProjectAsset[] }>(await invoke('designer_assets', args)).assets;
        await until(async () => (await assets()).some(asset => asset.id === address && asset.movable));
        const before = content<{ data: string }>(await invoke('designer_preview_image', args)).data;
        const board = async (open = false) => content<StageModel>(await invoke(open ? 'designer_stages_open' : 'designer_stages', args));
        const stageImage = async (stage: StageInfo) => (await invoke('designer_stage_image', {
            ...args, stage: stage.id, expected_epoch: stage.epoch,
        })).content.find(item => item.type === 'image')?.data;
        const initial = (await board(true)).stages[0];
        const stageBefore = await stageImage(initial);
        assert.ok(stageBefore);
        let stageEpoch = initial.epoch;
        const request = { ...args, path: address, destination: `${directory}/${destination}` };
        const plan = content<FileMovePlan>(await invoke('ferroui_project_plan_asset_move', request));
        assert.deepEqual(plan.publishedAssets, [{ from: address, to: nextAddress }]);
        assert.equal(plan.documents.length, 3);
        assert.ok(plan.documents.find(document => document.edit.uri.endsWith('/build.rs'))?.after.includes(`.asset(r#"/${destination}"#)`));
        content(await invoke('ferroui_project_move_asset', { ...request, review: plan.review }));
        const rustDocument = await vscode.workspace.openTextDocument(file('view.rs'));
        assert.equal(rustDocument.getText(), rust.replace(address, nextAddress));

        const expect = async (location: string, uri: string) => {
            assert.deepEqual(await vscode.workspace.fs.readFile(file(location)), bytes);
            assert.equal(editor.document.getText(), xaml.replace(address, uri));
            await until(async () => (await assets()).some(asset => asset.id === uri));
            const position = editor.document.positionAt(editor.document.getText().indexOf(uri) + 16);
            const physical = await realpath(file(location).fsPath);
            await until(async () => {
                const resolved = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', editor.document.uri, position);
                return resolved?.[0]?.uri.fsPath === physical;
            });
            assert.equal(content<{ data: string }>(await invoke('designer_preview_image', args)).data, before);
            await until(async () => {
                const stage = (await board()).stages[0];
                if (stage.epoch === stageEpoch || stage.status.state !== 'ready') return false;
                assert.equal(await stageImage(stage), stageBefore);
                stageEpoch = stage.epoch;
                return true;
            });
        };
        await expect(destination, nextAddress);
        for (const [command, location, uri] of [['undo', source, address], ['redo', destination, nextAddress], ['undo', source, address]]) {
            await vscode.window.showTextDocument(editor.document, { viewColumn: editor.viewColumn, preview: false });
            await vscode.commands.executeCommand(command);
            await until(async () => await vscode.workspace.fs.stat(file(location)).then(() => true, () => false));
            await expect(location, uri);
        }
        console.log('VS Code MCP moved a published asset, updated the Cargo declaration and XAML/RSX, retained pixels and source navigation, and replayed grouped history');
    } finally {
        const owned = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.input instanceof vscode.TabInputText
            && tab.input.uri.path.startsWith(`${root.path}/`));
        for (const tab of owned) {
            if (!tab.isDirty || !(tab.input instanceof vscode.TabInputText)) continue;
            await vscode.window.showTextDocument(tab.input.uri, { preview: false });
            await vscode.commands.executeCommand('workbench.action.files.revert');
        }
        if (owned.length) await vscode.window.tabGroups.close(owned);
        await vscode.workspace.fs.delete(root, { recursive: true, useTrash: false });
    }
}

function buildScript(manifest: string, source: string): string {
    const declarations = [
        '    let _ = Build.embed_assets(&["Assets"])',
        `        .asset(r#"/${source}"#);`,
    ];
    const location = (line: number, method: string, value: string) => ({ value, rust_file: 'build.rs',
        line: line + 4, column: declarations[line].indexOf(method) + 1 });
    const receipt = { version: 1, manifest, assembly: 'published_asset_move', crate_name: 'published_asset_move',
        assets: [{ key: `/${source}`, source }], publication: {
            directories: [location(0, 'embed_assets', 'Assets')], registrations: [location(1, 'asset', `/${source}`)],
        } };
    const schema = { format: 2, name: receipt.assembly, crate_name: receipt.crate_name, types: [] };
    return ['struct Build;', 'impl Build { fn embed_assets(self, _: &[&str]) -> Self { self } fn asset(self, _: &str) -> Self { self } }',
        'fn main() {', ...declarations,
        '    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());',
        `    std::fs::write(out.join("assets.xamlmeta"), r#"${JSON.stringify(schema)}"#).unwrap();`,
        `    std::fs::write(out.join("packaged-assets.json"), r#"${JSON.stringify(receipt)}"#).unwrap();`, '}', ''].join('\n');
}
