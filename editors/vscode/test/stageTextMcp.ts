import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyStageText({ invoke, content }: McpFixture): Promise<void> {
    const views = [
        ['StageText.xaml', '<TextBlock xmlns="https://github.com/ferroui" FontSize="18" Text="Account review 日本語"/>'],
        ['stage_text.rs', 'fn view() { let _ = rsx! { TextBlock { text: "Account review 日本語", font_size: 18.0 } }; }'],
    ];
    for (const [path, source] of views) {
        content(await invoke('ferroui_workspace_create', { path, source }));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
        const version = document.version;
        const first = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.equal(first.textEnvironment, true);
        const [card, neighbor] = first.stages;
        const image = async (stage: StageInfo) => {
            const result = await invoke('designer_stage_image', { session, stage: stage.id, expected_epoch: stage.epoch });
            const data = result.content.find(entry => entry.type === 'image')?.data;
            assert.ok(data, result.content[0]?.text);
            return data;
        };
        const update = (board: StageModel, stage: StageInfo, configuration = stage.configuration) =>
            invoke('designer_stage_change', { session, expected_epoch: board.epoch,
                arguments: { action: 'update', id: stage.id, configuration } });

        const neighborImage = await image(neighbor);
        const fitted = content<StageModel>(await update(first, card, { ...card.configuration, layout: 'size_that_fits' }));
        const normal = fitted.stages[0];
        const normalImage = await image(normal);
        const text = { locale: 'ja-JP', scale_percent: 200 };
        const enlarged = content<StageModel>(await update(fitted, normal, { ...normal.configuration, text }));
        const large = enlarged.stages[0];
        assert.ok(large.layout_size.height > normal.layout_size.height, JSON.stringify(large.layout_size));
        assert.deepEqual(enlarged.manifest.stages[0].text, text);
        const enlargedImage = await image(large);
        assert.notEqual(enlargedImage, normalImage);
        assert.equal(await image(enlarged.stages[1]), neighborImage);
        assert.ok((await update(enlarged, large, { ...large.configuration, text: { scale_percent: 301 } })).isError);
        assert.equal(await image(large), enlargedImage);

        const boardLabel = `Stages · ${path}`;
        await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === boardLabel));
        const restored = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.deepEqual(restored.stages[0].configuration.text, text);
        assert.deepEqual(restored.stages[0].layout_size, large.layout_size);
        assert.equal(await image(restored.stages[0]), enlargedImage);
        assert.equal(document.version, version);
        assert.equal(document.getText().trimEnd(), source);

        const labels = new Set([path, `Design · ${path}`, boardLabel]);
        await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => labels.has(tab.label)));
    }
    console.log('VS Code MCP verified stage locale and native text reflow in XAML and RSX, with independent sibling frames');
}
