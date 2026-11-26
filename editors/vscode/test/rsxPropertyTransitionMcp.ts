import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';
import { asRange } from '../src/textDocument';
import type { CanvasSnapshot, OutlineNode, Range } from '../src/protocol';

interface RustTransitionList {
    canEdit: boolean;
    properties: { property: string; label: string; typeName: string }[];
    entries: { id: number; property: string; typeName: string; durationSeconds: number; range: Range }[];
}

export async function verifyRustPropertyTransitions({ invoke, content }: McpFixture): Promise<void> {
    const name = 'RsxPropertyTransitionsWorkbench.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const doc = await vscode.workspace.openTextDocument(uri);
    const source = doc.getText();
    const card = doc.positionAt(source.indexOf('name: "AccountCard"'));
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: name, line: card.line, character: card.character }));
    const ready = async () => {
        const status = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(status.status, 'Ready');
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const initial = await ready();
    const rendered = content<{ source: string; counterpart: CanvasSnapshot['counterpart'] }>(await invoke('designer_document', {
        session, include_source: true, include_counterpart: true,
    }));
    assert.match(rendered.counterpart.source ?? '', /DoubleTransition Property="{x:Static Visual.OpacityProperty}"/);
    assert.match(rendered.counterpart.source ?? '', /ThicknessTransition Property="{x:Static Layoutable.MarginProperty}"[^>]*Easing="CubicEaseOut"/);
    assert.equal(rendered.counterpart.editable, false);
    assert.equal(rendered.source, source);

    const owner = initial.nodes.find(node => node.name === 'AccountCard')?.id;
    assert.ok(owner);
    const inspect = async () => content<RustTransitionList>(await invoke('designer_studio_query', {
        session, tool: 'designer_property_transitions', arguments: { owner },
    }));
    const edit = (revision: number, arguments_: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_property_transition_edit',
        arguments: { owner, ...arguments_ },
    });
    const snapshots: string[] = [];
    const commit = async (args: object) => {
        const frame = await ready();
        snapshots.push(doc.getText());
        content(await edit(frame.revision, args));
        const current = await ready();
        assert.ok(current.revision > frame.revision);
        return inspect();
    };
    const first = await inspect();
    assert.ok(first.canEdit);
    assert.deepEqual(first.entries.map(entry => entry.property), ['Opacity', 'Margin']);
    assert.match(doc.getText(asRange(first.entries[0].range)), /^fade\(.*hold 🌱.*800u64/);
    const timing = { action: 'timing', transition: first.entries[0].id, timing: {
        duration: 0.92, easing: { kind: 'Spring', value: { mass: 1, stiffness: 190, damping: 18, initial_velocity: 0 } },
    } };
    const retimed = await commit(timing);
    assert.equal(retimed.entries[0].durationSeconds, 0.92);
    assert.match(doc.getText(), /hold 🌱 \*\/ 920u64/);
    assert.match(doc.getText(), /SpringEasing::with_values\(1\.0, 190\.0, 18\.0, 0\.0\)/);
    assert.ok((await edit(initial.revision, timing)).isError);
    const radius = first.properties.find(property => property.label === 'CornerRadius');
    assert.ok(radius);
    const added = await commit({ action: 'add', property: radius.property, type_name: radius.typeName });
    assert.equal(added.entries[2].typeName, 'CornerRadiusTransition');
    const moved = await commit({ action: 'move', transition: added.entries[2].id, index: 0 });
    assert.deepEqual(moved.entries.map(entry => entry.property), ['CornerRadius', 'Opacity', 'Margin']);
    const background = first.properties.find(property => property.label === 'Background');
    assert.ok(background);
    const retargeted = await commit({
        action: 'retarget', transition: moved.entries[0].id, property: background.property, type_name: background.typeName,
    });
    assert.equal(retargeted.entries[0].typeName, 'BrushTransition');
    const removed = await commit({ action: 'remove', transition: retargeted.entries[0].id });
    assert.deepEqual(removed.entries.map(entry => entry.property), ['Opacity', 'Margin']);
    assert.match(doc.getText(), /Account dashboard/);
    for (const snapshot of snapshots.reverse()) {
        content(await invoke('designer_history', { session, expected_version: doc.version, action: 'undo' }));
        await ready();
        assert.equal(doc.getText(), snapshot);
    }
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === name || tab.label === `Design · ${name}`);
    await vscode.window.tabGroups.close(tabs);
}
