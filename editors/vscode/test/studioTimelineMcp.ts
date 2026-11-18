import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { McpFixture } from './mcp';

export async function verifyTimelineMcp(mcp: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const { invoke, content } = mcp;
    const original = document.getText();
    const wait = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: { id: number; name: string }[] }>(await invoke('designer_outline', { session }));
    };
    const edit = async (revision: number, args: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_program', arguments: args,
    });
    const programs = async () => content<{ programs: { id: number; name: string; targets: number[]; owner: number; timeline: {
        tracks: { keyframes: { time: number; value_text: string; easing_text: string }[] }[];
    } }[] }>(await invoke('designer_studio_query', { session, tool: 'designer_programs', arguments: {
        include_source: false, include_keyframe_text: true,
    } })).programs;
    const undo = async () => content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    let frame = await wait();
    content(await edit(frame.revision, { action: 'create', owner: frame.nodes[0].id, name: 'Pulse', selector: 'Button#saveButton', duration: 1 }));
    frame = await wait();
    const pulse = async () => {
        const program = (await programs()).find(({ name }) => name === 'Pulse');
        assert.ok(program);
        return program;
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
    const { properties } = content<{ properties: { name: string; effective: string | null }[] }>(
        await invoke('designer_inspect', { session, node: target }));
    const opacity = properties.find(property => property.name === 'Opacity')?.effective;
    assert.ok(opacity && Number.isFinite(Number(opacity)));
    content(await edit(frame.revision, { action: 'record_keyframe', program, target, property: 'Opacity', time: recordTime }));
    await wait();
    const recorded = (await pulse()).timeline.tracks[0].keyframes.find(key => key.time === recordTime);
    assert.ok(recorded);
    assert.equal(Number(recorded.value_text), Number(opacity));
    await undo();
    assert.equal(document.getText(), beforeRecording);
    frame = await wait();
    const duplicate = await keyframe();
    assert.ok(duplicate.isError && duplicate.content[0]?.text?.includes('occupies the destination time'));
    frame = await wait();
    content(await keyframe({ previous: .5, time: .75, value: '0.25' }));
    await wait();
    await check(.75, '0.25');
    await undo();
    await wait();
    await check(.5, '0.75');
    await undo();
    await wait();
    assert.deepEqual((await pulse()).timeline.tracks, []);
    await undo();
    assert.equal(document.getText(), original);
    console.log('VS Code MCP authored and scrubbed a native timeline, recorded an effective value, rejected a stale seek, and undid each source transaction');
}
