import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

interface TransitionList {
    canEdit: boolean;
    canAdd: boolean;
    canEditCollection: boolean;
    requiresTarget: boolean;
    properties: { property: string; label: string; typeName: string }[];
    entries: {
        id: number;
        property: string;
        typeName: string;
        durationSeconds: number;
        delaySeconds: number;
        editableTiming: string[];
    }[];
}

export async function verifyPropertyTransitions({ invoke, content }: McpFixture): Promise<void> {
    const name = 'PropertyTransitionsWorkbench.xaml';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const doc = await vscode.workspace.openTextDocument(uri);
    const source = doc.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: name }));
    const frame = async () => {
        const status = content<{ status: string }>(await invoke('designer_wait', { session }));
        assert.equal(status.status, 'Ready');
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    const initial = await frame();
    const owner = initial.nodes.find(node => node.name === 'AccountCard')?.id;
    assert.ok(owner);
    const inspect = async (node = owner, target_type?: string) => content<TransitionList>(await invoke('designer_studio_query', {
        session, tool: 'designer_property_transitions', arguments: {
            owner: node, ...(target_type === undefined ? {} : { target_type }),
        },
    }));
    const before = await inspect();
    assert.ok(before.canEdit);
    assert.ok(before.canAdd && before.canEditCollection);
    assert.deepEqual(before.entries.map(transition => transition.property), ['Opacity', 'Background']);
    assert.equal(before.entries[0].durationSeconds, .3);
    assert.ok(!before.entries[1].editableTiming.includes('Easing'));
    const edit = (revision: number, args: object, node = owner) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_property_transition_edit',
        arguments: { owner: node, ...args },
    });
    const apply = (revision: number, transition: number, timing: object) =>
        edit(revision, { action: 'timing', transition, timing });
    const blocked = await apply(initial.revision, before.entries[1].id, { easing: { kind: 'Linear' } });
    assert.ok(blocked.isError);
    assert.match(blocked.content[0]?.text ?? '', /Edit Easing at its source/);
    assert.equal(doc.getText(), source);
    const ready = await frame();
    content(await apply(ready.revision, (await inspect()).entries[0].id, {
        duration: .65, delay: .1, easing: { kind: 'Spring', value: { mass: 1, stiffness: 190, damping: 18, initial_velocity: 0 } },
    }));
    const updated = await frame();
    assert.ok(updated.revision > ready.revision);
    const after = await inspect();
    assert.equal(after.entries[0].durationSeconds, .65);
    assert.equal(after.entries[0].delaySeconds, .1);
    assert.match(doc.getText(), /<DoubleTransition\.Easing>\s*<SpringEasing/);
    assert.match(doc.getText(), /Easing="\{StaticResource CardEasing\}"/);
    assert.ok((await apply(initial.revision, before.entries[0].id, { duration: .8 })).isError);
    content(await invoke('designer_history', { session, expected_version: doc.version, action: 'undo' }));
    await frame();
    assert.equal(doc.getText(), source);
    assert.equal((await inspect()).entries[0].durationSeconds, .3);

    const snapshots: string[] = [];
    const commit = async (args: object, node = owner) => {
        const current = await frame();
        snapshots.push(doc.getText());
        content(await edit(current.revision, args, node));
        const accepted = await frame();
        assert.ok(accepted.revision > current.revision);
        return inspect(node);
    };
    assert.ok(before.properties.some(property => property.property === 'Margin' && property.typeName === 'ThicknessTransition'));
    const incompatible = await edit((await frame()).revision, {
        action: 'add', property: 'Margin', type_name: 'DoubleTransition',
    });
    assert.ok(incompatible.isError);
    assert.match(incompatible.content[0]?.text ?? '', /compatible property and transition class/);
    assert.equal(doc.getText(), source);
    const added = await commit({ action: 'add', property: 'Margin', timing: { duration: .55 } });
    assert.equal(added.entries[2].typeName, 'ThicknessTransition');
    assert.equal(added.entries[2].durationSeconds, .55);
    const moved = await commit({ action: 'move', transition: added.entries[2].id, index: 0 });
    assert.deepEqual(moved.entries.map(transition => transition.property), ['Margin', 'Opacity', 'Background']);
    const retargeted = await commit({ action: 'retarget', transition: moved.entries[1].id,
        property: 'BorderBrush', type_name: 'SolidColorBrushTransition' });
    assert.equal(retargeted.entries[1].typeName, 'SolidColorBrushTransition');
    assert.equal(retargeted.entries[1].durationSeconds, .3);
    assert.match(doc.getText(), /SolidColorBrushTransition Property="BorderBrush" Duration="0:0:0.3" Easing="CubicEaseOut"/);
    const removed = await commit({ action: 'remove', transition: retargeted.entries[0].id });
    assert.deepEqual(removed.entries.map(transition => transition.property), ['BorderBrush', 'Background']);
    assert.match(doc.getText(), /Preserve the separate timing/);
    assert.match(doc.getText(), /Easing="{StaticResource CardEasing}"/);

    const nodes = (await frame()).nodes;
    const card = nodes.find(node => node.name === 'SummaryCard')?.id;
    assert.ok(card);
    assert.equal((await inspect(card)).canAdd, false);
    const scope = (label: string, declaration: string) => nodes.find(node => node.label === label
        && doc.getText(new vscode.Range(node.range.start.line, node.range.start.character,
            node.range.end.line, node.range.end.character)).includes(declaration))?.id;
    const style = scope('Style', 'Border#SummaryCard');
    assert.ok(style);
    const styleEdit = await commit({ action: 'add', property: 'Opacity' }, style);
    assert.equal(styleEdit.entries[0].typeName, 'DoubleTransition');
    assert.match(doc.getText(), /<Setter Property="Transitions"><Transitions><DoubleTransition Property="Opacity"/);
    const theme = scope('ControlTheme', 'x:Key="AccountTheme"');
    assert.ok(theme);
    const themeEdit = await commit({ action: 'add', property: 'Margin' }, theme);
    assert.equal(themeEdit.entries[0].typeName, 'ThicknessTransition');
    const resource = scope('Transitions', 'x:Key="SharedFade"');
    assert.ok(resource);
    assert.ok((await inspect(resource)).requiresTarget);
    assert.ok((await inspect(resource, 'Border')).canAdd);
    const resourceEdit = await commit({ action: 'add', target_type: 'Border', property: 'Width' }, resource);
    assert.equal(resourceEdit.entries[1].typeName, 'DoubleTransition');
    assert.match(doc.getText(), /WidthProperty}/);
    for (const snapshot of snapshots.reverse()) {
        content(await invoke('designer_history', { session, expected_version: doc.version, action: 'undo' }));
        await frame();
        assert.equal(doc.getText(), snapshot);
    }
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === name || tab.label === `Design · ${name}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code edited native transition timing and collections on controls, styles, themes, and shared resources with exact Undo');
}
