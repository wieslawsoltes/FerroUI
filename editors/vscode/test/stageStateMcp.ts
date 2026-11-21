import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { PrototypeHotspot, StageModel, StageStateOptions } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyStageStates({ invoke, content }: McpFixture): Promise<void> {
    const path = 'StateVariants.xaml';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    content(await invoke('ferroui_workspace_create', { path, source: `<StackPanel xmlns="https://github.com/ferroui"
    xmlns:d="http://schemas.microsoft.com/expression/blend/2008" Name="CustomerStatus"
    d:Fixtures='{"default":"Content","fixtures":[{"name":"Content","data":{"Message":"Acme records"}},{"name":"Loading","data":{"Message":"Loading records"}}]}'>
    <TextBlock Name="Status" Text="Customer records" d:Text="Northwind Studio" Opacity="1"/>
    <TextBlock Name="Message" Text="{ReflectionBinding Message}"/>
</StackPanel>` }));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const { revision, nodes } = content<{ revision: number; nodes: { id: number; name?: string }[] }>(await invoke('designer_outline', { session }));
    const owner = nodes.find(node => node.name === 'CustomerStatus')?.id;
    assert.ok(owner);
    const model = { owner, name: 'Records', transitions: [], states: [
        { name: 'Loading', setters: [{ target: 'Status', property: 'Opacity', value: '0.35' }] },
        { name: 'Empty', setters: [] }, { name: 'Content', setters: [] }, { name: 'Error', setters: [] },
    ] };
    content(await invoke('designer_studio_edit', { session, expected_revision: revision, tool: 'designer_state', arguments: { action: 'upsert', model } }));
    const document = await vscode.workspace.openTextDocument(uri);
    assert.ok(await document.save());
    const source = document.getText();
    const board = content<StageModel>(await invoke('designer_stages_open', { session }));
    const stage = board.stages[0];
    const target = { session, stage: stage.id, expected_epoch: stage.epoch };
    const fixtureOptions = content<string[]>(await invoke('designer_stage_fixtures', target));
    assert.deepEqual(fixtureOptions, ['Content', 'Loading']);
    const [group] = content<StageStateOptions[]>(await invoke('designer_stage_states', target));
    assert.deepEqual([group.owner_name, group.group, group.states], ['CustomerStatus', 'Records', ['Loading', 'Empty', 'Content', 'Error']]);
    const hotspots = content<PrototypeHotspot[]>(await invoke('designer_stage_hotspots', target));
    const trigger = hotspots.find(item => item.label === 'TextBlock · Status');
    assert.ok(trigger?.range && trigger.uri);
    const { start, end } = trigger.range;
    assert.equal(document.getText(new vscode.Range(start.line, start.character, end.line, end.character)), 'TextBlock');
    const capture = async (current: StageModel['stages'][number]) => {
        const response = await invoke('designer_stage_image', { session, stage: current.id, expected_epoch: current.epoch });
        const png = response.content.find(item => item.type === 'image')?.data;
        assert.ok(png, JSON.stringify(response.content[0]));
        return png;
    };
    const before = await capture(stage);
    const other = await capture(board.stages[1]);
    const next = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: board.epoch, arguments: {
        action: 'update', id: stage.id, configuration: { ...stage.configuration,
            design_fixture: 'Loading', states: [{ owner: group.owner, group: group.group, state: 'Loading' }] },
    } }));
    assert.equal(next.stages[0].status.state, 'ready');
    const loading = await capture(next.stages[0]);
    assert.notEqual(loading, before);
    const actual = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: next.epoch, arguments: {
        action: 'update', id: stage.id, configuration: { ...next.stages[0].configuration, design_time_data: false },
    } }));
    assert.equal(actual.manifest.stages[0].design_time_data, false);
    assert.notEqual(await capture(actual.stages[0]), loading);
    assert.equal(await capture(actual.stages[1]), other);
    assert.ok((await invoke('designer_stage_states', target)).isError);
    assert.equal(document.getText(), source);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code selected a stage-local fixture and visual state, mapped a prototype trigger, and fenced the previous stage');
}
