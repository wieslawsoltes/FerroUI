import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import { relativeOriginChannels, relativeOriginMarkup } from './hostedRelativeCurves';
import type { McpFixture } from './mcp';

export async function verifyTimelineRelativeMcp({ invoke, content }: McpFixture, session: number, document: vscode.TextDocument) {
    const original = document.getText();
    const source = relativeOriginMarkup('0%,10%', true);
    content(await invoke('designer_source', { session, expected_version: document.version, source }));
    content(await invoke('designer_wait', { session }));
    const query = async <Result>(tool: string, args: object) => content<Result>(await invoke('designer_studio_query', {
        session, tool, arguments: args,
    }));
    const { programs: [program] } = await query<{ programs: TimelineProgram[] }>('designer_programs', { include_source: false });
    const track = program.timeline?.tracks.find(track => track.property === 'RenderTransformOrigin');
    assert.ok(track);
    assert.deepEqual(track.curve_channels, relativeOriginChannels);
    const { points } = await query<{ points: CurvePoint[] }>('designer_animation_curve', { program: program.id, target: track.target,
        property: track.property, channel: 'percent_x', metric: 'velocity', window: { start: 0, end: 4 } });
    assert.ok(points.some(point => point.time > 2.5 && point.break_before));
    assert.ok(points.every(point => point.time < 1.5 || point.time > 2.5));
    assert.equal(document.getText(), source);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    content(await invoke('designer_wait', { session }));
    assert.equal(document.getText(), original);
    console.log('Studio MCP listed relative channels and kept their velocity segments separate');
}
