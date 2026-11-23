import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { TimelineLayout } from '../src/timelineLayout';
import type { TimelinePosition } from '../src/timelineProtocol';
import type { TimelineSnapState, TimelineWorkAreaChange, TimelineWorkAreaState } from '../src/timelineTimingProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface TimingView {
    layout: TimelineLayout;
    revision: number;
    generation?: number;
    sequence?: number;
    work_area?: TimelineWorkAreaState;
    frame_snap?: TimelineSnapState;
}

export async function verifyTimelineTimingMcp({ invoke, content }: McpFixture, session: number,
    document: vscode.TextDocument, hosted = false): Promise<void> {
    const prefix = hosted ? 'designer_hosted_timeline' : 'designer_timeline';
    const transport = hosted ? 'designer_hosted_animation' : 'designer_timeline_control';
    const source = document.getText();
    const version = document.version;
    const read = async () => content<TimingView>(await invoke(prefix + '_view', { session }));
    const identity = (view: TimingView) => ({ session, expected_revision: view.revision,
        ...(hosted && { expected_generation: view.generation, expected_sequence: view.sequence }) });
    const layout = async (layout: TimelineLayout) => {
        const { expected_sequence: _, ...args } = identity(await read());
        content(await invoke(prefix + '_set_layout', { ...args, layout }));
    };
    const ready = async (scope: TimelineWorkAreaState['scope'], at?: number) => {
        let result!: TimingView;
        await until(async () => {
            result = await read();
            const area = result.work_area;
            return Boolean(area?.editable && area.scope === scope && (at === undefined || Math.abs(area.playhead - at) < 1e-7));
        });
        return result;
    };
    const request = (view: TimingView, work_area: TimelineWorkAreaChange) =>
        invoke(prefix + '_set_work_area', { ...identity(view), work_area });
    const apply = async (work_area: TimelineWorkAreaChange) =>
        content<TimingView>(await request(await ready(work_area.scope), work_area)).work_area!;
    const initial = await read();
    await layout('tracks');
    const before = await ready('program');
    const area = before.work_area!;
    const program = area.program!;
    assert.ok(program);
    assert.equal(area.clock, hosted ? 'elapsed' : 'authored');
    const subject = { scope: 'program' as const, program };
    const extent = Math.min(area.limit ?? area.range.end, 1);
    const proposed = { ...subject, action: 'set' as const, range: { start: extent / 4, end: extent / 2, loop: false } };
    const snap = before.frame_snap;
    let clips: TimelineWorkAreaState | undefined;
    try {
        assert.ok((await request({ ...before, revision: before.revision + 1 }, proposed)).isError);
        if (hosted) {
            assert.ok(before.sequence !== undefined);
            assert.match((await request({ ...before, sequence: before.sequence + 1 }, proposed)).content[0]?.text ?? '', /frame sequence/);
        }
        assert.match((await request(before, { ...proposed, program: program + 1000 })).content[0]?.text ?? '', /visible Timeline clock/);
        const invalid = { ...proposed, range: { ...proposed.range, start: proposed.range.end } };
        assert.match((await request(before, invalid)).content[0]?.text ?? '', /In before Out/);
        assert.deepEqual((await read()).work_area, area);

        const changed = await apply(proposed);
        assert.equal(changed.enabled, true);
        assert.deepEqual(changed.range, proposed.range);
        const disabled = await apply({ ...subject, action: 'disable' });
        assert.equal(disabled.enabled, false);
        assert.deepEqual(disabled.range, proposed.range);
        assert.deepEqual((await apply({ ...subject, action: 'enable' })).range, proposed.range);
        const beforeSeek = await ready('program');
        const seekTime = extent * .375;
        const seek = hosted ? { action: 'seek', program, elapsed: seekTime } : { action: 'seek', program, time: seekTime };
        content(await invoke(transport, { ...identity(beforeSeek), arguments: seek }));
        const pose = await ready('program', seekTime);
        assert.equal((await apply({ ...subject, action: 'set_in' })).range.start, pose.work_area?.playhead);
        await apply(proposed);
        assert.equal((await apply({ ...subject, action: 'set_out' })).range.end, pose.work_area?.playhead);
        await apply(proposed);

        if (hosted) assert.equal(snap, undefined);
        else {
            assert.deepEqual(snap?.rates, [0, 24, 30, 60]);
            const setRate = async (frame_rate: number) => invoke(prefix + '_set_frame_rate', {
                ...identity(await ready('program')), frame_snap: { program, frame_rate },
            });
            assert.match((await setRate(25)).content[0]?.text ?? '', /frame rate listed/);
            assert.equal(content<TimingView>(await setRate(24)).frame_snap?.frame_rate, 24);
            assert.equal((await read()).frame_snap?.frame_rate, 24);
        }

        await layout('clips');
        const clipView = await ready('clips');
        clips = clipView.work_area!;
        assert.equal(clips.clock, 'elapsed');
        assert.equal(clipView.frame_snap, undefined);
        assert.match((await request(clipView, proposed)).content[0]?.text ?? '', /visible Timeline clock/);
        const range = { start: .125, end: .5, loop: false };
        const combined = await apply({ scope: 'clips', action: 'set', range });
        assert.deepEqual(combined.range, range);
        const accepted = await ready('clips');
        const result = content<TimelinePosition>(await invoke(transport, { ...identity(accepted), arguments: hosted
            ? { action: 'play_range', range } : { action: 'play_range', program, all: true, range } }));
        if (!hosted) {
            assert.equal(result.all, true);
            assert.deepEqual(result.previewRange, range);
        }
        const held = await ready('clips', range.end);
        assert.deepEqual(held.work_area?.range, range);
        await layout('tracks');
        assert.deepEqual((await ready('program')).work_area?.range, proposed.range);
    } finally {
        if (clips) {
            await layout('clips');
            const accepted = await ready('clips');
            content(await invoke(transport, { ...identity(accepted),
                arguments: hosted ? { action: 'stop' } : { action: 'stop', program } }));
            await apply({ scope: 'clips', action: 'set', range: clips.range });
            if (!clips.enabled) await apply({ scope: 'clips', action: 'disable' });
        }
        await layout('tracks');
        await apply({ ...subject, action: 'set', range: area.range });
        if (!area.enabled) await apply({ ...subject, action: 'disable' });
        if (snap) content(await invoke(prefix + '_set_frame_rate', { ...identity(await ready('program')),
            frame_snap: { program, frame_rate: snap.frame_rate } }));
        await layout(initial.layout);
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
    console.log((hosted ? 'App Preview' : 'Timeline') + ' MCP kept separate work areas and held the exact native Out pose');
}
