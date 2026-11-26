import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import * as vscode from 'vscode';
import type { PrototypeHotspot, PrototypeInputResult, PrototypeScene, StageInput, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyPrototypeMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'AccountJourney.xaml', overlayPath = 'AccountSettings.xaml';
    content(await invoke('ferroui_workspace_create', { path, source: `<StackPanel xmlns="https://github.com/ferroui" Background="#EDF3FA" Margin="24" Spacing="12">
        <Button Name="Open" Content="Edit account"/><TextBox Name="Search" Text="Search accounts"/>
    </StackPanel>` }));
    content(await invoke('ferroui_workspace_create', { path: overlayPath, source: `<Border xmlns="https://github.com/ferroui" Background="White" CornerRadius="16" Padding="22">
        <StackPanel Spacing="14"><TextBlock Text="Account settings" FontWeight="SemiBold" FontSize="24"/>
            <TextBox Name="Company" AutomationProperties.Name="Company" Text="Northwind"/>
            <Button Name="Save" Content="Save account"/></StackPanel>
    </Border>` }));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const initial = content<StageModel>(await invoke('designer_stages_open', { session }));
    const add = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: initial.epoch,
        arguments: { action: 'add', name: 'Account settings', sourcePath: overlayPath, width: 400, height: 240, dark: false } }));
    const [base, , modal] = add.stages;
    const hits = content<PrototypeHotspot[]>(await invoke('designer_stage_hotspots', { session, stage: base.id, expected_epoch: base.epoch }));
    const open = hits.find(hit => hit.label === 'Button · Open');
    assert.ok(open);
    const transitions = [{ link: 0, transition: { effect: 'dissolve', duration_ms: 500, easing: 'ease_in_out' } }];
    const board = content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: add.epoch,
        arguments: { action: 'setPrototype', graph: { flows: [{ name: 'Account settings', start: 0 }], links: [
            { source: 0, trigger: open.anchor, name: 'Edit account', action: { kind: 'open_overlay', stage: 2,
                options: { placement: { kind: 'center' }, dim_background: true, dismiss_on_outside: true } } },
        ] }, transitions } }));
    assert.equal(board.manifest.version, 4);
    assert.deepEqual(board.manifest.prototype_transitions, transitions);
    const start = content<PrototypeScene>(await invoke('designer_stage_prototype', { session, expected_epoch: board.epoch,
        arguments: { action: 'start', flow: 'Account settings', reducedMotion: true } }));
    assert.ok(start.frame);
    const input = (scene: PrototypeScene, input: StageInput) => invoke('designer_stage_prototype_input', {
        session, expected_frame: scene.frame, input: { kind: 'keyboard', input },
    });
    const opened = content<PrototypeInputResult>(await input(start, { kind: 'key', key: 'Enter' }));
    assert.ok(opened.scene?.frame);
    assert.equal(opened.scene.animating, false);
    assert.equal(opened.scene.current, modal.id);
    assert.deepEqual(opened.focus, { name: 'Company', role: 'Edit' });
    assert.ok((await input(start, { kind: 'focus' })).isError);
    const image = async (scene: PrototypeScene) => {
        const result = await invoke('designer_stage_prototype_image', { session, expected_frame: scene.frame });
        const png = result.content.find(item => item.type === 'image')?.data;
        assert.ok(png, result.content[0]?.text);
        return createHash('sha256').update(png).digest('hex');
    };
    const before = await image(opened.scene);
    content(await input(opened.scene, { kind: 'text', text: ' Studio' }));
    assert.notEqual(await image(opened.scene), before);
    const back = content<PrototypeInputResult>(await input(opened.scene, { kind: 'key', key: 'Escape' }));
    assert.equal(back.scene?.current, base.id);
    assert.equal(back.focus?.name, 'Edit account');
    assert.equal(content<PrototypeInputResult>(await input(back.scene!, { kind: 'key', key: 'Escape' })).scene, null);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code MCP operated the native prototype modal, captured its composite, and restored the opener');
}
