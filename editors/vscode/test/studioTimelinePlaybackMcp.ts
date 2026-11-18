import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { TimelinePosition, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

const scene = `<Canvas xmlns="https://github.com/ferroui">
  <Canvas.Styles>
    <Style Selector="Button#Lead"><Style.Animations>
      <Animation Duration="0:0:2" IterationCount="3" PlaybackDirection="Alternate" FillMode="Both">
        <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0"/></KeyFrame>
        <KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame>
      </Animation>
    </Style.Animations></Style>
    <Style Selector="Button#Peer"><Style.Animations>
      <Animation Duration="0:0:1" FillMode="Forward">
        <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0.2"/></KeyFrame>
        <KeyFrame Cue="100%"><Setter Property="Opacity" Value="0.6"/></KeyFrame>
      </Animation>
    </Style.Animations></Style>
  </Canvas.Styles>
  <Button Name="Lead" Content="First"/>
  <Button Name="Peer" Content="Second" Canvas.Top="56" Opacity="0.9"/>
</Canvas>`;

export async function verifyTimelinePlayback({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const original = document.getText();
    const source = async (text: string) => content(await invoke('designer_source', { session, expected_version: document.version, source: text }));
    const wait = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const programs = async () => content<{ programs: TimelineProgram[] }>(await invoke('designer_studio_query', {
        session, tool: 'designer_programs', arguments: { include_source: false },
    })).programs;
    const undo = async () => content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    await source(scene);
    let frame = await wait();
    let program = (await programs()).find(item => item.selector === 'Button#Lead')!.id;
    const present = async (action: string, fields: object = {}) => {
        content(await invoke('designer_studio_present', { session, expected_revision: frame.revision, tool: 'designer_program',
            arguments: { action, program, ...fields } }));
        frame = await wait();
    };
    const control = async (action: string) => {
        const position = content<TimelinePosition>(await invoke('designer_timeline_control', {
            session, expected_revision: frame.revision, arguments: { action, program },
        }));
        frame = await wait();
        return position;
    };
    const opacity = async (name: string) => {
        const node = frame.nodes.find(node => node.name === name)!.id;
        const { properties } = content<{ properties: { name: string; effective: string }[] }>(await invoke('designer_inspect', { session, node }));
        return Number(properties.find(property => property.name === 'Opacity')?.effective);
    };

    await present('play_program', { time: 0 });
    await present('advance_program', { seconds: 3 });
    await present('pause_program');
    assert.equal(await opacity('Lead'), .5);
    const earlierProgram = program;
    await source(scene.replace('<Canvas.Styles>', '<Canvas.Styles>\n    <Style Selector="TextBlock"/>')
        .replace('Content="First"', 'Content="Updated"'));
    frame = await wait();
    program = (await programs()).find(item => item.selector === 'Button#Lead')!.id;
    assert.notEqual(program, earlierProgram, 'A source update can renumber a legacy program');
    assert.equal(await opacity('Lead'), .5, 'The first replacement image is already at the paused elapsed pose');
    const paused = await control('pause');
    assert.equal(paused.elapsed, 3);
    assert.equal(paused.time, 1);
    assert.equal(paused.playing, false);
    await present('play_program');
    await present('advance_program', { seconds: .5 });
    assert.equal(await opacity('Lead'), .25);
    await undo();
    frame = await wait();
    assert.equal(document.getText(), scene);
    program = (await programs()).find(item => item.selector === 'Button#Lead')!.id;
    assert.equal(program, earlierProgram);
    assert.equal((await control('pause')).elapsed, 3.5);

    await present('play_all', { time: 0 });
    await present('advance_program', { seconds: 3 });
    assert.equal(await opacity('Lead'), .5);
    assert.equal(await opacity('Peer'), .6);
    const all = await control('pause');
    assert.equal(all.all, true);
    assert.equal(all.elapsed, 3);
    await control('stop');
    assert.equal(await opacity('Peer'), .9);
    assert.equal(document.getText(), scene);
    await undo();
    assert.equal(document.getText(), original);
    await wait();
    console.log('VS Code restored the paused alternating iteration across a source edit and played both clips on the native clock');
}
