import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { BehaviorReport } from '../src/behaviorEditor';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import type { OutlineNode } from '../src/protocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export const rustBehaviors = `use ferroui_view::behavior::{event as on_click, change_property};
use ferroui_view::prelude::*;
fn heading() -> Element { rsx! { TextBlock { text: "Keep this view 🌿" } } }
fn approval() -> Element {
    rsx! {
        StackPanel {
            width: 420.0,
            spacing: 12.0,
            Button {
                name: "Approve", width: 210.0, height: 50.0, content: "Approve",
                behaviors: [on_click("Click", None, [
                    change_property(None, "Width", /* first width 🌿 */ r#"278"#),
                    change_property(None, "Width", "338"),
                ])],
            }
            Button { name: "Notify", width: 160.0, height: 50.0, content: "Notify" }
        }
    }
}
`;

export async function verifyRsxBehaviors({ invoke, content }: McpFixture): Promise<void> {
    const name = 'approval_behaviors.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const original = rustBehaviors.replaceAll('\n', '\r\n');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(original));
    const document = await vscode.workspace.openTextDocument(uri);
    const tool = async <T>(command: string, args: object) => content<T>(await invoke(command, args));
    const caret = document.positionAt(original.lastIndexOf('rsx!') + 8);
    const { session } = await tool<{ session: number }>('designer_open', { path: name, line: caret.line, character: caret.character });
    const read = async (control = 'Approve') => {
        const result = await tool<{ renderError: string | null }>('designer_wait', { session });
        assert.ok(!result.renderError, result.renderError ?? undefined);
        const frame = await tool<{ revision: number; nodes: OutlineNode[] }>('designer_outline', { session });
        const owner = frame.nodes.find(node => node.name === control);
        assert.ok(owner);
        const report = await tool<BehaviorReport>('designer_studio_query', {
            session, tool: 'designer_behaviors', arguments: { owner: owner.id },
        });
        return { owner, report, revision: frame.revision };
    };
    const edit = (revision: number, arguments_: object) => invoke('designer_studio_edit', {
        session, expected_revision: revision, tool: 'designer_behavior', arguments: arguments_,
    });
    const present = (revision: number, arguments_: object) => tool('designer_studio_present', {
        session, expected_revision: revision, tool: 'designer_behavior', arguments: arguments_,
    });
    const undo = async () => {
        await tool('designer_history', { session, expected_version: document.version, action: 'undo' });
        assert.equal(document.getText(), original);
    };
    const { designer } = await vscode.extensions.getExtension<{ designer: DesignerWorkbench }>('FerroUI.ferroui')!.activate();
    const panel = designer.designs().find(panel => panel.session === session)!;
    try {
        const before = await read();
        const behavior = before.report.behaviors[0];
        assert.ok(behavior.canPreview && behavior.canEditTrigger && behavior.canEditActions);
        assert.match(document.getText(asRange(behavior.actionRows[0].range)), /change_property\(None, "Width", \/\* first width 🌿 \*\/ r#"278"#\)/u);
        await tool('designer_select', { session, expected_revision: before.revision, node: before.owner.id });
        await tool('designer_view', { session, view: 'behaviors' });
        const view = designer.behaviors;
        await until(async () => (await view.getChildren()).some(row => row.editor.context.token === panel.currentToken));
        const root = (await view.getChildren())[0];
        const actions = await view.getChildren(root);
        const cancellation = new vscode.CancellationTokenSource();
        try {
            const transfer = new vscode.DataTransfer();
            view.handleDrag([actions[0]], transfer, cancellation.token);
            await view.handleDrop(actions[1], transfer, cancellation.token);
        } finally { cancellation.dispose(); }
        const ordered = await read();
        assert.ok(document.getText().indexOf('"338"') < document.getText().indexOf('r#"278"#'));
        await present(ordered.revision, { action: 'preview', behavior: ordered.report.behaviors[0].id });
        const played = await read();
        assert.equal(played.owner.bounds?.width, 278);
        assert.ok((await edit(ordered.revision, { action: 'remove', behavior: behavior.id })).isError);
        await present(played.revision, { action: 'clear_preview' });
        assert.equal((await read()).owner.bounds?.width, 210);
        await undo();

        const accept = await read();
        const current = accept.report.behaviors[0];
        content(await edit(accept.revision, { action: 'set_action', behavior: current.id, action_node: current.actionRows[0].id,
            item: { ChangeProperty: { target: null, property: 'Width', value: '295' } } }));
        assert.equal(document.getText(), original.replace('r#"278"#', '"295"'));
        await undo();

        const notify = await read('Notify');
        assert.ok(notify.report.canAttach);
        content(await edit(notify.revision, { action: 'upsert', model: {
            owner: notify.owner.id,
            trigger: { Data: { binding: 'IsReady', comparison: 'Equal', value: 'True', revert_on_false: true } },
            actions: [{ AddClass: { target: null, class: 'ready' } }],
        } }));
        const attached = await read('Notify');
        assert.ok('Data' in attached.report.behaviors[0].trigger);
        assert.match(document.getText(), /ferroui_view::behavior::data\("IsReady", ferroui_view::behavior::Comparison::Equal/);
        assert.doesNotMatch(document.getText(), /(?<!\r)\n/);
        await undo();
    } finally {
        panel.panel.dispose();
        await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === name));
        await vscode.workspace.fs.delete(uri);
    }
    console.log('Mapped Rust Behaviors dock, UTF-16 ranges, action drag, native preview, edit and exact Undo passed');
}
