import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { TimelineMarker, TimelinePosition, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';
import { verifyTimelineGraphMcp } from './timelineGraphMcp';
import { until } from './wait';

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
    await verifyTimelineGraphMcp({ invoke, content }, session, document);
    let program = (await programs()).find(item => item.selector === 'Button#Lead')!.id;
    const present = async (action: string, fields: object = {}) => {
        content(await invoke('designer_studio_present', { session, expected_revision: frame.revision, tool: 'designer_program',
            arguments: { action, program, ...fields } }));
        frame = await wait();
    };
    const control = async (action: string, fields: object = {}, refresh = true) => {
        const position = content<TimelinePosition>(await invoke('designer_timeline_control', {
            session, expected_revision: frame.revision, arguments: { action, program, ...fields },
        }));
        if (refresh) frame = await wait();
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

    const range = { start: .25, end: .75, loop: false };
    const invalid = await invoke('designer_timeline_control', { session, expected_revision: frame.revision,
        arguments: { action: 'play_range', program, range: { ...range, start: 1 } } });
    assert.equal(invalid.isError, true, 'In must precede Out');
    const started = await control('play_range', { range }, false);
    assert.deepEqual(started.previewRange, range);
    assert.equal(started.time, range.start);
    let outRevision: number | undefined;
    await until(async () => {
        frame = await wait();
        const node = frame.nodes.find(item => item.name === 'Lead')!.id;
        const result = await invoke('designer_inspect', { session, node });
        if (result.isError) return false;
        const { revision, properties } = content<{ revision: number; properties: { name: string; effective: string }[] }>(result);
        if (Number(properties.find(property => property.name === 'Opacity')?.effective) !== .375) return false;
        outRevision = revision;
        return true;
    });
    assert.ok(outRevision);
    const latePause = (revision: number) => invoke('designer_timeline_control', {
        session, expected_revision: revision, arguments: { action: 'pause', program },
    });
    const completed = content<TimelinePosition>(await latePause(outRevision - 1));
    assert.equal(completed.time, range.end, 'Pause at the final frame boundary retains the authored Out pose');
    assert.equal(completed.playing, false);
    assert.equal(completed.completed, true);
    assert.equal((await latePause(outRevision - 2)).isError, true, 'Older presentation frames are rejected');
    frame = await wait();
    const loop = await control('play_range', { range: { ...range, loop: true } }, false);
    assert.equal(loop.time, range.start, 'Play at Out restarts the preview from In');
    let pausedRange: TimelinePosition | undefined;
    await until(async () => {
        frame = await wait();
        const result = await invoke('designer_timeline_control', { session, expected_revision: frame.revision, arguments: { action: 'pause', program } });
        if (result.isError) return false;
        pausedRange = content<TimelinePosition>(result);
        return true;
    });
    assert.ok(pausedRange && !pausedRange.playing && pausedRange.time >= range.start && pausedRange.time <= range.end);
    frame = await wait();
    await control('stop');
    const markerFrame = { session, expected_revision: frame.revision, program };
    const markers = async () => content<{ markers: TimelineMarker[] }>(await invoke('designer_timeline_markers', markerFrame)).markers;
    const editMarker = async (arguments_: object) => invoke('designer_timeline_marker_edit', { ...markerFrame, arguments: arguments_ });
    assert.equal((await editMarker({ action: 'set', name: 'Settled', time: 3 })).isError, true);
    content(await editMarker({ action: 'set', name: 'Settled', time: .75 }));
    content(await editMarker({ action: 'set', name: 'In', time: .25 }));
    content(await editMarker({ action: 'set', name: 'Settled', time: .8 }));
    assert.deepEqual(await markers(), [{ name: 'In', time: .25 }, { name: 'Settled', time: .8 }]);
    content(await editMarker({ action: 'remove', name: 'In' }));
    assert.deepEqual(await markers(), [{ name: 'Settled', time: .8 }]);
    const markerPose = content<{ position: TimelinePosition }>(await editMarker({ action: 'seek', name: 'Settled' }));
    assert.equal(markerPose.position.time, .8);
    assert.equal((await editMarker({ action: 'remove', name: 'Settled' })).isError, true, 'A retired frame cannot edit markers');
    assert.equal(await opacity('Lead'), .4);
    assert.equal(document.getText(), scene);

    frame = await wait();
    const sharedFrame = { session, expected_revision: frame.revision, all: true };
    const shared = (action: string, time?: number, revision = frame.revision) => invoke('designer_timeline_marker_edit', {
        ...sharedFrame, expected_revision: revision, arguments: { action, name: 'Settled', ...(time !== undefined && { time }) },
    });
    content(await shared('set', 1.25));
    const sharedPose = content<{ position: TimelinePosition }>(await shared('seek'));
    assert.equal(sharedPose.position.all, true);
    assert.equal(sharedPose.position.elapsed, 1.25);
    frame = await wait();
    content(await shared('remove'));
    const independent = content<{ markers: TimelineMarker[] }>(await invoke('designer_timeline_markers', { ...markerFrame, expected_revision: frame.revision }));
    assert.deepEqual(independent.markers, [{ name: 'Settled', time: .8 }]);

    const clips = await programs();
    assert.deepEqual(clips.map(clip => clip.clip), [{ start: 0, end: 6, editable: true }, { start: 0, end: 1, editable: true }]);
    const place = (revision: number, ids: number[]) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_program',
        arguments: { action: 'place_clips', programs: ids, start: .25, gap: .5 },
    });
    assert.ok((await place(frame.revision, [clips[0].id, clips[0].id])).isError);
    assert.equal(document.getText(), scene);
    frame = await wait();
    const retired = frame.revision;
    content(await place(retired, clips.map(clip => clip.id)));
    assert.equal(document.getText().replace(' Delay="0:0:0.2500000"', '').replace(' Delay="0:0:6.7500000"', ''), scene);
    assert.ok((await place(retired, clips.map(clip => clip.id))).isError);
    frame = await wait();
    const placed = await programs();
    assert.deepEqual(placed.map(clip => clip.clip), [{ start: .25, end: 6.25, editable: true }, { start: 6.75, end: 7.75, editable: true }]);
    program = placed[0].id;
    const elapsed = await control('scrub_all', { time: 1.25 });
    assert.equal(elapsed.all, true);
    assert.equal(elapsed.elapsed, 1.25);
    assert.equal(elapsed.playing, false);
    assert.equal(await opacity('Lead'), .5);
    assert.equal(await opacity('Peer'), .9);
    await control('scrub_all', { time: 7.25 });
    assert.ok(Math.abs(await opacity('Peer') - .4) < 1e-9);
    await undo();
    assert.equal(document.getText(), scene, 'Undo reverses the whole sequence');
    await undo();
    assert.equal(document.getText(), original);
    await wait();
    console.log('VS Code restored a paused iteration, rehearsed a range, and authored and scrubbed a two-clip sequence with one Undo');
}
