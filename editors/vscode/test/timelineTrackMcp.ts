import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { AnimationGraphMode } from '../src/animationGraph';
import type { TimelineTrackCommand, TimelineTrackView } from '../src/timelineTrackProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

interface TimelineDisplay {
    mode: AnimationGraphMode;
    track_view?: TimelineTrackView;
    revision: number;
    generation?: number;
    sequence?: number;
}

async function openTrackView({ invoke, content }: McpFixture, prefix: string, session: number, hosted: boolean) {
    const read = async () => content<TimelineDisplay>(await invoke(prefix + '_view', { session }));
    const before = await read();
    const args = ({ revision, generation, sequence }: TimelineDisplay) => ({
        session, expected_revision: revision, ...(hosted && { expected_generation: generation, expected_sequence: sequence }),
    });
    const graph = async (graph_mode: AnimationGraphMode) => {
        const { expected_sequence: _, ...identity } = args(await read());
        content(await invoke(prefix + '_set_graph', { ...identity, graph_mode }));
    };
    await graph('value');
    let display = await read();
    await until(async () => {
        display = await read();
        return Boolean(display.track_view?.tracks.length);
    });
    const request = (track: TimelineTrackCommand, identity = args(display)) => invoke(prefix + '_track_action', { ...identity, track });
    const apply = async (track: TimelineTrackCommand) => content<TimelineDisplay>(await request(track, args(await read()))).track_view!;
    return { read, request, apply, args, display, restore: () => graph(before.mode) };
}

export async function verifyTimelineTrackMcp(fixture: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const source = document.getText();
    const version = document.version;
    const { read, request, apply, args, display, restore } = await openTrackView(fixture, 'designer_timeline', session, false);
    const initial = display.track_view!;
    const program = initial.program;
    const group = initial.groups!.find(group => group.canLock && group.canCollapse)!;
    assert.ok(group, 'An authored target group can be collapsed and locked');
    const component = initial.tracks.find(track => track.target === group.target && track.canChooseChannel)!;
    assert.ok(component?.channels?.length, 'The compiled document has a multi-component curve');
    const target = group.target;
    const track = { target, property: component.property };
    const componentBefore = component.channel!;
    assert.ok(componentBefore);
    const collapsed = (value: boolean) => ({ program, action: 'collapse' as const, target, collapsed: value });
    const lock = (locked: boolean) => ({ program, action: 'lock' as const, target, locked });
    const filter = (query: string, selectedOnly = false) => ({ program, action: 'filter' as const, query, selectedOnly });
    const targetState = (view: TimelineTrackView) => view.groups!.find(group => group.target === target)!;
    try {
        const stale = await request(filter('anything'), { ...args(display), expected_revision: display.revision + 1 });
        assert.match(stale.content[0]?.text ?? '', /current .*revision/);
        const retired = await request({ ...filter('anything'), program: program + 123 });
        assert.match(retired.content[0]?.text ?? '', /current program/);

        assert.equal(targetState(await apply(collapsed(true))).expanded, false);
        const exact = await apply(filter(component.property.toUpperCase()));
        assert.equal(targetState(exact).expanded, true);
        assert.equal(targetState(exact).canCollapse, false);
        assert.ok(exact.tracks.some(item => item.visible && item.property === component.property));
        assert.ok(exact.tracks.some(item => !item.visible && item.property !== component.property));
        assert.match((await request(collapsed(false))).content[0]?.text ?? '', /track filter cleared/);
        const empty = await apply(filter('no property with this label', true));
        assert.equal(empty.filter?.selectedOnly, true);
        assert.ok(empty.tracks.every(track => !track.visible));
        assert.equal(targetState(await apply(filter(''))).expanded, false);

        await apply(collapsed(false));
        assert.equal(targetState(await apply(lock(true))).locked, true);
        const chosen = component.channels.find(channel => channel !== componentBefore)!;
        const changed = await apply({ program, action: 'channel', ...track, channel: chosen });
        assert.equal(changed.tracks.find(item => item.target === target && item.property === component.property)?.channel, chosen);
        assert.deepEqual((await read()).track_view, changed);
        await apply({ program, action: 'channel', ...track, channel: componentBefore });
    } finally {
        await apply(filter(''));
        await apply(lock(group.locked));
        await apply(collapsed(!group.expanded));
        if (initial.filter?.query || initial.filter?.selectedOnly) {
            await apply(filter(initial.filter.query, initial.filter.selectedOnly));
        }
        await restore();
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
    console.log('Timeline MCP read back filter, disclosure, lock, and component controls with exact source history');
}

export async function verifyHostedTrackMcp(fixture: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const source = document.getText();
    const version = document.version;
    const { read, request, apply, args, display, restore } = await openTrackView(fixture, 'designer_hosted_timeline', session, true);
    const { program, tracks, selected } = display.track_view!;
    assert.ok(selected && display.sequence !== undefined);
    const component = tracks.find(track => track.property === 'CornerRadius')!;
    assert.ok(component?.channels?.length);
    const track = { target: component.target, property: component.property };
    const select: TimelineTrackCommand = { program, action: 'select', ...track };
    try {
        assert.match((await request(select, { ...args(display), expected_sequence: display.sequence + 1 })).content[0]?.text ?? '', /current App Preview frame sequence/);
        assert.match((await request({ ...select, target: select.target + 5000 })).content[0]?.text ?? '', /live target/);
        const chosen = await apply(select);
        assert.deepEqual(chosen.selected, track);
        assert.equal(chosen.tracks.find(row => row.visible)?.property, component.property);
        const current = chosen.tracks.find(row => row.visible)?.channel!;
        const channel = component.channels.find(channel => channel !== current)!;
        const command = { program, action: 'channel' as const, ...track, channel };
        const changed = await apply(command);
        assert.deepEqual((await read()).track_view, changed);
        assert.equal(changed.tracks.find(row => row.visible)?.channel, channel);
        await apply({ ...command, channel: current });
    } finally {
        await apply({ program, action: 'select', ...selected });
        await restore();
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
    console.log('App Preview MCP selected a live property and component with exact source history');
}

export async function verifyRsxTrackMcp(fixture: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const source = document.getText();
    const version = document.version;
    const { display, apply, restore } = await openTrackView(fixture, 'designer_timeline', session, false);
    const view = display.track_view!;
    const group = view.groups!.find(group => group.name === 'Forecast')!;
    assert.ok(group?.canLock, 'The mapped Rust track can be locked through the same control');
    const lock = (locked: boolean) => ({ program: view.program, action: 'lock' as const, target: group.target, locked });
    const filter = (query: string) => ({ program: view.program, action: 'filter' as const, query, selectedOnly: false });
    try {
        const matched = await apply(filter('forecast opacity'));
        assert.ok(matched.tracks.some(track => track.target === group.target && track.property === 'Opacity' && track.visible));
        const locked = await apply(lock(true));
        assert.equal(locked.groups?.find(item => item.target === group.target)?.locked, true);
    } finally {
        await apply(lock(group.locked));
        await apply({ ...filter(view.filter?.query ?? ''), selectedOnly: view.filter?.selectedOnly ?? false });
        await restore();
    }
    assert.equal(document.version, version);
    assert.equal(document.getText(), source);
    console.log('Mapped RSX Timeline filtering and locking preserve CRLF, comments, and expressions');
}
