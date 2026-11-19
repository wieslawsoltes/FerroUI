import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { SpacingCatalog, SpacingChange, SpacingDraft, SpacingHandle } from '../src/spacingProtocol';
import type { McpFixture } from './mcp';
import { until } from './wait';

const xaml = `<StackPanel xmlns="https://github.com/ferroui" Name="Flow" Spacing="12" RenderTransform="scale(2)" UseLayoutRounding="False">
  <Border Name="Card" Padding="8,10,12,14" Margin="2,4,6,8" Height="70"><TextBlock Text="Preview"/></Border>
  <Border Name="Second" Height="60" Margin="{Binding CardInsets}" />
</StackPanel>`;
const rust = [
    'fn header() -> Element { rsx! { TextBlock { text: "Keep this invocation" } } }',
    'fn view() -> Element {',
    '    rsx! { StackPanel { name: "Flow", spacing: 12.0,',
    '        Border { name: "Card", padding: (8.0, /* 🪴 keep this note */ 10.0, 12.0, 14.0), margin: insets(),',
    '            TextBlock { text: "Draft" }',
    '        }',
    '        for item in items { TextBlock { text: item.name } }',
    '    } }',
    '}',
    '',
].join('\r\n');

export async function verifySpacing({ invoke, content }: McpFixture): Promise<void> {
    const open = async (path: string, source: string, line: number) => {
        const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        const { session } = content<{ session: number }>(await invoke('designer_open', { path, line, character: 13 }));
        const outline = async () => content<{ revision: number; nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const query = (tool: string, selection: number[], args: object) =>
            invoke('designer_studio_query', { session, tool, selection, arguments: args });
        const read = async (...names: string[]) => {
            const { revision, nodes } = await outline();
            const selection = names.map(name => nodes.find(node => node.name === name)!.id);
            const catalog = content<SpacingCatalog>(await query('designer_spacing_helpers', selection, {}));
            return { ...catalog, revision, selection };
        };
        type Model = Awaited<ReturnType<typeof read>>;
        const edit = (model: Model, args: SpacingChange, revision = model.revision) => invoke('designer_studio_edit', {
            session, expected_revision: revision, tool: 'designer_spacing_edit', selection: model.selection, arguments: args,
        });
        const draft = (model: Model, args: SpacingChange) => query('designer_spacing_draft', model.selection, args);
        const undo = async () => {
            const version = document.version;
            content(await invoke('designer_history', { session, expected_version: version, action: 'undo' }));
            await until(() => document.version === version + 1);
            assert.equal(document.getText(), source);
        };
        const toggle = async (show: boolean) => content<{ spacingHelpers: boolean }>(await invoke('designer_canvas', {
            session, spacing_helpers: show,
        }));
        return { session, document, outline, read, edit, draft, undo, toggle };
    };

    const view = await open('measured-spacing.xaml', xaml, 0);
    const toolset = content<{ tools: { name: string }[]; edits: { name: string }[] }>(
        await invoke('designer_studio_tools', { session: view.session }));
    assert.ok(toolset.tools.some(tool => tool.name === 'designer_spacing_helpers'));
    assert.ok(toolset.tools.some(tool => tool.name === 'designer_spacing_draft'));
    assert.ok(toolset.edits.some(tool => tool.name === 'designer_spacing_edit'));
    assert.equal((await view.toggle(true)).spacingHelpers, true);
    assert.equal((await view.toggle(false)).spacingHelpers, false);
    const initial = await view.read('Card');
    const gap = handle(initial, 'Spacing', 'Flow');
    const padding = handle(initial, 'Padding left', 'Card');
    assert.equal(padding.value, 8);
    assert.ok(initial.bands.some(band => band.padding && band.filled));
    const originalVersion = view.document.version;

    const drag = { model: initial.model, handle: gap.id, adjustment: { delta: { x: gap.axis.x * 8, y: gap.axis.y * 8 } } };
    const preview = content<SpacingDraft>(await view.draft(initial, drag));
    assert.equal(preview.handle.value, 20);
    assert.ok(preview.changed && preview.handle.position.y > gap.position.y);
    assert.equal(view.document.version, originalVersion);
    content(await view.edit(initial, drag));
    assert.equal(view.document.version, originalVersion + 1);
    assert.equal(view.document.getText(), xaml.replace('Spacing="12"', 'Spacing="20"'));
    await view.undo();

    const current = await view.read('Card');
    assert.notEqual(current.model, initial.model);
    assert.ok((await view.edit(initial, drag, current.revision)).isError);
    const noChange = view.document.version;
    content(await view.edit(current, exact(current, 'Padding left', 'Card', 8)));
    assert.equal(view.document.version, noChange);
    assert.equal((await view.outline()).revision, current.revision);
    assert.ok((await view.edit(current, exact(current, 'Padding left', 'Card', -1))).isError);
    content(await view.edit(current, exact(current, 'Margin right', 'Card', -3.125)));
    assert.equal(view.document.getText(), xaml.replace('Margin="2,4,6,8"', 'Margin="2,4,-3.125,8"'));
    await view.undo();

    const group = await view.read('Card', 'Second');
    assert.ok(group.handles.some(handle => handle.ownerLabel === 'Card' && handle.property === 'Margin'));
    const bound = handle(group, 'Margin right', 'Second');
    assert.ok(bound.disabledReason);
    assert.ok((await view.draft(group, exact(group, 'Margin right', 'Second', 8))).isError);
    assert.ok((await view.edit(group, exact(group, 'Margin right', 'Second', 8))).isError);
    await view.read('Card');
    assert.ok((await view.edit(group, exact(group, 'Spacing', 'Flow', 20))).isError);
    assert.equal(view.document.getText(), xaml);

    const mapped = await open('measured-spacing.rs', rust, 2);
    const model = await mapped.read('Card');
    assert.match(handle(model, 'Margin left', 'Card').disabledReason ?? '', /Rust expression/);
    const changed = exact(model, 'Padding top', 'Card', 17.5);
    assert.equal(content<SpacingDraft>(await mapped.draft(model, changed)).handle.value, 17.5);
    assert.equal(mapped.document.getText(), rust);
    content(await mapped.edit(model, changed));
    assert.equal(mapped.document.getText(), rust.replace('*/ 10.0, 12.0, 14.0)', '*/ 17.5, 12.0, 14.0)'));
    await mapped.undo();
    const fresh = await mapped.read('Card');
    content(await mapped.edit(fresh, exact(fresh, 'Spacing', 'Flow', 18.25)));
    assert.equal(mapped.document.getText(), rust.replace('spacing: 12.0', 'spacing: 18.25'));
    await mapped.undo();
    console.log('VS Code and Studio MCP measured transformed gaps and inset sides, guarded stale and protected drafts, and undid XAML and mapped RSX edits');
}

function handle(model: SpacingCatalog, label: string, owner: string): SpacingHandle {
    const found = model.handles.find(handle => handle.label === label && handle.ownerLabel === owner);
    assert.ok(found, `Measure ${label} on ${owner}`);
    return found;
}

function exact(model: SpacingCatalog, label: string, owner: string, value: number): SpacingChange {
    return { model: model.model, handle: handle(model, label, owner).id, adjustment: { value } };
}
