import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

interface TransitionList {
    canEdit: boolean;
    entries: {
        id: number;
        property: string;
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
    const inspect = async () => content<TransitionList>(await invoke('designer_studio_query', {
        session, tool: 'designer_property_transitions', arguments: { owner },
    }));
    const before = await inspect();
    assert.ok(before.canEdit);
    assert.deepEqual(before.entries.map(transition => transition.property), ['Opacity', 'Background']);
    assert.equal(before.entries[0].durationSeconds, .3);
    assert.ok(!before.entries[1].editableTiming.includes('Easing'));
    const apply = (revision: number, transition: number, timing: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_property_transition_edit',
        arguments: { action: 'timing', owner, transition, timing },
    });
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
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => tab.label === name || tab.label === `Design · ${name}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code edited native property transition timing, preserved a resource-backed easing, and restored the full XAML in one Undo');
}
