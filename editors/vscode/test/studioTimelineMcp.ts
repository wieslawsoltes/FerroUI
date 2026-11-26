import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import { acceptedKeyframe } from '../src/timelineKeyframes';
import type { CurvePoint, KeyframeLocation, TimelinePosition, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

export async function verifyTimelineMcp(mcp: McpFixture, session: number, document: vscode.TextDocument, rsxClipboard: unknown): Promise<void> {
    const closeKeys = [.5, .5000001].map(time => ({ time, value_text: '0.75', easing_text: 'LinearEasing' }));
    assert.equal(acceptedKeyframe({ target: 1, property: 'Opacity', keyframes: closeKeys }, .500000072), closeKeys[1]);
    const { invoke, content } = mcp;
    const original = document.getText();
    const wait = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: { id: number; name: string }[] }>(await invoke('designer_outline', { session }));
    };
    const edit = async (revision: number, args: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_program', arguments: args,
    });
    const programs = async () => content<{ programs: TimelineProgram[] }>(await invoke('designer_studio_query', { session, tool: 'designer_programs', arguments: {
        include_source: false, include_keyframe_text: true,
    } })).programs;
    const undo = async () => content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    let frame = await wait();
    const opacity = async () => {
        const target = frame.nodes.find(node => node.name === 'saveButton')!.id;
        const result = content<{ properties: { name: string; effective: string | null }[] }>(
            await invoke('designer_inspect', { session, node: target }));
        const value = result.properties.find(property => property.name === 'Opacity')?.effective;
        assert.ok(value && Number.isFinite(Number(value)));
        return Number(value);
    };
    const baseOpacity = await opacity();
    content(await edit(frame.revision, { action: 'create', owner: frame.nodes[0].id, name: 'Pulse', selector: 'Button#saveButton', duration: 1 }));
    frame = await wait();
    const pulse = async () => {
        const program = (await programs()).find(({ name }) => name === 'Pulse');
        assert.ok(program?.timeline);
        return { ...program, timeline: program.timeline };
    };
    const keyframe = async (options: object = {}) => {
        const program = await pulse();
        return edit(frame.revision, { action: 'upsert_keyframe', program: program.id, target: program.targets[0] ?? program.owner,
            property: 'Opacity', time: .5, value: '0.75', easing: '0.42,0,0.58,1', ...options });
    };
    content(await keyframe());
    frame = await wait();
    const check = async (time: number, value: string) => {
        const keys = (await pulse()).timeline.tracks[0].keyframes;
        const key = keys.find(key => key.time === time);
        assert.ok(key, `No keyframe at ${time}s`);
        assert.equal(key.value_text, value);
        assert.match(key.easing_text, /0\.42/);
    };
    await check(.5, '0.75');
    const program = (await pulse()).id;
    const control = (revision: number, args: object) => invoke('designer_timeline_control', {
        session, expected_revision: revision, arguments: { program, ...args },
    });
    const atHalf = content<{ time: number; playing: boolean; busy: boolean }>(await control(frame.revision, { action: 'seek', time: .5 }));
    assert.equal(atHalf.time, .5);
    assert.equal(atHalf.playing, false);
    assert.equal(atHalf.busy, false);
    assert.ok((await control(frame.revision, { action: 'seek', time: 0 })).isError);
    frame = await wait();
    const beforeRecording = document.getText();
    const recordTime = .625;
    content(await control(frame.revision, { action: 'seek', time: recordTime }));
    frame = await wait();
    const target = frame.nodes.find(node => node.name === 'saveButton')!.id;
    const sampledOpacity = await opacity();
    const { points } = content<{ points: CurvePoint[] }>(await invoke('designer_studio_query', { session, tool: 'designer_animation_curve',
        arguments: { program, target, property: 'Opacity', window: { start: 0, end: 1 } } }));
    assert.equal(points.find(point => point.time === recordTime)?.value, sampledOpacity);
    assert.equal(await opacity(), sampledOpacity);
    assert.equal(document.getText(), beforeRecording);
    content(await edit(frame.revision, { action: 'record_keyframe', program, target, property: 'Opacity', time: recordTime }));
    await wait();
    const recorded = (await pulse()).timeline.tracks[0].keyframes.find(key => key.time === recordTime);
    assert.ok(recorded);
    assert.equal(Number(recorded.value_text), sampledOpacity);
    await undo();
    assert.equal(document.getText(), beforeRecording);
    frame = await wait();
    const timing = { duration: 2, delay: .25, delay_between_iterations: .25, speed_ratio: 2, repeat: '3',
        playback_direction: 'Alternate', playback_behavior: 'OnlyIfVisible', fill: 'Both', easing: 'LinearEasing' };
    content(await edit(frame.revision, { action: 'update_timing', program, timing }));
    assert.ok((await edit(frame.revision, { action: 'update_timing', program, timing: { delay: 0 } })).isError);
    frame = await wait();
    const updated = (await pulse()).timeline;
    assert.equal(updated.duration, 2);
    assert.equal(updated.playback_direction, 'Alternate');
    assert.equal(updated.playback_behavior, 'OnlyIfVisible');
    assert.equal(updated.fill, 'Both');
    assert.equal(updated.repeat_text, '3');
    assert.equal(updated.total_duration, 3.375);
    await check(1, '0.75');
    const beforeReverseKey = document.getText();
    content(await keyframe({ time: 1.5, value: '0.25' }));
    frame = await wait();
    const timedSource = document.getText();
    const keyframes = [1, 1.5].map(time => ({
        target: frame.nodes.find(node => node.name === 'saveButton')!.id, property: 'Opacity', time,
    }));
    const editKeys = (action: string, offset?: number, keys = keyframes) => edit(frame.revision, { action, program, keyframes: keys, offset });
    const keyTimes = async () => (await pulse()).timeline.tracks[0].keyframes.map(key => key.time);
    content(await editKeys('move_keyframes', -.25));
    frame = await wait();
    assert.ok((await keyTimes()).includes(.75));
    assert.ok((await keyTimes()).includes(1.25));
    await undo();
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    const beforeScale = (await pulse()).timeline.tracks[0].keyframes;
    const scale = { action: 'scale_keyframes', program, keyframes, pivot: 1, factor: .5 };
    content(await edit(frame.revision, scale));
    frame = await wait();
    const scaled = (await pulse()).timeline.tracks[0].keyframes;
    assert.deepEqual(scaled, beforeScale.map(key => key.time === 1.5 ? { ...key, time: 1.25 } : key));
    assert.ok((await edit(frame.revision, { ...scale, factor: 0 })).isError);
    await undo();
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    const collision = await editKeys('duplicate_keyframes', .5);
    assert.ok(collision.isError && collision.content[0]?.text?.includes('occupies a destination time'));
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    content(await editKeys('duplicate_keyframes', .125));
    frame = await wait();
    const duplicated = document.getText();
    const copies = keyframes.map(key => ({ ...key, time: key.time + .125,
        target: frame.nodes.find(node => node.name === 'saveButton')!.id }));
    const copiedTimes = await keyTimes();
    assert.ok(copies.every(key => copiedTimes.includes(key.time)));
    content(await editKeys('remove_keyframes', undefined, copies));
    await wait();
    const remainingTimes = await keyTimes();
    assert.ok(copies.every(key => !remainingTimes.includes(key.time)));
    await undo();
    assert.equal(document.getText(), duplicated);
    await wait();
    await undo();
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    const destination = { target: keyframes[0].target, property: 'Opacity' };
    const paste = { action: 'paste_keyframes', program, paste: { clipboard: rsxClipboard, destinations: [destination], time: .125 } };
    const pasted = content<{ keyframes: KeyframeLocation[] }>(await edit(frame.revision, paste));
    assert.deepEqual(pasted.keyframes.map(key => key.time), [.125, .625]);
    frame = await wait();
    assert.deepEqual((await pulse()).timeline.tracks[0].keyframes.filter(key => [.125, .625].includes(key.time)).map(key => key.value_text), ['0.25', '0.65']);
    await undo();
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    assert.ok((await edit(frame.revision, { ...paste, paste: { ...paste.paste, time: .5 } })).isError);
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    const reflected = content<{ keyframes: KeyframeLocation[] }>(await edit(frame.revision, {
        action: 'mirror_keyframes', program, keyframes, pivot: .875,
    }));
    assert.deepEqual(reflected.keyframes.map(key => key.time), [.75, .25]);
    frame = await wait();
    assert.deepEqual(await keyTimes(), [0, .25, .75, 2]);
    await undo();
    assert.equal(document.getText(), timedSource);
    frame = await wait();
    const present = async (action: string, options: object) => {
        content(await invoke('designer_studio_present', { session, expected_revision: frame.revision, tool: 'designer_program',
            arguments: { action, program, ...options } }));
        frame = await wait();
    };
    await present('play_program', { time: .125 });
    await present('advance_program', { seconds: .5 });
    assert.equal(await opacity(), .75);
    await present('advance_program', { seconds: .875 });
    assert.equal(await opacity(), .25);
    await present('pause_program', {});
    const verified = content<TimelinePosition>(await control(frame.revision, { action: 'seek', time: 1.5 }));
    assert.equal(verified.time, 1.5);
    assert.equal(verified.elapsed, undefined);
    frame = await wait();
    assert.equal(await opacity(), .25);
    const stopped = content<TimelinePosition>(await control(frame.revision, { action: 'stop' }));
    assert.equal(stopped.time, 0);
    assert.equal(stopped.playing, false);
    frame = await wait();
    assert.equal(await opacity(), baseOpacity);
    assert.equal(document.getText(), timedSource);
    await undo();
    assert.equal(document.getText(), beforeReverseKey);
    frame = await wait();
    assert.ok((await edit(frame.revision, { action: 'update_timing', program, timing: { repeat: '9223372036854775808' } })).isError);
    frame = await wait();
    assert.equal((await pulse()).timeline.total_duration, 3.375);
    await undo();
    assert.equal(document.getText(), beforeRecording);
    frame = await wait();
    const duplicate = await keyframe();
    assert.ok(duplicate.isError && duplicate.content[0]?.text?.includes('occupies the destination time'));
    frame = await wait();
    const draft = { program, target: frame.nodes.find(node => node.name === 'saveButton')!.id, property: 'Opacity',
        previous: .5, time: .75, value: '0.25', easing: '0.42,1.8,0.58,1.4' };
    const previewDraft = (arguments_: object) => invoke('designer_studio_query', { session, tool: 'designer_keyframe_draft', arguments: arguments_ });
    const unchanged = document.version;
    const graph = content<{ samples: number[]; valueKind: string }>(await previewDraft(draft));
    assert.equal(graph.valueKind, 'Number');
    assert.equal(graph.samples.length, 33);
    assert.equal(graph.samples[0], 0);
    assert.equal(graph.samples.at(-1), 1);
    assert.ok(Math.max(...graph.samples) > 1.1);
    assert.ok((await previewDraft({ ...draft, easing: '2,0,0.5,1' })).isError);
    assert.ok((await previewDraft({ ...draft, time: 3 })).isError);
    assert.ok((await previewDraft({ ...draft, value: 'blue' })).isError);
    assert.equal(document.version, unchanged);
    content(await keyframe(draft));
    await wait();
    await check(.75, '0.25');
    assert.match((await pulse()).timeline.tracks[0].keyframes.find(key => key.time === .75)!.easing_text, /1\.8/);
    await undo();
    assert.equal(document.getText(), beforeRecording);
    await wait();
    await check(.5, '0.75');
    await undo();
    await wait();
    assert.deepEqual((await pulse()).timeline.tracks, []);
    await undo();
    assert.equal(document.getText(), original);
    console.log('VS Code MCP validated keyframe drafts, RSX-to-XAML paste, mirror, collision rejection, one-step Undo, and native playback');
}
