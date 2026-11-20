import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { StageFocus, StageInput, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcp';

export async function verifyStageKeyboard({ invoke, content }: McpFixture): Promise<void> {
    for (const [path, source] of [
        ['StageKeyboard.xaml', `<StackPanel xmlns="https://github.com/ferroui" Spacing="8">
  <Button Name="Apply" Content="Apply"/>
  <TextBox Name="Customer" Width="260" Text="Northwind"/>
</StackPanel>`],
        ['stage_keyboard.rs', `fn stage_view() {
    let _ = rsx! { StackPanel {
        Button { name: "Apply", "Apply" }
        TextBox { name: "Customer", width: 260.0, text: "Northwind" }
    } };
}`],
    ] as const) {
        content(await invoke('ferroui_workspace_create', { path, source }));
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        const document = await vscode.workspace.openTextDocument(uri);
        const version = document.version;
        const first = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.equal(first.keyboardInput, true);
        const [selection, other] = first.stages;
        const target = (stage: StageModel['stages'][number]) => ({ session, stage: stage.id, expected_epoch: stage.epoch });
        const focus = { kind: 'focus' } as const;
        assert.ok((await invoke('designer_stage_input', { ...target(selection), input: focus })).isError);
        const update = async (board: StageModel, stage: StageModel['stages'][number], configuration: object) =>
            content<StageModel>(await invoke('designer_stage_change', { session, expected_epoch: board.epoch, arguments: {
                action: 'update', id: stage.id, configuration: { ...stage.configuration, ...configuration },
            } }));
        const live = await update(first, selection, { mode: 'live' });
        const stage = live.stages[0];
        assert.ok((await invoke('designer_stage_input', { ...target(selection), input: focus })).isError);
        const send = async (input: StageInput) => content<StageFocus>(await invoke('designer_stage_input', { ...target(stage), input }));
        const key = (key: string) => send({ kind: 'key', key });
        assert.equal((await send(focus)).role, 'Button');
        assert.equal((await key('Shift+Tab')).role, 'Edit');
        assert.equal((await key('Tab')).role, 'Button');
        assert.equal((await key('Tab')).role, 'Edit');
        const capture = async (entry: StageModel['stages'][number]) => {
            const result = await invoke('designer_stage_image', target(entry));
            const png = result.content.find(item => item.type === 'image')?.data;
            assert.ok(png, JSON.stringify(result.content[0]));
            return png;
        };
        const [before, unaffected] = await Promise.all([capture(stage), capture(other)]);
        await key('End');
        await key('Space');
        await send({ kind: 'text', text: 'Tokyo 🪴' });
        assert.notEqual(await capture(stage), before);
        assert.equal(await capture(other), unaffected);
        assert.equal(document.version, version);
        assert.equal(document.getText().trimEnd(), source.trimEnd());
        const paused = await update(live, stage, { auto_update: false });
        assert.ok((await invoke('designer_stage_input', { ...target(paused.stages[0]), input: focus })).isError);
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
            .filter(tab => tab.label === path || tab.label === `Design · ${path}` || tab.label === `Stages · ${path}`);
        await vscode.window.tabGroups.close(tabs);
    }
    console.log('VS Code MCP drove scoped native keyboard input in current XAML and RSX stages, preserving the other card and source');
}
