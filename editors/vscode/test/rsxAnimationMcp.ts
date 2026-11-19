import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxAnimation({ invoke, content }: McpFixture): Promise<void> {
    const path = 'animation_view.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const document = await vscode.workspace.openTextDocument(uri);
    assert.equal(document.eol, vscode.EndOfLine.CRLF);
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const outline = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const query = (tool: string, args: object) => invoke('designer_studio_query', { session, tool, arguments: args });
    const programs = async () => content<{ programs: TimelineProgram[] }>(
        await query('designer_programs', { include_source: false, include_keyframe_text: true })).programs;
    const edit = (revision: number, draft: object) => invoke('designer_studio_edit', { session,
        expected_revision: revision, tool: 'designer_program', arguments: { action: 'upsert_keyframe', ...draft } });
    assert.deepEqual(await programs(), []);
    const initial = await outline();
    content(await invoke('designer_block', { session, expected_revision: initial.revision, index: 1 }));
    let frame = await outline();
    const [program] = await programs();
    assert.ok(program?.timeline && program.sourceRange && program.preview?.available);
    assert.match(document.getText(asRange(program.sourceRange)), /^ferroui_view::animation\(/);
    assert.ok(frame.nodes.every(node => !node.label.endsWith('.Styles') && node.label !== 'KeyFrame'));
    const target = frame.nodes.find(node => node.name === 'Forecast')!;
    assert.ok(target.bounds && program.targets.includes(target.id));
    assert.match(document.getText(asRange(target.nameRange)), /^Border$/);
    const tools = content<{ edits: { name: string; inputSchema: { properties: { action?: { enum?: string[] } } } }[] }>(
        await invoke('designer_studio_tools', { session }));
    assert.deepEqual(tools.edits.find(tool => tool.name === 'designer_program')?.inputSchema.properties.action?.enum,
        ['create', 'rename', 'duplicate', 'remove', 'upsert_keyframe', 'record_keyframe', 'remove_keyframe',
            'move_keyframes', 'scale_keyframes', 'duplicate_keyframes', 'remove_keyframes']);

    const draft = { program: program.id, target: program.timeline.tracks[0].target, property: 'Opacity',
        previous: .5, time: 1.4, value: '0.82', easing: '0.25,0.3,0.75,1.4' };
    assert.ok((await query('designer_keyframe_draft', { ...draft, time: 2 })).isError);
    const preview = content<{ samples: number[] }>(await query('designer_keyframe_draft', draft));
    assert.ok(preview.samples.some(value => value > 1));
    content(await edit(frame.revision, draft));
    assert.equal(document.getText(), original.replace('.key_frame(0.25,', '.key_frame(0.7,')
        .replace('0.65f64', '0.82f64').replace('0.25, 0.1, 0.75, 1.0', '0.25, 0.3, 0.75, 1.4'));
    assert.ok((await edit(frame.revision, draft)).isError);
    frame = await outline();
    const [updated] = await programs();
    const key = updated.timeline?.tracks[0].keyframes[1];
    assert.equal(key?.time, 1.4);
    assert.equal(key?.value_text, '0.82');
    const curve = { program: updated.id, target: updated.timeline!.tracks[0].target, property: 'Opacity',
        window: { start: 0, end: 2 } };
    const samples = content<{ points: CurvePoint[] }>(await query('designer_animation_curve', curve)).points;
    assert.equal(samples.find(point => point.time === 1.4)?.value, .82);
    assert.ok(samples.some(point => point.time < 1.4 && point.value > .82));
    assert.ok((await query('designer_animation_curve', { ...curve, window: { start: 1, end: 3 } })).isError);
    content(await invoke('designer_timeline_control', { session, expected_revision: frame.revision,
        arguments: { action: 'seek', program: updated.id, time: 1.4 } }));
    frame = await outline();
    const inspection = content<{ properties: { name: string; effective: string }[] }>(await invoke('designer_inspect', {
        session, node: frame.nodes.find(node => node.name === 'Forecast')!.id,
    }));
    assert.equal(Number(inspection.properties.find(property => property.name === 'Opacity')?.effective), .82);
    const editVersion = document.version;
    content(await invoke('designer_history', { session, expected_version: editVersion, action: 'undo' }));
    await until(() => document.version !== editVersion).catch(() => assert.fail(
        `Undo left the Rust buffer at version ${editVersion}. Focused window: ${vscode.window.state.focused}. Active editor: ${vscode.window.activeTextEditor?.document.fileName}.`));
    assert.equal(document.getText(), original);
    frame = await outline();
    const [restored] = await programs();
    const track = restored.timeline!.tracks[0];
    const keyframes = track.keyframes.slice(1).map(key => ({ target: track.target, property: track.property, time: key.time }));
    const move = { action: 'move_keyframes', program: restored.id, keyframes, offset: -0.25 };
    content(await edit(frame.revision, move));
    assert.equal(document.getText(), original.replace('.key_frame(0.25,', '.key_frame(0.125,')
        .replace('.key_frame(1.0,', '.key_frame(0.875,'));
    assert.ok((await edit(frame.revision, move)).isError);
    const movedVersion = document.version;
    content(await invoke('designer_history', { session, expected_version: movedVersion, action: 'undo' }));
    await until(() => document.version !== movedVersion);
    assert.equal(document.getText(), original);
    frame = await outline();
    const scale = { action: 'scale_keyframes', program: restored.id, keyframes, pivot: 0, factor: .5 };
    content(await edit(frame.revision, scale));
    assert.equal(document.getText(), original.replace('.key_frame(0.25,', '.key_frame(0.125,')
        .replace('.key_frame(1.0,', '.key_frame(0.5,'));
    assert.ok((await edit(frame.revision, scale)).isError);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    frame = await outline();
    const undo = async (expected: string) => {
        const version = document.version;
        content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
        await until(() => document.version !== version);
        assert.equal(document.getText(), expected);
        return outline();
    };
    const value = { program: restored.id, target: track.target, property: track.property };
    const insertion = { ...value, time: .75, value: '0.74', easing: 'LinearEasing' };
    assert.equal(content<{ valueKind: string }>(await query('designer_keyframe_draft', insertion)).valueKind, 'Number');
    content(await edit(frame.revision, insertion));
    const withKey = document.getText();
    const sourceLine = /\r\n[ \t]+\.key_frame\(0\.375, \|key\| key\.set\(ferroui_base::Visual::opacity_property\(\), 0\.74\)\)/;
    assert.match(withKey, sourceLine);
    assert.equal(withKey.replace(sourceLine, ''), original);
    assert.ok((await edit(frame.revision, insertion)).isError);
    frame = await outline();
    const [withKeyProgram] = await programs();
    content(await edit(frame.revision, { ...value, action: 'remove_keyframe', program: withKeyProgram.id,
        target: withKeyProgram.timeline!.tracks[0].target, time: .75 }));
    assert.equal(document.getText(), original);
    await undo(withKey);
    frame = await undo(original);

    content(await edit(frame.revision, { ...value, action: 'record_keyframe', time: 1.25 }));
    frame = await outline();
    const recorded = (await programs())[0].timeline!.tracks[0].keyframes.find(key => key.time === 1.25);
    assert.ok(recorded && Math.abs(Number(recorded.value_text) - .825) < 1e-10);
    frame = await undo(original);

    const originals = track.keyframes.slice(0, 2).map(key => ({ target: track.target, property: track.property, time: key.time }));
    content(await edit(frame.revision, { action: 'duplicate_keyframes', program: restored.id, keyframes: originals, offset: .25 }));
    frame = await outline();
    const [copiedProgram] = await programs();
    const copiedTrack = copiedProgram.timeline!.tracks[0];
    const copies = copiedTrack.keyframes;
    assert.deepEqual(copies.map(key => key.time), [0, .25, .5, .75, 2]);
    assert.equal(copies[1].value_text, copies[0].value_text);
    assert.equal(copies[3].easing_text, copies[2].easing_text);
    const duplicated = document.getText();
    content(await edit(frame.revision, { action: 'remove_keyframes', program: copiedProgram.id,
        keyframes: originals.map(key => ({ ...key, target: copiedTrack.target, time: key.time + .25 })) }));
    assert.equal(document.getText(), original);
    await undo(duplicated);
    frame = await undo(original);

    content(await invoke('designer_block', { session, expected_revision: frame.revision, index: 0 }));
    await outline();
    assert.deepEqual(await programs(), []);
    const fixtureTabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === 'Design · animation_view.rs' || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(fixtureTabs);
    console.log('RSX animation validated native keyframes, added, retimed, duplicated, and removed mapped keys, rejected stale revisions, and restored CRLF and Unicode with Undo');
}
