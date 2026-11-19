import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';

export async function verifyRsxAnimation({ invoke, content }: McpFixture): Promise<void> {
    const path = 'animation_view.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const original = new TextDecoder().decode(await vscode.workspace.fs.readFile(uri)).replace(/\r?\n/g, '\r\n');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(original));
    const document = await vscode.workspace.openTextDocument(uri);
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
    assert.deepEqual(tools.edits.find(tool => tool.name === 'designer_program')?.inputSchema.properties.action?.enum, ['upsert_keyframe']);

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
    content(await invoke('designer_timeline_control', { session, expected_revision: frame.revision,
        arguments: { action: 'seek', program: updated.id, time: 1.4 } }));
    frame = await outline();
    const inspection = content<{ properties: { name: string; effective: string }[] }>(await invoke('designer_inspect', {
        session, node: frame.nodes.find(node => node.name === 'Forecast')!.id,
    }));
    assert.equal(Number(inspection.properties.find(property => property.name === 'Opacity')?.effective), .82);
    content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
    assert.equal(document.getText(), original);
    frame = await outline();
    content(await invoke('designer_block', { session, expected_revision: frame.revision, index: 0 }));
    await outline();
    assert.deepEqual(await programs(), []);
    console.log('RSX animation used native playback and keyframe validation, preserved CRLF and Unicode, rejected a stale edit, and restored exact Rust with Undo');
}
