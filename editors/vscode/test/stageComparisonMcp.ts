import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { StageComparison, StageComparisonPlan, StageModel, StageStateOptions } from '../src/stagesProtocol';
import type { McpFixture } from './mcpFixture';

export async function verifyStageComparisons({ invoke, content }: McpFixture): Promise<void> {
    const fixtures = [
        ['CompareStages.xaml', `<StackPanel xmlns="https://github.com/ferroui"
    xmlns:d="http://schemas.microsoft.com/expression/blend/2008" Name="Review"
    d:Fixtures='{"default":"Ready","fixtures":[{"name":"Ready","data":{"Title":"Review"}},{"name":"Draft","data":{"Title":"Draft"}}]}'>
    <TextBlock Name="Title" Text="{ReflectionBinding Title}" Opacity="1"/>
    <Button Content="Save review"/>
</StackPanel>`],
        ['compare_stages.rs', `fn review() {
    let _ = rsx! { StackPanel {
        TextBlock { name: "Title", text: "Review" }
        Button { "Save review" }
    } };
}`],
    ] as const;
    for (const [path, source] of fixtures) {
        const xaml = path.endsWith('.xaml');
        content(await invoke('ferroui_workspace_create', { path, source }));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        if (xaml) {
            const { revision, nodes } = content<{ revision: number; nodes: { id: number; name?: string }[] }>(
                await invoke('designer_outline', { session }));
            const owner = nodes.find(node => node.name === 'Review')?.id;
            assert.ok(owner);
            content(await invoke('designer_studio_edit', { session, expected_revision: revision, tool: 'designer_state', arguments: {
                action: 'upsert', model: { owner, name: 'ReviewStates', transitions: [], states: [
                    { name: 'Ready', setters: [] }, { name: 'Draft', setters: [{ target: 'Title', property: 'Opacity', value: '0.5' }] },
                ] },
            } }));
        }
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        const document = await vscode.workspace.openTextDocument(uri);
        const before = document.getText();
        const initial = content<StageModel>(await invoke('designer_stages_open', { session }));
        const updated = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: initial.epoch, arguments: {
            action: 'update', id: initial.stages[0].id, configuration: {
                ...initial.stages[0].configuration,
                environment: { ...initial.stages[0].configuration.environment, width: 260, height: 180 },
                layout: 'fixed_width', text: { locale: 'en-GB', scale_percent: 120 },
                ...(xaml ? { design_fixture: 'Draft' } : {}),
            },
        } }));
        const base = updated.stages[0];
        const target = { session, stage: base.id, expected_epoch: base.epoch };
        const plan = async (comparison: StageComparison) =>
            content<StageComparisonPlan>(await invoke('designer_stage_comparison', { ...target, comparison }));
        const sizes = await plan({ kind: 'preset', preset: 'sizes' });
        assert.deepEqual(sizes.configurations.map(({ name, environment }) => [name, environment.width, environment.height]),
            [['Phone', 390, 844], ['Tablet', 768, 1024], ['Desktop', 1280, 800]]);
        assert.equal(sizes.epoch, updated.epoch);
        const states = xaml ? content<StageStateOptions[]>(await invoke('designer_stage_states', target)).map(({ owner, group, states }) =>
            ({ owner, group, states })) : undefined;
        const comparison: StageComparison = { kind: 'matrix', matrix: {
            sizes: [{ name: 'Compact review', width: 260, height: 180 }],
            themes: ['light', 'dark'], state_groups: states,
        } };
        const draft = await plan(comparison);
        assert.equal(draft.configurations.length, xaml ? 4 : 2);
        assert.ok(draft.configurations.every(row => row.source_uri === base.configuration.source_uri && row.layout === 'fixed_width'
            && row.text?.scale_percent === 120 && row.design_fixture === (xaml ? 'Draft' : undefined)));
        const overflow: StageComparison = { kind: 'matrix', matrix: {
            sizes: Array.from({ length: 8 }, (_, index) => ({ name: `Wide ${index}`, width: 300 + index, height: 220 })),
            themes: ['light', 'dark'], design_time_data: [true, false],
        } };
        assert.ok((await invoke('designer_stage_comparison', { ...target, comparison: overflow })).isError);
        assert.deepEqual(content<StageModel>(await invoke('designer_stages', { session })).manifest, updated.manifest);
        const apply = { session, expected_epoch: draft.epoch, arguments: { action: 'compare',
            base: { id: base.id, epoch: base.epoch }, comparison } };
        const accepted = content<StageModel>(await invoke('designer_stage_change', apply));
        assert.deepEqual(accepted.manifest.stages.slice(updated.stages.length), draft.configurations);
        assert.deepEqual(accepted.manifest.stages.slice(0, updated.stages.length), updated.manifest.stages);
        assert.ok(accepted.stages.every(stage => stage.status.state === 'ready'), JSON.stringify(accepted.stages));
        assert.ok((await invoke('designer_stage_change', apply)).isError);
        assert.ok((await invoke('designer_stage_change', { ...apply, expected_epoch: accepted.epoch,
            arguments: { ...apply.arguments, base: { id: base.id, epoch: base.epoch + 1 } } })).isError);
        assert.equal(document.getText(), before);
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs);
        await vscode.window.tabGroups.close(tabs.filter(tab => tab.label === `Stages · ${path}`));
        const reopened = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.deepEqual(reopened.manifest, accepted.manifest);
        await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs)
            .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`));
    }
    console.log('VS Code MCP planned shared XAML and RSX stage matrices, checked epochs and capacity, and restored every accepted card');
}
