import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

const xaml = `<Canvas xmlns="https://github.com/ferroui" Width="520" Height="300" UseLayoutRounding="False">
  <Border Name="Overview" Width="116" Height="76" Canvas.Left="20.5" Canvas.Top="24" Margin="4" />
  <!-- Keep this 🍃 with the grouped cards -->
  <Border Name="Notes" Width="142" Height="62" Canvas.Right="40" Canvas.Bottom="110" />
  <Border Name="Actions" Width="78" Height="40" Canvas.Right="16" Canvas.Bottom="20" />
</Canvas>`;
const rust = [
    'fn heading() -> Element { rsx! { TextBlock { text: "🍃 Another invocation" } } }',
    'fn view() -> Element {',
    '    rsx! { StackPanel { name: "Overview",',
    '        Button { name: "Notes", content: caption(), on_click: |_, _| show_notes() }',
    '        // Keep the controls in source order',
    '        Button { name: "Actions", content: "More actions" }',
    '        for item in items { TextBlock { text: item.name } }',
    '    } }',
    '}',
    '',
].join('\r\n');

export async function verifyWrapping({ invoke, content }: McpFixture): Promise<void> {
    const open = async (path: string, source: string, line = 0) => {
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        const { session } = content<{ session: number }>(await invoke('designer_open', { path, line, character: 10 }));
        const outline = async () => content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const inspect = async (...names: string[]) => {
            const tree = await outline();
            return { ...tree, selection: names.map(name => tree.nodes.find(node => node.name === name)!.id) };
        };
        const plan = (selection: number[], container?: string) => invoke('designer_studio_query', {
            session, tool: 'designer_wrap_selection', selection, arguments: container ? { container } : {},
        });
        const apply = (revision: number, selection: number[], container: string, model: string) => invoke('designer_studio_edit', {
            session, expected_revision: revision, tool: 'designer_wrap_selection_edit', selection,
            arguments: { container, expected_model: model },
        });
        const history = async (action: 'undo' | 'redo') => {
            const version = document.version;
            content(await invoke('designer_history', { session, expected_version: version, action }));
            await until(() => document.version === version + 1);
        };
        const selected = async () => content<{ selected: number }>(await invoke('designer_document', { session })).selected;
        return { document, outline, inspect, plan, apply, history, selected };
    };
    const view = await open('layout-wrap.xaml', xaml);
    const before = await view.inspect('Notes', 'Overview');
    const options = content<{ choices: { container: string; reason: string | null }[] }>(await view.plan(before.selection));
    assert.equal(options.choices.find(choice => choice.container === 'canvas')?.reason, null);
    assert.match(options.choices.find(choice => choice.container === 'border')?.reason ?? '', /one child/);
    const separated = await view.inspect('Overview', 'Actions');
    assert.match((await view.plan(separated.selection, 'canvas')).content[0]?.text ?? '', /consecutive/);
    const draft = content<{ model: string; layout: { children: number[] } }>(await view.plan(before.selection, 'canvas'));
    assert.deepEqual(draft.layout.children, [...before.selection].reverse());
    assert.equal(view.document.getText(), xaml);
    assert.ok((await view.apply(before.revision, separated.selection, 'canvas', draft.model)).isError);
    const single = await view.inspect('Overview');
    assert.match((await view.apply(before.revision, single.selection, 'canvas', draft.model)).content[0]?.text ?? '', /model changed/);
    const fresh = content<{ model: string }>(await view.plan(before.selection, 'canvas'));
    const version = view.document.version;
    content(await view.apply(before.revision, before.selection, 'canvas', fresh.model));
    assert.equal(view.document.version, version + 1);
    assert.match(view.document.getText(), /<Canvas Canvas.Left="20.5" Canvas.Top="24" Width="459.5" Height="166">/);
    assert.match(view.document.getText(), /    <!-- Keep this 🍃 with the grouped cards -->/);
    const after = await view.inspect('Notes', 'Overview');
    const selectedId = await view.selected();
    const wrapper = after.nodes.find(node => node.id === selectedId);
    assert.equal(wrapper?.label, 'Canvas');
    for (const name of ['Overview', 'Notes', 'Actions']) {
        assert.deepEqual(after.nodes.find(node => node.name === name)?.bounds, before.nodes.find(node => node.name === name)?.bounds);
    }
    assert.ok(after.nodes.filter(node => ['Overview', 'Notes'].includes(node.name ?? '')).every(node => node.parent === wrapper?.id));
    assert.ok((await view.apply(after.revision, after.selection, 'canvas', fresh.model)).isError);
    const wrapped = view.document.getText();
    await view.history('undo');
    assert.equal(view.document.getText(), xaml);
    await view.history('redo');
    assert.equal(view.document.getText(), wrapped);

    const rsx = await open('layout-wrap.rs', rust, 2);
    const rustBefore = await rsx.inspect('Actions', 'Notes');
    const mapped = content<{ model: string }>(await rsx.plan(rustBefore.selection, 'horizontal_stack'));
    content(await rsx.apply(rustBefore.revision, rustBefore.selection, 'horizontal_stack', mapped.model));
    const rusty = rsx.document.getText();
    assert.ok(rusty.includes('orientation: ::ferroui_base::layout::Orientation::Horizontal,'));
    for (const part of ['"🍃 Another invocation"', 'content: caption(), on_click: |_, _| show_notes()', '// Keep the controls in source order',
        '        for item in items { TextBlock { text: item.name } }']) assert.ok(rusty.includes(part), part);
    assert.ok(!rusty.replaceAll('\r\n', '').includes('\n'));
    await rsx.history('undo');
    assert.equal(rsx.document.getText(), rust);

    const interrupted = rust.replace('        // Keep the controls in source order', '        for extra in extras { TextBlock { text: extra.name } }');
    const split = await open('layout-wrap-dynamic.rs', interrupted, 2);
    const splitNodes = await split.inspect('Notes', 'Actions');
    assert.match((await split.plan(splitNodes.selection, 'vertical_stack')).content[0]?.text ?? '', /dynamic Rust child/);
    assert.equal(split.document.getText(), interrupted);
    console.log('VS Code wrapped measured XAML and mapped Rust siblings in one edit, preserved bounds and trivia, and rejected stale and interrupted runs');
}
