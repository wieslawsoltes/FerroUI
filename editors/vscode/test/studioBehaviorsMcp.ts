import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { BehaviorReport } from '../src/behaviorEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyStudioBehaviors({ invoke, content }: McpFixture): Promise<void> {
    const name = 'BehaviorsWorkbench.xaml';
    const file = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const document = await vscode.workspace.openTextDocument(file);
    const original = document.getText();
    const tool = async <T>(command: string, args: object) => content<T>(await invoke(command, args));
    const { session } = await tool<{ session: number }>('designer_open', { path: name });
    const read = async (control = 'Approve') => {
        const status = await tool<{ renderError: string | null }>('designer_wait', { session });
        assert.ok(!status.renderError, status.renderError ?? undefined);
        const frame = await tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
        const owner = frame.nodes.find(node => node.name === control);
        assert.ok(owner);
        const report = await tool<BehaviorReport>('designer_studio_query', {
            session, tool: 'designer_behaviors', arguments: { owner: owner.id },
        });
        return { owner, report, revision: frame.revision };
    };
    const present = (revision: number, arguments_: object) => tool('designer_studio_present', {
        session, expected_revision: revision, tool: 'designer_behavior', arguments: arguments_,
    });
    const edit = (revision: number, arguments_: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_behavior', arguments: arguments_,
    });
    const undo = async (text = original) => {
        await tool('designer_history', { session, expected_version: document.version, action: 'undo' });
        await until(() => document.getText() === text);
    };
    const catalog = await tool<{ presentations: { name: string }[]; edits: { name: string }[] }>('designer_studio_tools', { session });
    assert.ok(catalog.presentations.some(tool => tool.name === 'designer_behavior'));
    assert.ok(catalog.edits.some(tool => tool.name === 'designer_behavior'));
    const before = await read();
    const [trigger] = before.report.behaviors;
    assert.ok(trigger.canPreview && trigger.canEditTrigger && trigger.canEditActions);
    assert.equal(trigger.actionRows.length, 3);
    assert.match(document.getText(asRange(trigger.actionRows[0].range)), /Value='278'/);
    await tool('designer_select', { session, expected_revision: before.revision, node: before.owner.id });
    await tool('designer_view', { session, view: 'behaviors' });
    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    const view = designer.behaviors;
    await until(async () => (await view.getChildren()).some(row => row.behavior.id === trigger.id
        && row.editor.context.token === panel.currentToken));
    const root = (await view.getChildren())[0];
    const first = (await view.getChildren(root))[0];
    assert.match(view.getTreeItem(root).contextValue ?? '', /\bpreview\b/);
    assert.equal(view.getTreeItem(first).description, 'Width → 278');
    await vscode.commands.executeCommand('ferroui.behaviors.preview', root);
    const played = await read();
    assert.equal(played.owner.bounds?.width, 338);
    assert.equal(played.report.activePreview, trigger.id);
    assert.equal(document.getText(), original);
    const retired = await edit(before.revision, { action: 'remove', behavior: trigger.id });
    assert.ok(retired.isError);
    await present(played.revision, { action: 'clear_preview' });
    assert.equal((await read()).owner.bounds?.width, 210);
    assert.equal(document.getText(), original);

    await until(async () => (await view.getChildren()).some(row => row.editor.context.token === panel.currentToken));
    const currentRoot = (await view.getChildren())[0];
    const children = await view.getChildren(currentRoot);
    const cancellation = new vscode.CancellationTokenSource();
    try {
        const transfer = new vscode.DataTransfer();
        view.handleDrag([children[0]], transfer, cancellation.token);
        const cancelled = new vscode.CancellationTokenSource();
        cancelled.cancel();
        await view.handleDrop(children[1], transfer, cancelled.token);
        cancelled.dispose();
        assert.equal(document.getText(), original);
        await view.handleDrop(children[1], transfer, cancellation.token);
        const moved = await read();
        const source = document.getText();
        assert.ok(source.indexOf("Value='338'") < source.indexOf("Value='278'"));
        assert.ok(source.includes('<!-- Keep the review and archive steps together -->'));
        await present(moved.revision, { action: 'preview', behavior: moved.report.behaviors[0].id });
        const reordered = await read();
        assert.equal(reordered.owner.bounds?.width, 278);
        await present(reordered.revision, { action: 'clear_preview' });
        await undo();
    } finally { cancellation.dispose(); }

    const current = await read();
    const item = current.report.behaviors[0];
    content(await edit(current.revision, { action: 'set_action', behavior: item.id,
        action_node: item.actionRows[1].id, item: { ChangeProperty: { target: null, property: 'Width', value: '315' } } }));
    assert.equal(document.getText(), original.replace("Value='338'", "Value='315'"));
    await undo();

    const triggerModel = await read();
    content(await edit(triggerModel.revision, { action: 'set_trigger', behavior: triggerModel.report.behaviors[0].id,
        trigger: { Event: { event: 'PointerPressed', source: null } } }));
    assert.equal(document.getText(), original.replace("EventName='Click'", "EventName='PointerPressed'"));
    await undo();

    const failure = await read();
    const failingBehavior = failure.report.behaviors[0];
    content(await edit(failure.revision, { action: 'set_action', behavior: failingBehavior.id,
        action_node: failingBehavior.actionRows[1].id,
        item: { ChangeProperty: { target: null, property: 'MissingBehaviorField', value: '338' } } }));
    const invalidAction = await read();
    const rejected = await invoke('designer_studio_present', { session, expected_revision: invalidAction.revision,
        tool: 'designer_behavior', arguments: { action: 'preview', behavior: invalidAction.report.behaviors[0].id } });
    assert.ok(rejected.isError && rejected.content[0]?.text?.includes('MissingBehaviorField'));
    const recovered = await read();
    assert.equal(recovered.owner.bounds?.width, 210);
    assert.equal(recovered.report.activePreview, null);
    await undo();

    const selected = await read('Notify');
    assert.deepEqual(selected.report.behaviors, []);
    assert.ok(selected.report.canAttach);
    content(await edit(selected.revision, { action: 'upsert', model: {
        owner: selected.owner.id,
        trigger: { Data: { binding: '{Binding IsChecked}', comparison: 'Equal', value: 'True', revert_on_false: true } },
        actions: [{ AddClass: { target: null, class: 'notify' } }],
    } }));
    const attached = await read('Notify');
    assert.ok('Data' in attached.report.behaviors[0].trigger);
    await undo();
    panel.panel.dispose();
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === name));
    console.log('Native Behaviors preview, action drop, reset, typed edit, stale rejection, and grouped Undo passed');
}
