import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import { transformCard } from './hostedTransformCurves';
import type { McpFixture } from './mcp';

export async function verifyTimelineTransformMcp({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument) {
    const original = document.getText();
    const source = transformCard(20, 4, true);
    content(await invoke('designer_source', { session, expected_version: document.version, source }));
    content(await invoke('designer_wait', { session }));
    const property = 'RotateTransform.Angle';
    const query = async <Result>(tool: string, args: object) => content<Result>(await invoke('designer_studio_query', {
        session, tool, arguments: args,
    }));
    const programs = () => query<{ programs: TimelineProgram[] }>('designer_programs', {
        include_source: false, include_keyframe_text: true,
    });
    const { programs: [program] } = await programs();
    const { revision } = content<{ revision: number }>(await invoke('designer_outline', { session }));
    const target = program.targets[0];
    const { points } = await query<{ points: CurvePoint[] }>('designer_animation_curve', {
        program: program.id, target, property, window: { start: 0, end: 2 },
    });
    assert.deepEqual(points.find(point => point.time === .5), { time: .5, value: 50 });
    assert.equal(document.getText(), source);
    const record = () => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_program',
        arguments: { action: 'record_keyframe', program: program.id, target, property, time: .5 },
    });
    content(await record());
    assert.ok((await record()).isError);
    content(await invoke('designer_wait', { session }));
    const { programs: [edited] } = await programs();
    const keys = edited.timeline?.tracks.find(track => track.property === property)?.keyframes;
    assert.deepEqual(keys?.map(key => [key.time, key.value_text]), [[.5, '50'], [1, '80'], [2, '100']]);
    const undo = async () => content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    await undo();
    assert.equal(document.getText(), source);
    await undo();
    content(await invoke('designer_wait', { session }));
    assert.equal(document.getText(), original);
    console.log('VS Code MCP sampled a transform track, recorded a native key, rejected a retired frame, and restored the exact source');
}
