import { strict as assert } from 'node:assert';
import { Buffer } from 'node:buffer';
import * as vscode from 'vscode';
import type { PrototypeHotspot, StageInfo, StageModel } from '../src/stagesProtocol';
import type { McpFixture } from './mcpFixture';

export async function verifyStageOrder({ invoke, content }: McpFixture): Promise<void> {
    for (const [path, source] of [
        ['ArrangeStages.xaml', '<Button xmlns="https://github.com/ferroui" Name="Details" Content="Details"/>'],
        ['arrange_stages.rs', 'fn view() { rsx! { Button { name: "Details", "Details" } }; }'],
    ] as const) {
        await invoke('ferroui_workspace_create', { path, source }).then(content);
        const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        const document = await vscode.workspace.openTextDocument(uri);
        const buffer = document.getText();
        const initial = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.equal(initial.stageArrangement, true);
        const change = (board: StageModel, args: object) => invoke('designer_stage_change', {
            session, expected_epoch: board.epoch, arguments: args,
        });
        const target = ({ id, epoch }: StageInfo) => ({ id, epoch });
        const images = async (stage: StageInfo) => {
            const reply = await invoke('designer_stage_image', { session, stage: stage.id, expected_epoch: stage.epoch });
            const png = reply.content.find(item => item.type === 'image')?.data;
            assert.ok(png, reply.content[0]?.text);
            return png;
        };
        const name = '🪴'.repeat(32);
        const configured = content<StageModel>(await change(initial, {
            action: 'update', id: initial.stages[0].id,
            configuration: { ...initial.stages[0].configuration, name, layout: 'fixed_width', pinned: true },
        }));
        const [base, peer] = configured.stages;
        const choices = content<PrototypeHotspot[]>(await invoke('designer_stage_hotspots', {
            session, stage: base.id, expected_epoch: base.epoch,
        }));
        const trigger = choices.find(option => option.label === 'Button · Details');
        assert.ok(trigger);
        const linked = content<StageModel>(await change(configured, { action: 'setPrototype', graph: {
            flows: [{ name: 'Details', start: 0 }],
            links: [{ name: 'Open details', source: 0, trigger: trigger.anchor, action: { kind: 'navigate', stage: 1 } }],
        } }));
        const before = await images(linked.stages[0]);
        const copied = content<StageModel>(await change(linked, { action: 'duplicate', target: target(linked.stages[0]) }));
        const copy = copied.stages[2];
        assert.ok(copy.configuration.name.endsWith(' copy') && Buffer.byteLength(copy.configuration.name) <= 128);
        assert.deepEqual(copy.configuration, { ...base.configuration, name: copy.configuration.name });
        assert.notEqual(copy.id, base.id);
        const moving = { action: 'move', target: target(copied.stages[0]), index: 2 };
        const moved = content<StageModel>(await change(copied, moving));
        assert.deepEqual(moved.stages.map(stage => stage.id), [peer.id, copy.id, base.id]);
        assert.deepEqual(target(moved.stages[2]), target(copied.stages[0]));
        assert.equal(await images(moved.stages[2]), before);
        assert.equal(moved.manifest.prototype?.flows[0].start, 2);
        assert.equal(moved.manifest.prototype?.links[0].source, 2);
        assert.deepEqual(moved.manifest.prototype?.links[0].action, { kind: 'navigate', stage: 0 });
        assert.ok((await change(copied, moving)).isError);
        assert.ok((await change(moved, { ...moving, target: { id: base.id, epoch: base.epoch + 1 } })).isError);
        assert.ok((await change(moved, { ...moving, index: 20 })).isError);
        const tabs = () => vscode.window.tabGroups.all.flatMap(group => group.tabs);
        await vscode.window.tabGroups.close(tabs().filter(tab => tab.label === `Stages · ${path}`));
        const reopened = content<StageModel>(await invoke('designer_stages_open', { session }));
        assert.deepEqual(reopened.manifest, moved.manifest);
        assert.equal(document.getText(), buffer);
        await vscode.window.tabGroups.close(tabs().filter(tab => [path, `Design · ${path}`, `Stages · ${path}`].includes(tab.label)));
    }
    console.log('VS Code MCP copied and moved XAML and RSX stages, kept accepted pixels and prototype links, and restored the board order');
}
