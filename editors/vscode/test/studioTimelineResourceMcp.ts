import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { TimelinePosition, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

const scene = `<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Background="#F4F6FB">
  <Canvas.Resources>
    <Animation x:Key="CardShadow" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Forward">
      <KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame>
    </Animation>
  </Canvas.Resources>
  <Border Name="Card" Canvas.Left="30" Canvas.Top="30" Width="180" Height="90" Background="White"/>
</Canvas>`;

export async function verifyTimelineResource({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const original = document.getText();
    const source = async (text: string) => content(await invoke('designer_source', { session, expected_version: document.version, source: text }));
    const wait = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const resource = async () => content<{ programs: TimelineProgram[] }>(await invoke('designer_studio_query', {
        session, tool: 'designer_programs', arguments: { include_source: false },
    })).programs.find(program => program.name === 'CardShadow')!;
    const image = async () => {
        const snapshot = await invoke('designer_preview_image', { session });
        assert.equal(snapshot.isError, false);
        return snapshot.content.find(item => item.type === 'image')!.data;
    };
    await source(scene);
    let frame = await wait();
    let program = await resource();
    assert.equal(program.timeline, null);
    assert.equal(program.preview?.available, false);
    assert.equal(program.preview?.native, true);
    const before = await image();
    const present = async (action: string, fields: object = {}) => {
        content(await invoke('designer_studio_present', { session, expected_revision: frame.revision,
            tool: 'designer_program', arguments: { action, program: program.id, ...fields } }));
        frame = await wait();
    };
    const targets = [frame.nodes.find(node => node.name === 'Card')!.id];
    await present('preview_targets', { targets });
    program = await resource();
    assert.equal(program.preview?.explicit_targets, true);
    assert.equal(program.preview?.total_duration, 2);
    assert.deepEqual(program.targets, targets);
    const seek = async (time: number) => {
        const position = content<TimelinePosition>(await invoke('designer_timeline_control', {
            session, expected_revision: frame.revision, arguments: { action: 'seek', program: program.id, time },
        }));
        frame = await wait();
        return position;
    };
    assert.equal((await seek(1)).time, 1);
    const sampled = await image();
    assert.notEqual(sampled, before);
    await source(scene.replace('<Canvas.Resources>', '<!-- resource preview -->\n  <Canvas.Resources>'));
    frame = await wait();
    program = await resource();
    assert.deepEqual(program.targets, [frame.nodes.find(node => node.name === 'Card')!.id]);
    assert.equal(await image(), sampled);
    content(await invoke('designer_timeline_control', { session, expected_revision: frame.revision,
        arguments: { action: 'stop', program: program.id } }));
    frame = await wait();
    assert.equal(await image(), before);
    await present('preview_targets', { targets: null });
    assert.equal((await resource()).preview?.available, false);
    for (const expected of [scene, original]) {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        frame = await wait();
        assert.equal(document.getText(), expected);
    }
    console.log('VS Code assigned and retained a resource clip target, scrubbed a native shadow and restored the base frame without authoring keys');
}
