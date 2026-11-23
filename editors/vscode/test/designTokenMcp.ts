import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { TokenPlan } from '../src/designTokenProtocol';
import type { PropertyField } from '../src/protocol';
import type { StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyDesignTokens({ invoke, content }: McpFixture): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder, 'customer-design.tokens.json'));
    const source = document.getText();
    const plan = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'import', source, options: {
        mappings: { 'brand.card': { key: 'CustomerCardBrush' }, 'brand.banner': { xamlType: 'RadialGradientBrush' },
            'motion.hover': { transition: { property: 'Visual.Opacity' } } }, pixelsPerRem: 16,
    } }));
    assert.equal(plan.mappings.length, 15);
    assert.deepEqual(plan.issues, []);
    assert.equal(plan.mappings.find(row => row.key === 'CustomerCardBrush')?.value, '#2D6A51');
    assert.equal(plan.mappings.find(row => row.key === 'layout.padding')?.value, '24');
    assert.equal(plan.mappings.find(row => row.key === 'brand.banner')?.xamlType, 'RadialGradientBrush');
    assert.equal(plan.mappings.find(row => row.key === 'depth.card')?.xamlType, 'BoxShadows');
    assert.equal(plan.mappings.find(row => row.key === 'motion.settle')?.value, '0.2,0.8,0.3,1.18');
    assert.equal(plan.mappings.find(row => row.key === 'text.title')?.xamlType, 'ControlTheme');
    assert.equal(plan.mappings.find(row => row.key === 'text.family')?.value, 'Inter, Arial, DejaVu Sans');
    assert.equal(plan.mappings.find(row => row.key === 'text.emphasis')?.value, '600');
    assert.equal(plan.mappings.find(row => row.key === 'motion.hover')?.transition?.property, 'Visual.Opacity');
    assert.match(plan.source, /<DoubleTransition Property="\{x:Static Visual\.OpacityProperty\}"/);
    assert.match(plan.source, /StrokeDashArray" Value="2,1"/);
    assert.equal(document.getText(), source);
    const dictionary = 'CustomerTokens.xaml';
    const create = { path: dictionary, source: plan.source };
    const created = content<{ revision: string }>(await invoke('ferroui_workspace_create', create));
    assert.ok((await invoke('ferroui_workspace_create', create)).isError);
    const view = 'CustomerTokenView.xaml';
    content(await invoke('ferroui_workspace_create', { path: view, source: `<Border xmlns="https://github.com/ferroui"
        Name="Card" Background="{StaticResource CustomerCardBrush}" Width="{StaticResource layout.cardWidth}" BoxShadow="{StaticResource depth.card}" Padding="16" CornerRadius="12"
        Theme="{StaticResource outline.card}" Transitions="{StaticResource motion.hover}">
        <Border.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries>
            <ResourceInclude Source="${dictionary}"/>
        </ResourceDictionary.MergedDictionaries></ResourceDictionary></Border.Resources>
        <Border Name="Banner" Background="{StaticResource brand.banner}" Padding="18" CornerRadius="8">
            <StackPanel Spacing="4">
                <TextBlock Name="Heading" Theme="{StaticResource text.title}" Text="Customer review" Foreground="White"/>
                <TextBlock Name="Subtitle" FontFamily="{StaticResource text.family}" FontWeight="{StaticResource text.emphasis}"
                           Text="Accounts and renewals" Foreground="White"/>
                <Rectangle Name="Divider" Width="180" Height="12" Theme="{StaticResource outline.divider}" />
            </StackPanel>
        </Border>
    </Border>` }));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: view }));
    const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(await invoke('designer_outline', { session }));
    const card = nodes.find(node => node.name === 'Card');
    assert.ok(card);
    assert.ok(nodes.some(node => node.name === 'Banner'));
    const { properties } = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(properties.find(field => field.name === 'Background')?.effective, '#FF2D6A51');
    assert.equal(properties.find(field => field.name === 'Width')?.effective, '320');
    assert.match(properties.find(field => field.name === 'BorderThickness')?.effective ?? '', /^2(?:, ?2){0,3}$/);
    const propertiesOf = async (name: string) => {
        const node = nodes.find(node => node.name === name);
        assert.ok(node);
        const result = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: node.id }));
        return (property: string) => result.properties.find(field => field.name === property)?.effective;
    };
    const heading = await propertiesOf('Heading');
    assert.equal(heading('FontSize'), '20');
    assert.equal(heading('LineHeight'), '24');
    assert.equal(heading('LetterSpacing'), '0.25');
    assert.match(heading('FontFamily') ?? '', /Inter, Arial, DejaVu Sans$/);
    assert.match(heading('FontWeight') ?? '', /^(600|SemiBold)$/);
    const subtitle = await propertiesOf('Subtitle');
    assert.equal(subtitle('FontFamily'), heading('FontFamily'));
    assert.equal(subtitle('FontWeight'), heading('FontWeight'));
    const divider = await propertiesOf('Divider');
    assert.equal(divider('StrokeThickness'), '4');
    assert.equal(divider('StrokeLineCap'), 'Round');
    const trace = content<{ effective: string; contributors: { kind: string; state: string }[] }>(
        await invoke('designer_studio_query', { session, tool: 'designer_property_trace',
            arguments: { node: nodes.find(node => node.name === 'Heading')!.id, property: 'FontSize' } }));
    assert.equal(trace.effective, '20');
    assert.ok(trace.contributors.some(row => row.kind === 'theme' && row.state === 'effective'));
    const stageModel = () => invoke('designer_stages', { session }).then(content<StageModel>);
    const firstBoard = content<StageModel>(await invoke('designer_stages_open', { session }));
    const board = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: firstBoard.epoch,
        arguments: { action: 'add', name: 'Independent', width: 360, height: 280, dark: false, sourcePath: 'SettingsView.xaml' } }));
    const stage = board.stages[0];
    const independent = board.stages.at(-1)!;
    const image = async (stage: StageInfo) => {
        const result = await invoke('designer_stage_image', { session, stage: stage.id, expected_epoch: stage.epoch });
        const data = result.content.find(item => item.type === 'image')?.data;
        assert.ok(data, JSON.stringify(result.content[0]));
        return data;
    };
    assert.equal(stage.status.state, 'ready', JSON.stringify(stage.status));
    const originalFrame = await image(stage);
    const independentFrame = await image(independent);
    const draft = plan.source.replace('x:Key="CustomerCardBrush" Color="#2D6A51"', 'x:Key="CustomerCardBrush" Color="#1B2C47"');
    assert.notEqual(draft, plan.source);
    content(await invoke('ferroui_workspace_edit', { files: [{ path: dictionary, expected_revision: created.revision, source: draft }] }));
    const updated = content<{ revision: number }>(await invoke('designer_wait', { session }));
    assert.ok(updated.revision > revision);
    const live = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(live.properties.find(field => field.name === 'Background')?.effective, '#FF1B2C47');
    const refreshed = await stageModel();
    assert.equal(refreshed.stages[0].status.state, 'ready');
    assert.ok(refreshed.stages[0].epoch > stage.epoch);
    assert.notEqual(await image(refreshed.stages[0]), originalFrame);
    assert.equal(refreshed.stages.at(-1)?.epoch, independent.epoch);
    assert.equal(await image(independent), independentFrame);
    assert.ok((await invoke('designer_stage_hotspots', { session, stage: stage.id, expected_epoch: stage.epoch })).isError);
    const saved = vscode.Uri.joinPath(folder, dictionary);
    assert.equal(Buffer.from(await vscode.workspace.fs.readFile(saved)).toString(), plan.source);
    const authored = content<{ revision: string }>(await invoke('ferroui_workspace_read', { path: dictionary }));
    content(await invoke('ferroui_workspace_file', { path: dictionary, action: 'undo', expected_revision: authored.revision }));
    content(await invoke('designer_wait', { session }));
    const restored = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: card.id }));
    assert.equal(restored.properties.find(field => field.name === 'Background')?.effective, '#FF2D6A51');
    assert.equal(await image((await stageModel()).stages[0]), originalFrame);
    const valid = content<{ revision: string }>(await invoke('ferroui_workspace_read', { path: dictionary }));
    content(await invoke('ferroui_workspace_edit', { files: [{ path: dictionary, expected_revision: valid.revision, source: '<ResourceDictionary' }] }));
    const failed = (await stageModel()).stages[0];
    assert.equal(failed.status.state, 'failed', JSON.stringify(failed.status));
    assert.match(failed.status.message ?? '', /included XAML/);
    assert.equal(await image(failed), originalFrame);
    const invalid = content<{ revision: string }>(await invoke('ferroui_workspace_read', { path: dictionary }));
    content(await invoke('ferroui_workspace_file', { path: dictionary, action: 'undo', expected_revision: invalid.revision }));
    const recovered = (await stageModel()).stages[0];
    assert.equal(recovered.status.state, 'ready');
    assert.equal(await image(recovered), originalFrame);
    const exported = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'export', source: plan.source, options: {
        mappings: { CustomerCardBrush: { token: 'brand.card' }, 'layout.cardWidth': { kind: 'dimension' } },
    } }));
    assert.equal(exported.issues.length, 0);
    const exportedJson = JSON.parse(exported.source);
    assert.equal(exportedJson.brand.card.$extensions['com.github.ferroui'].key, 'CustomerCardBrush');
    assert.deepEqual(exportedJson.layout.cardWidth.$value, { value: 320, unit: 'px' });
    assert.deepEqual(exportedJson.motion.fade.$value, { value: 250, unit: 'ms' });
    assert.deepEqual(exportedJson.motion.settle.$value, [0.2, 0.8, 0.3, 1.18]);
    assert.deepEqual(exportedJson.motion.hover.$value, {
        duration: { value: 250, unit: 'ms' }, delay: { value: 30, unit: 'ms' }, timingFunction: [0.2, 0.8, 0.3, 1.18],
    });
    assert.equal(exportedJson.motion.hover.$extensions['com.github.ferroui'].transition.property, 'Visual.Opacity');
    assert.deepEqual(exportedJson.outline.divider.$value.style.dashArray, [{ value: 8, unit: 'px' }, { value: 4, unit: 'px' }]);
    assert.equal(exportedJson['depth'].card.$value[1].inset, true);
    assert.equal(exportedJson.brand.banner.$extensions['com.github.ferroui'].xamlType, 'RadialGradientBrush');
    assert.deepEqual(exportedJson.text.title.$value, {
        fontFamily: ['Inter', 'Arial', 'DejaVu Sans'], fontSize: { value: 20, unit: 'px' },
        fontWeight: 600, letterSpacing: { value: 0.25, unit: 'px' }, lineHeight: 1.2,
    });
    const roundtrip = content<TokenPlan>(await invoke('designer_design_tokens', { direction: 'import', source: exported.source }));
    assert.equal(roundtrip.issues.length, 0);
    assert.equal(roundtrip.mappings.find(row => row.key === 'CustomerCardBrush')?.value, '#2D6A51');
    assert.equal(roundtrip.mappings.find(row => row.key === 'brand.banner')?.xamlType, 'RadialGradientBrush');
    const labels = [view, `Design · ${view}`, `Stages · ${view}`];
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => labels.includes(tab.label));
    await vscode.window.tabGroups.close(tabs);
    await vscode.commands.executeCommand('ferroui.designTokens.import', document.uri);
    const reviewLabel = 'Tokens · customer-design.tokens.json';
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === reviewLabel)));
    const generated = vscode.workspace.textDocuments.find(document => document.uri.scheme === 'ferroui-token-review' && document.uri.path.endsWith('/DesignTokens.xaml'));
    assert.match(generated?.getText() ?? '', /<LinearGradientBrush/);
    assert.match(generated?.getText() ?? '', /<KeySpline/);
    assert.match(generated?.getText() ?? '', /<ControlTheme x:Key="text.title" TargetType="TextBlock">/);
    assert.match(generated?.getText() ?? '', /<ControlTheme x:Key="outline.divider" TargetType="Shape">/);
    const reviewTabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === reviewLabel || tab.label === 'DesignTokens.xaml');
    await vscode.window.tabGroups.close(reviewTabs);
    console.log('VS Code rendered DTCG resources and typography in the canvas and Stages, retained good frames through an invalid include, and exported the reverse mapping');
}
