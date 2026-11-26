import { strict as assert } from 'node:assert';
import type * as vscode from 'vscode';
import type { PropertyField } from '../src/protocol';
import type { TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

export async function verifyTimelineRecording(mcp: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const { invoke, content } = mcp;
    const original = document.getText();
    const binding = 'Opacity="{Binding AccountOpacity, FallbackValue=0.4}"';
    const bound = original.replace('x:Name="saveButton"', `x:Name="saveButton" ${binding}`);
    assert.notEqual(bound, original);
    content(await invoke('designer_source', { session, expected_version: document.version, source: bound }));
    const read = async () => {
        content(await invoke('designer_wait', { session }));
        const frame = content<{ revision: number; nodes: { id: number; name?: string }[] }>(
            await invoke('designer_outline', { session }));
        return { revision: frame.revision, root: frame.nodes[0].id, button: frame.nodes.find(node => node.name === 'saveButton')!.id };
    };
    let frame = await read();
    const studioEdit = (revision: number, tool: string, args: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool, arguments: args,
    });
    content(await studioEdit(frame.revision, 'designer_program', {
        action: 'create', owner: frame.root, name: 'Property recording', selector: 'Button#saveButton', duration: 1,
    }));
    frame = await read();
    const authored = document.getText();
    const readProgram = async () => {
        const response = content<{ programs: TimelineProgram[] }>(await invoke('designer_studio_query', {
            session, tool: 'designer_programs', arguments: { include_source: false, include_keyframe_text: true },
        }));
        const program = response.programs.find(program => program.name === 'Property recording');
        assert.ok(program?.timeline);
        return { ...program, timeline: program.timeline };
    };
    const property = async (name: string) => {
        const result = content<{ properties: PropertyField[] }>(await invoke('designer_inspect', { session, node: frame.button }));
        return result.properties.find(property => property.name === name)!;
    };
    const program = await readProgram();
    const control = (args: object) => invoke('designer_timeline_control', {
        session, expected_revision: frame.revision, arguments: { program: program.id, ...args },
    });
    const setProperty = async (name: string, value: string, revision = frame.revision) =>
        invoke('designer_edit_property', { session, expected_revision: revision, node: frame.button, action: 'set', property: name, value });
    const before = await property('Opacity');
    assert.equal(Number(before.effective), .4);
    assert.equal(before.canRecord, true);
    const time = .679363636363636;
    const armed = content<{ recording: boolean; time: number }>(await control({ action: 'record', enabled: true, time }));
    assert.equal(armed.recording, true);
    assert.equal(armed.time, time);
    frame = await read();
    const stale = frame.revision;
    content(await setProperty('Opacity', '0.65'));
    frame = await read();
    assert.match(document.getText(), /Opacity="\{Binding AccountOpacity, FallbackValue=0\.4\}"/);
    assert.equal(Number((await property('Opacity')).effective), .65, 'The first accepted frame must already include the recorded sample');
    const track = (await readProgram()).timeline.tracks.find(track => track.property === 'Opacity');
    assert.equal(track?.keyframes.find(key => key.time === .6793636)?.value_text, '0.65');
    assert.ok((await setProperty('Opacity', '0.95', stale)).isError);
    const recorded = document.getText();
    assert.ok((await studioEdit(frame.revision, 'designer_record_property', {
        program: program.id, time, nodes: [frame.button], property: 'Opacity', value: 'blue',
    })).isError);
    assert.equal(document.getText(), recorded);
    frame = await read();
    const disarmed = content<{ recording: boolean }>(await control({ action: 'record', enabled: false }));
    assert.equal(disarmed.recording, false);
    content(await setProperty('Width', '152'));
    frame = await read();
    assert.equal((await property('Width')).authored, '152');
    assert.ok(!(await readProgram()).timeline.tracks.some(track => track.property === 'Width'));
    assert.equal(Number((await property('Opacity')).effective), .65, 'Unrelated source edits must retain the authored pose');
    content(await control({ action: 'stop' }));
    frame = await read();
    assert.equal(Number((await property('Opacity')).effective), .4);
    const undo = async (expected: string) => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
        await read();
    };
    await undo(recorded);
    await undo(authored);
    await undo(bound);
    await undo(original);
    console.log('VS Code recorded a Properties edit at the playhead, retained the bound base and native pose, and resumed ordinary edits after disarming');
}
