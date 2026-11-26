import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode, Range } from '../src/protocol';
import type { StateTransitionPosition } from '../src/stateTransitionProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export const reviewStateSource = `// Customer review 🪴
fn review() {
    ferroui_view::rsx! {
        StackPanel {
            visual_states: [
                ferroui_view::visual_state_group("Review")
                    .state(ferroui_view::visual_state("Idle")
                        .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 120.0))
                    .state(ferroui_view::visual_state("Ready")
                        .set_on("Card", ferroui_base::layout::Layoutable::width_property(), 260.0))
                    .transition(None, "Ready", ferroui_view::ms(350), "LinearEasing")
                    .transition(Some("Idle"), "Ready", ferroui_view::ms(/* keep */ 640u64), "CubicEaseIn")
                    .transition(Some("Ready"), "Idle", ferroui_view::ms(370), "LinearEasing"),
            ],
            Border { name: "Card", width: 100.0, height: 120.0, background: "#234976" }
        }
    }
}
`.replaceAll('\n', '\r\n');

export async function verifyRsxStates({ invoke, content }: McpFixture): Promise<void> {
    const source = reviewStateSource;
    const path = 'review_states.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    assert.equal(document.eol, vscode.EndOfLine.CRLF);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const outline = async () => {
        content(await invoke('designer_wait', { session }));
        return content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    };
    let frame = await outline();
    const query = await invoke('designer_studio_query', { session, tool: 'designer_states', arguments: {} });
    const { groups: [group], sources: [mapping] } = content<{ groups: Array<{ owner: number; name: string;
        states: Array<{ name: string; setters: Array<{ value: string }> }>;
        transitions: Array<{ from: string | null; to: string; duration: number; easing: { kind: string; value?: string } }>;
    }>; sources: Array<{ range: Range; transitions: Array<{ range: Range }> }> }>(query);
    assert.equal(group.name, 'Review');
    assert.match(document.getText(asRange(mapping.range)), /^ferroui_view::visual_state_group\(/);
    assert.match(document.getText(asRange(mapping.transitions[1].range)), /^transition\(Some\("Idle"\)/);
    const width = async () => {
        const node = frame.nodes.find(node => node.name === 'Card')!.id;
        const { properties } = content<{ properties: Array<{ name: string; effective: string }> }>(
            await invoke('designer_inspect', { session, node }));
        return Number(properties.find(property => property.name === 'Width')?.effective);
    };
    const control = async (arguments_: object) => {
        const result = content<StateTransitionPosition>(await invoke('designer_transition_control', {
            session, expected_revision: frame.revision, arguments: arguments_,
        }));
        frame = await outline();
        return result;
    };
    const open = (from: string | null, to: string, time: number) =>
        control({ action: 'open', owner: group.owner, group: 'Review', to, time, ...(from !== null ? { from } : {}) });
    assert.equal((await open(null, 'Ready', .175)).duration, .35);
    assert.equal(await width(), 180);
    assert.equal((await open('Ready', 'Idle', .185)).duration, .37);
    assert.equal(await width(), 190);
    const forward = await open('Idle', 'Ready', .32);
    assert.equal(forward.duration, .64);
    assert.ok(forward.sourceRange);
    assert.equal(document.getText(asRange(forward.sourceRange)), document.getText(asRange(mapping.transitions[1].range)));
    assert.equal(await width(), 137.5);
    await control({ action: 'reveal' });
    assert.equal(vscode.window.activeTextEditor?.document.uri.toString(), uri.toString());
    await control({ action: 'stop' });
    assert.equal(await width(), 100);
    group.states[1].setters[0].value = '280';
    group.transitions[1].duration = .824;
    group.transitions[1].easing = { kind: 'Linear' };
    const edit = (expected_revision: number) => invoke('designer_studio_edit', { session,
        expected_revision, tool: 'designer_state', arguments: { action: 'upsert', model: group },
    });
    const retired = frame.revision;
    content(await edit(retired));
    assert.equal(document.getText(), source.replace('260.0', '280.0').replace('640u64', '824u64')
        .replace('CubicEaseIn', 'LinearEasing'));
    assert.ok((await edit(retired)).isError);
    frame = await outline();
    assert.equal((await open('Idle', 'Ready', .412)).duration, .824);
    assert.equal(await width(), 200);
    await control({ action: 'stop' });
    const version = document.version;
    content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
    await until(() => document.version !== version);
    assert.equal(document.getText(), source);
    await outline();
    const model = { owner: group.owner, name: 'Highlight', states: [
        { name: 'Normal', setters: [] },
        { name: 'Ready', setters: [{ target: 'Card', property: 'Width', value: '160' }] },
    ], transitions: [{ from: 'Normal', to: 'Ready', duration: .23, easing: { kind: 'Linear' } }] };
    const statesEdit = async (arguments_: object) => {
        frame = await outline();
        content(await invoke('designer_studio_edit', {
            session, expected_revision: frame.revision, tool: 'designer_state', arguments: arguments_,
        }));
        frame = await outline();
    };
    await statesEdit({ action: 'upsert', model });
    const authored = document.getText();
    assert.match(authored, /::ferroui_view::visual_state_group\("Highlight"\)/);
    assert.ok(authored.includes('.set_on("Card", ::ferroui_base::layout::Layoutable::width_property(), 160.0)'));
    assert.equal(await width(), 100, 'The accepted base width remains visible before playing a new group');
    const route = await control({ action: 'open', owner: model.owner, group: model.name, from: 'Normal', to: 'Ready', time: .115 });
    assert.equal(route.duration, .23);
    assert.equal(await width(), 130);
    await control({ action: 'stop' });
    model.states[1].setters = [];
    model.transitions = [];
    await statesEdit({ action: 'upsert', model });
    assert.ok(!document.getText().includes('160.0'));
    assert.ok(!document.getText().includes('ms(230)'));
    await statesEdit({ action: 'remove', owner: model.owner, group: model.name });
    assert.equal(document.getText(), source);
    const beforeUndo = document.version;
    content(await invoke('designer_history', { session, expected_version: beforeUndo, action: 'undo' }));
    await until(() => document.version !== beforeUndo);
    assert.match(document.getText(), /visual_state_group\("Highlight"\)/);
    assert.ok(!document.getText().includes('ms(230)'));
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === 'Design · review_states.rs' || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    await vscode.workspace.fs.delete(uri, { useTrash: false });
    console.log('VS Code and MCP sampled both mapped Rust directions and the default route, authored and removed state groups, setters and routes, rejected a retired frame, and preserved CRLF and undo');
}
