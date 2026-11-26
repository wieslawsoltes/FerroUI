import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { Bounds, OutlineNode } from '../src/protocol';
import type { TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

const scene = `<Canvas xmlns="https://github.com/ferroui">
    <Border Name="MotionCard" Background="#336699" Canvas.Left="24" Canvas.Right="8" Canvas.Bottom="250" Width="100" Height="60"/>
    <Border Name="OtherCard" Background="#996633" Canvas.Left="195" Canvas.Top="32" Width="100" Height="60"/>
</Canvas>`;

export async function verifyTimelineGeometry({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const original = document.getText();
    content(await invoke('designer_source', { session, expected_version: document.version, source: scene }));
    const read = async () => {
        content(await invoke('designer_wait', { session }));
        const frame = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const card = frame.nodes.find(node => node.name === 'MotionCard');
        assert.ok(card?.bounds);
        return { ...frame, card: { ...card, bounds: card.bounds } };
    };
    let frame = await read();
    const native = (tool: string, args: object) => invoke('designer_studio_edit', { session, expected_revision: frame.revision, tool, arguments: args });
    content(await native('designer_program', { action: 'create', name: 'Card motion', owner: frame.nodes[0].id, selector: 'Border#MotionCard', duration: 2 }));
    frame = await read();
    const authored = document.getText();
    const programs = async () => content<{ programs: TimelineProgram[] }>(await invoke('designer_studio_query', {
        session, tool: 'designer_programs', arguments: { include_source: false, include_keyframe_text: true },
    })).programs;
    const program = (await programs()).find(candidate => candidate.name === 'Card motion');
    assert.ok(program);
    const control = (args: object) => invoke('designer_timeline_control', {
        session, expected_revision: frame.revision, arguments: { program: program.id, ...args },
    });
    const time = .9993182;
    content(await control({ action: 'record', enabled: true, time }));
    frame = await read();
    const base = frame.card.bounds;
    const preserved = /Canvas.Left="24" Canvas.Right="8" Canvas.Bottom="250" Width="100" Height="60"/;
    content(await invoke('designer_resize', { session, expected_revision: frame.revision, node: frame.card.id, handle: 4, dx: 24, dy: 16 }));
    frame = await read();
    assert.deepEqual(frame.card.bounds, { ...base, width: 124, height: 76 });
    assert.match(document.getText(), preserved);
    const sized = (await programs()).find(candidate => candidate.name === 'Card motion')!.timeline!;
    assert.ok(!sized.tracks.some(track => track.property === 'Canvas.Left'), 'The effective leading anchor stays in source');
    assert.equal(sized.tracks.find(track => track.property === 'Canvas.Top')?.keyframes[0].value_text, String(base.y));
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), authored);
    frame = await read();
    const move = { session, expected_revision: frame.revision, node: frame.card.id, x: base.x + 40, y: base.y + 20 };
    const hint = content<{ description: string; bounds: Bounds }>(await invoke('designer_placement_hint', move));
    assert.equal(hint.description, 'Record move at playhead');
    assert.equal(hint.bounds.x, move.x);
    content(await invoke('designer_place', move));
    frame = await read();
    assert.deepEqual(frame.card.bounds, { ...base, x: move.x, y: move.y });
    const moved = document.getText();
    assert.match(moved, preserved);
    assert.ok((await invoke('designer_place', move)).isError);

    const other = frame.nodes.find(node => node.name === 'OtherCard')!;
    const group = { session, expected_revision: frame.revision, selection: [frame.card.id, other.id], x: move.x + 12, y: move.y + 8 };
    assert.ok((await invoke('designer_place', group)).isError, 'Choose one native animation scope before recording a group');
    assert.equal(document.getText(), moved);
    frame = await read();
    content(await invoke('designer_resize', { session, expected_revision: frame.revision, node: frame.card.id, handle: 4, dx: 24, dy: 16 }));
    frame = await read();
    assert.deepEqual(frame.card.bounds, { x: move.x, y: move.y, width: 124, height: 76 });
    const timeline = (await programs()).find(candidate => candidate.name === 'Card motion')!.timeline!;
    for (const [name, initial, value] of [['Canvas.Left', 24, move.x], ['Canvas.Top', base.y, move.y], ['Width', 100, 124], ['Height', 60, 76]] as const) {
        const track = timeline.tracks.find(candidate => candidate.property === name);
        assert.equal(track?.keyframes.find(key => key.time === 0)?.value_text, String(initial));
        assert.equal(track?.keyframes.find(key => key.time === time)?.value_text, String(value));
    }
    assert.match(document.getText(), preserved);
    content(await control({ action: 'stop' }));
    frame = await read();
    assert.deepEqual(frame.card.bounds, base);
    for (const expected of [moved, authored, scene, original]) {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
    }
    console.log('VS Code recorded native Canvas movement and size at the playhead, preserved base anchors, and undid one gesture at a time');
}
