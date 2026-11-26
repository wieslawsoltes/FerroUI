import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyStageContrast({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ContrastReview.xaml';
    const source = '<StackPanel xmlns="https://github.com/ferroui" Spacing="12"><TextBlock Text="Review account"/><Button Content="Continue"/></StackPanel>';
    content(await invoke('ferroui_workspace_create', { path, source }));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const first = content<StageModel>(await invoke('designer_stages_open', { session }));
    assert.ok(first.availableThemes?.includes('high_contrast'));
    const [control, compare] = first.stages;
    const image = async (stage: StageInfo) => {
        const result = await invoke('designer_stage_image', { session, stage: stage.id, expected_epoch: stage.epoch });
        const data = result.content.find(entry => entry.type === 'image')?.data;
        assert.ok(data, JSON.stringify(result.content[0]));
        return data;
    };
    const original = await image(control);
    const sibling = await image(compare);
    const updated = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: first.epoch, arguments: {
        action: 'update', id: control.id, configuration: {
            ...control.configuration, environment: { ...control.configuration.environment, theme: 'high_contrast' },
        },
    } }));
    const [contrasted, unchanged] = updated.stages;
    assert.equal(contrasted.configuration.environment.theme, 'high_contrast');
    assert.equal(updated.manifest.stages[0].environment.theme, 'high_contrast');
    assert.notEqual(await image(contrasted), original);
    assert.equal(await image(unchanged), sibling);
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    assert.equal(document.getText().trimEnd(), source);
    const labels = new Set([path, `Design · ${path}`, `Stages · ${path}`]);
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => labels.has(tab.label)));
    console.log('High contrast selected the native theme for one stage, preserved its sibling, and persisted in the stage set');
}
