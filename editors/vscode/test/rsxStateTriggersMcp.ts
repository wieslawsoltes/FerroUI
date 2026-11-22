import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode, Viewport } from '../src/protocol';
import type { StateTriggerReport } from '../src/studioStateTriggers';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export const stateTriggerSource = `fn approval() {
    ferroui_view::rsx! {
        StackPanel {
            width: 360.0, height: 190.0, spacing: 12.0,
            visual_states: [ferroui_view::visual_state_group("Review")
                .state(ferroui_view::visual_state("Ready")
                    .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 210.0))],
            Button {
                name: "Approve", width: 360.0, height: 44.0,
                // Preserve normal Rust event code 🪴
                on_click: |event| keep_existing_handler(event),
                "Approve"
            }
            Border { name: "Card", width: 96.0, height: 96.0, background: "#254679",
                horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left }
        }
    }
}
`.replaceAll('\n', '\r\n');

export async function verifyRsxStateTriggers({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ApprovalStateTrigger.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(stateTriggerSource));
    const document = await vscode.workspace.openTextDocument(uri);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    try {
        const read = async () => {
            const { viewport } = content<{ viewport: Viewport }>(await invoke('designer_wait', { session }));
            const outline = content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
            const button = outline.nodes.find(node => node.name === 'Approve');
            assert.ok(button?.bounds);
            return { ...outline, viewport, button };
        };
        const base = await read();
        const report = async (source: number) => content<{ stateTriggers: StateTriggerReport }>(
            await invoke('designer_studio_query', { session, tool: 'designer_states', arguments: { event_source: source } })).stateTriggers;
        const choices = await report(base.button.id);
        assert.ok(choices.events.includes('Click'));
        assert.deepEqual(choices.groups, [{ group: 'Review', states: ['Ready'] }]);
        const edit = (frame: typeof base, state: string | null) => invoke('designer_studio_edit', {
            session, expected_revision: frame.revision, tool: 'designer_state', arguments: {
                action: 'set_event_trigger', event_source: frame.button.id, event: 'Click', group: 'Review', state,
            },
        });
        assert.ok((await edit(base, 'Missing')).isError);
        const editable = await read();
        content(await edit(editable, 'Ready'));
        assert.match(document.getText(), /state_trigger\("Click", "Review", "Ready"\)/u);
        assert.match(document.getText(), /on_click: \|event\| keep_existing_handler\(event\)/u);
        assert.ok((await edit(editable, null)).isError);
        const linked = await read();
        const [trigger] = (await report(linked.button.id)).triggers;
        assert.equal(document.getText(asRange(trigger.range)), 'state_trigger("Click", "Review", "Ready")');
        assert.equal(document.eol, vscode.EndOfLine.CRLF);
        content(await invoke('designer_canvas', { session, mode: 'interact', tab: 'preview' }));
        const interactive = await read();
        const { x, y, width, height } = interactive.button.bounds!;
        content(await invoke('designer_canvas_input', { session, expected_revision: interactive.revision,
            input: { kind: 'click', x: (x + width / 2) / interactive.viewport.width, y: (y + height / 2) / interactive.viewport.height } }));
        const rendered = await read();
        const card = rendered.nodes.find(node => node.name === 'Card');
        assert.ok(card?.bounds);
        assert.equal(card.bounds.width, 210, 'The real preview click enters the linked Rust state');
        const version = document.version;
        content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
        await until(() => document.version > version);
        assert.equal(document.getText(), stateTriggerSource);
        console.log('VS Code MCP linked a Rust routed event to a state, dispatched native input, and undid the exact source edit');
    } finally {
        const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
            tab.label === 'Design · ApprovalStateTrigger.rs'
            || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
        await vscode.window.tabGroups.close(tabs);
        await vscode.workspace.fs.delete(uri, { useTrash: false });
    }
}
