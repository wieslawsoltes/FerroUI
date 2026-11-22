import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import * as vscode from 'vscode';
import type { StageContact, StageConfiguration, StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyStagePointer({ invoke, content }: McpFixture): Promise<void> {
    for (const [path, source, profile] of [
        ['StagePointer.xaml', `<Slider xmlns="https://github.com/ferroui" Name="Volume" Width="280" Height="56"
  HorizontalAlignment="Center" VerticalAlignment="Center" Minimum="0" Maximum="100" Value="50"/>`, 'touch'],
        ['stage_pointer.rs', `fn volume() { let _ = rsx! { Slider { name: "Volume", width: 280.0, height: 56.0,
  horizontal_alignment: "Center", vertical_alignment: "Center", minimum: 0.0, maximum: 100.0, value: 50.0 } }; }`, 'pen'],
    ] as const) {
        content(await invoke('ferroui_workspace_create', { path, source }));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
        const version = document.version;
        const board = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.equal(board.pointerInput, true);
        const target = (stage: StageInfo) => ({ session, stage: stage.id, expected_epoch: stage.epoch });
        const change = async ({ epoch }: StageModel, stage: StageInfo, settings: Partial<StageConfiguration>) =>
            content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: epoch, arguments: {
                action: 'update', id: stage.id, configuration: { ...stage.configuration, ...settings },
            } }));
        const picture = async (stage: StageInfo) => {
            const result = await invoke('designer_stage_image', target(stage));
            const image = result.content.find(item => item.type === 'image')?.data;
            assert.ok(image, JSON.stringify(result.content[0]));
            return createHash('sha256').update(image).digest('hex');
        };
        // The Fluent slider thumb sits above the middle of its 56-DIP control
        // because its template reserves room for the focus and tick visuals.
        const down = { contact: 44, phase: 'down', device: 'mouse', x: 0.5, y: 0.48, pressure: 0.8, tiltX: 15 } as const;
        const send = (stage: StageInfo, input: StageContact) => invoke('designer_stage_pointer', { ...target(stage), input });
        const [old, other] = board.stages;
        assert.ok((await send(old, down)).isError);
        const live = await change(board, old, { mode: 'live', pointer_input: profile });
        const stage = live.stages[0];
        assert.equal(live.manifest.stages[0].pointer_input, profile);
        assert.ok((await send(old, down)).isError);
        assert.ok((await send(stage, { ...down, phase: 'move' })).isError);
        const [before, sibling] = await Promise.all([picture(stage), picture(other)]);
        content(await send(stage, down));
        content(await send(stage, { ...down, phase: 'move', x: 1.2 }));
        content(await send(stage, { ...down, phase: 'up', x: 1.2 }));
        assert.notEqual(await picture(stage), before, `${path} ${profile} drag should move the thumb`);
        assert.equal(await picture(other), sibling);
        assert.ok((await send(stage, { ...down, phase: 'up' })).isError);
        content(await send(stage, down));
        content(await send(stage, { ...down, phase: 'cancel' }));
        assert.ok((await send(stage, { ...down, phase: 'up' })).isError);
        const paused = await change(live, stage, { auto_update: false });
        assert.ok((await send(paused.stages[0], down)).isError);
        assert.equal(document.version, version);
        assert.equal(document.getText().trimEnd(), source.trimEnd());
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
            .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
        await vscode.window.tabGroups.close(tabs);
    }
    console.log('VS Code MCP dragged native touch and pen sliders in XAML and RSX, cancelled contacts, and preserved the other card and source');
}
