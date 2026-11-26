import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CurvePoint, TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxColorConstants({ invoke, content }: McpFixture): Promise<void> {
    const path = 'palette_motion.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const document = await vscode.workspace.openTextDocument(uri);
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const accept = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number }>(await invoke('designer_outline', { session }));
    };
    const query = async <T>(tool: string, args: object) => content<T>(
        await invoke('designer_studio_query', { session, tool, arguments: args }));
    const programs = async () => (await query<{ programs: TimelineProgram[] }>(
        'designer_programs', { include_keyframe_text: true })).programs;
    const before = await accept();
    const [program] = await programs();
    const track = program.timeline!.tracks.find(track => track.property === 'Background')!;
    assert.deepEqual(track.keyframes.map(key => key.value_text), ['#FF274676', '#FF274676']);
    const draft = { program: program.id, target: track.target, property: 'Background',
        previous: 2, time: 2, easing: 'LinearEasing', value: '#FF376E55' };
    await query('designer_keyframe_draft', draft);
    const update = { session, expected_revision: before.revision, tool: 'designer_program',
        arguments: { action: 'upsert_keyframe', ...draft } };
    content(await invoke('designer_studio_edit', update));
    assert.equal(document.getText(), original.replace('brand::/* end ink 🎨 */ INK', '/* end ink 🎨 */ 0xFF37_6E55_u32'));
    assert.ok((await invoke('designer_studio_edit', update)).isError);

    await accept();
    const [after] = await programs();
    const { points } = await query<{ points: CurvePoint[] }>('designer_animation_curve', {
        program: after.id, target: after.targets[0], property: 'Background', channel: 'green', window: { start: 0, end: 2 },
    });
    assert.equal(points[0].value, 0x46 / 255);
    assert.equal(points.at(-1)?.value, 0x6e / 255);
    const version = document.version;
    content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
    await until(() => document.version !== version);
    assert.equal(document.getText(), original);

    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === 'Design · palette_motion.rs' || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code updated one use of a shared Rust ARGB constant, sampled both keys, rejected a stale edit, and restored exact source with Undo');
}
