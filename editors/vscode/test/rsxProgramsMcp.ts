import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { TimelineProgram } from '../src/timelineProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyRsxPrograms({ invoke, content }: McpFixture): Promise<void> {
    const path = 'animation_view.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const document = await vscode.workspace.openTextDocument(uri);
    const original = document.getText();
    const sibling = original.slice(original.indexOf('fn forecast'));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const accept = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const catalog = async () => {
        const result = await invoke('designer_studio_query', { session, tool: 'designer_programs',
            arguments: { include_source: false, include_keyframe_text: true } });
        return content<{ programs: TimelineProgram[] }>(result).programs;
    };
    const edit = (revision: number, args: object) => invoke('designer_studio_edit', { session, expected_revision: revision,
        tool: 'designer_program', arguments: args });
    let frame = await accept();
    assert.deepEqual(await catalog(), []);
    const owner = frame.nodes.find(node => node.label === 'TextBlock');
    assert.ok(owner?.bounds);
    const create = { action: 'create', owner: owner.id, name: 'Headline reveal', selector: 'TextBlock', duration: .85 };
    assert.ok((await edit(frame.revision, { ...create, selector: 'TextBlock[' })).isError);
    assert.equal(document.getText(), original);
    frame = await accept();
    content(await edit(frame.revision, create));
    const created = document.getText();
    assert.match(created, /styles: \[\r\n/);
    assert.ok(created.endsWith(sibling));
    assert.ok((await edit(frame.revision, create)).isError);
    frame = await accept();
    const [clip] = await catalog();
    assert.ok(clip?.sourceRange && clip.source_key && clip.preview?.available);
    assert.equal(clip.name, 'Headline reveal');
    assert.equal(clip.timeline?.duration, .85);
    assert.match(document.getText(asRange(clip.sourceRange)), /^ferroui_view::animation\(/);
    content(await edit(frame.revision, { action: 'upsert_keyframe', program: clip.id, target: clip.targets[0],
        property: 'Opacity', time: .425, value: '0.55', easing: 'LinearEasing' }));
    const withKey = document.getText();
    frame = await accept();
    const [keyed] = await catalog();
    content(await edit(frame.revision, { action: 'rename', name: 'Headline / review 🪴', program: keyed.id }));
    const renamed = document.getText();
    frame = await accept();
    const [renamedClip] = await catalog();
    assert.equal(renamedClip.name, 'Headline / review 🪴');
    assert.equal(renamedClip.source_key, clip.source_key);
    content(await edit(frame.revision, { action: 'duplicate', name: 'Headline copy', program: renamedClip.id }));
    const duplicated = document.getText();
    frame = await accept();
    const [first, copy] = await catalog();
    assert.equal(copy.name, 'Headline copy');
    assert.notEqual(copy.source_key, first.source_key);
    assert.equal(copy.timeline?.tracks[0].keyframes[0].value_text, '0.55');
    const place = { action: 'place_clips', programs: [first.id, copy.id], start: .3, gap: -.1 };
    const retired = frame.revision;
    content(await edit(retired, place));
    assert.ok((await edit(retired, place)).isError);
    const placed = document.getText();
    assert.match(placed, /\.delay\(ferroui_view::ms\(300\)\)/);
    assert.match(placed, /\.delay\(ferroui_view::ms\(1050\)\)/);
    assert.equal(placed.replace(/\.delay\(ferroui_view::ms\((?:300|1050)\)\)/g, ''), duplicated);
    assert.ok(placed.endsWith(sibling));
    frame = await accept();
    assert.deepEqual((await catalog()).map(program => program.clip?.start), [.3, 1.05]);
    const afterPlacement = document.version;
    content(await invoke('designer_history', { session, expected_version: afterPlacement, action: 'undo' }));
    await until(() => document.version !== afterPlacement);
    assert.equal(document.getText(), duplicated);
    frame = await accept();
    content(await edit(frame.revision, { action: 'remove', program: first.id }));
    const removed = document.getText();
    frame = await accept();
    const [remaining] = await catalog();
    assert.equal(remaining.source_key, copy.source_key);
    content(await edit(frame.revision, { action: 'remove', program: remaining.id }));
    await accept();
    assert.deepEqual(await catalog(), []);
    assert.ok(document.getText().endsWith(sibling));

    for (const expected of [removed, duplicated, renamed, withKey, created, original]) {
        const version = document.version;
        content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
        await until(() => document.version !== version);
        assert.equal(document.getText(), expected);
    }
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === 'Design · animation_view.rs' || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code created, sequenced, renamed, duplicated, and removed source-backed Rust clips with checked source and one Undo per action');
}
