import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { AnimationGraphMode } from '../src/animationGraph';
import type { TimelineLayout } from '../src/timelineLayout';
import type { TimelineRuler, TimelineViewportChange, TimelineViewportState } from '../src/timelineViewportProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface TimelineDisplay {
    mode: AnimationGraphMode;
    layout: TimelineLayout;
    revision: number;
    available_modes: AnimationGraphMode[];
    generation?: number;
    viewport?: TimelineViewportState;
}

export async function verifyTimelineGraphMcp(fixture: McpFixture, session: number,
    document: vscode.TextDocument, generation?: number): Promise<void> {
    const { invoke, content } = fixture;
    const prefix = generation === undefined ? 'designer_timeline' : 'designer_hosted_timeline';
    const read = async () => content<TimelineDisplay>(await invoke(`${prefix}_view`, { session }));
    const first = await read();
    const version = document.version;
    const source = document.getText();
    assert.deepEqual(first.available_modes, ['keys', 'value', 'velocity']);
    assert.equal(first.generation, generation);
    const select = (graph_mode: AnimationGraphMode, expected_revision = first.revision, expected_generation = generation) =>
        invoke(`${prefix}_set_graph`, { session, expected_revision, graph_mode,
            ...(expected_generation === undefined ? {} : { expected_generation }) });
    assert.match((await select('value', first.revision + 1)).content[0]?.text ?? '', /current .*revision/);
    if (generation !== undefined) {
        assert.match((await select('velocity', first.revision, generation + 1)).content[0]?.text ?? '', /current .*generation/);
    }
    assert.equal((await read()).mode, first.mode);
    for (const mode of ['velocity', 'value', 'keys', first.mode] as const) {
        const applied = content<TimelineDisplay>(await select(mode));
        assert.equal(applied.mode, mode);
        assert.equal(applied.revision, first.revision);
        assert.equal((await read()).mode, mode);
        if (mode === 'value' && first.layout === 'tracks') {
            await verifyViewport(fixture, prefix, session, generation === undefined ? 'tracks' : 'curve');
        }
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
}

export async function verifyTimelineLayoutMcp(fixture: McpFixture, session: number,
    document: vscode.TextDocument, generation?: number): Promise<void> {
    const { invoke, content } = fixture;
    const prefix = generation === undefined ? 'designer_timeline' : 'designer_hosted_timeline';
    const read = async () => content<TimelineDisplay>(await invoke(`${prefix}_view`, { session }));
    const first = await read();
    const version = document.version;
    const source = document.getText();
    const select = (layout: TimelineLayout, expected_revision = first.revision, expected_generation = generation) =>
        invoke(`${prefix}_set_layout`, { session, expected_revision, layout,
            ...(expected_generation === undefined ? {} : { expected_generation }) });
    assert.match((await select('clips', first.revision + 1)).content[0]?.text ?? '', /current .*revision/);
    if (generation !== undefined) {
        assert.match((await select('clips', first.revision, generation + 1)).content[0]?.text ?? '', /current .*generation/);
    }
    assert.equal((await read()).layout, first.layout);
    for (const layout of ['clips', 'tracks', first.layout] as const) {
        const applied = content<TimelineDisplay>(await select(layout));
        assert.equal(applied.layout, layout);
        assert.equal(applied.revision, first.revision);
        assert.equal(applied.mode, first.mode);
        assert.equal((await read()).layout, layout);
        if (layout === 'clips') await verifyViewport(fixture, prefix, session, 'clips');
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
}

async function verifyViewport({ invoke, content }: McpFixture, prefix: string, session: number, ruler: TimelineRuler) {
    const read = async () => content<TimelineDisplay>(await invoke(prefix + '_view', { session }));
    let display: TimelineDisplay | undefined;
    await until(async () => {
        display = await read();
        return display.viewport?.ruler === ruler;
    });
    const { revision, generation, viewport } = display!;
    const { duration, program } = viewport!;
    const apply = (viewport: TimelineViewportChange, expected_revision = revision, expected_generation = generation) =>
        invoke(prefix + '_set_viewport', { session, expected_revision, viewport,
            ...(expected_generation === undefined ? {} : { expected_generation }) });
    const window = { action: 'window', ruler, program, start: duration / 4, end: duration * .75 } as const;
    assert.match((await apply(window, revision + 1)).content[0]?.text ?? '', /current .*revision/);
    if (generation !== undefined) {
        assert.match((await apply(window, revision, generation + 1)).content[0]?.text ?? '', /current .*generation/);
    }
    assert.match((await apply({ ...window, ruler: ruler === 'clips' ? 'tracks' : 'clips' })).content[0]?.text ?? '', /ruler or animation changed/);
    assert.match((await apply({ ...window, end: duration + 1 })).content[0]?.text ?? '', /within the ruler/);
    assert.deepEqual((await read()).viewport, viewport);
    const changed = content<TimelineDisplay>(await apply(window)).viewport!;
    assert.equal(changed.start, window.start);
    assert.ok(Math.abs(changed.end - window.end) < 1e-8);
    assert.deepEqual((await read()).viewport, changed);
    const fitted = content<TimelineDisplay>(await apply({ action: 'fit', ruler, program })).viewport!;
    assert.deepEqual([fitted.start, fitted.end], [0, duration]);
    console.log(prefix + ' verified the visible ' + ruler + ' viewport and exact source history');
}
