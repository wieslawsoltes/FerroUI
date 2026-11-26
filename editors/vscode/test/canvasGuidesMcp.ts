import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CanvasGuidesState } from '../src/canvasGuidesProtocol';
import type { McpFixture } from './mcp';

type Accepted = CanvasGuidesState & { revision: number; selected?: number };

export async function verifyCanvasGuides({ invoke, content }: McpFixture): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const open = async (path: string, source: string) => {
        const uri = vscode.Uri.joinPath(folder, path);
        await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
        const document = await vscode.workspace.openTextDocument(uri);
        const { session } = content<{ session: number }>(await invoke('designer_open', { path, line: 0 }));
        return { session, document };
    };
    const read = async (session: number) => content<Accepted>(await invoke('designer_canvas_guides', { session }));
    const edit = (guide: Accepted, command: object) => invoke('designer_canvas_guides_edit', {
        session: guide.frame.session, expected_revision: guide.revision, expected_epoch: guide.epoch, edit: command,
    });
    const xaml = `<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Border.Resources>
    <DataTemplate x:Key="Card"><TextBlock Text="Guide the template"/></DataTemplate>
  </Border.Resources>
  <TextBlock Text="Guide the document"/>
</Border>`;
    const { session, document } = await open('canvas-guides.xaml', xaml);
    const version = document.version;
    const initial = await read(session);
    assert.equal(initial.scope, 'document');
    const root = content<Accepted>(await edit(initial, { action: 'apply', rulers: true, guides: true,
        lines: [{ axis: 'Horizontal', position: 73.5 }, { axis: 'Vertical', position: -18.125 }] }));
    assert.equal(root.revision, initial.revision);
    assert.deepEqual(root.settings.guides.map(guide => guide.position), [73.5, -18.125]);
    assert.ok((await edit(initial, { action: 'clear' })).isError);
    assert.ok((await edit(root, { action: 'move', id: 999, position: 1 })).isError);
    assert.ok((await edit(root, { action: 'apply', rulers: false, guides: false,
        lines: [root.settings.guides[0], root.settings.guides[0]] })).isError);
    assert.deepEqual((await read(session)).settings, root.settings);

    const library = content<{ components: { node: number; key: string }[] }>(await invoke('designer_studio_query', {
        session, tool: 'designer_components', arguments: {},
    }));
    const node = library.components.find(component => component.key === 'Card')!.node;
    const show = async (action: 'enter' | 'exit') => {
        const frame = await read(session);
        content(await invoke('designer_studio_present', { session, expected_revision: frame.revision,
            tool: 'designer_definition', arguments: action === 'enter' ? { action, node } : { action } }));
        return read(session);
    };
    const definition = await show('enter');
    assert.equal(definition.scope, 'definition');
    assert.deepEqual(definition.settings.guides, []);
    const definitionEdit = content<Accepted>(await edit(definition, { action: 'add', axis: 'Vertical', position: 41.25 }));
    assert.deepEqual((await show('exit')).settings, root.settings);
    assert.deepEqual((await show('enter')).settings, definitionEdit.settings);
    assert.ok((await edit(root, { action: 'clear' })).isError);
    await show('exit');
    assert.equal(document.version, version);
    assert.equal(document.getText(), xaml);
    assert.equal(document.isDirty, false);

    const rsx = 'fn first() { rsx! { TextBlock { text: "First" } } }\r\nfn second() { rsx! { TextBlock { text: "Second" } } }\r\n';
    const rust = await open('canvas-guides.rs', rsx);
    const first = await read(rust.session);
    assert.equal(first.scope, 'rsx');
    const firstEdit = content<Accepted>(await edit(first, { action: 'add', axis: 'Horizontal', position: 23.75 }));
    const selectBlock = async (index: number) => {
        const frame = await read(rust.session);
        content(await invoke('designer_block', { session: rust.session, expected_revision: frame.revision, index }));
        return read(rust.session);
    };
    assert.deepEqual((await selectBlock(1)).settings.guides, []);
    assert.deepEqual((await selectBlock(0)).settings, firstEdit.settings);
    const sourceVersion = rust.document.version;
    content(await invoke('designer_source', { session: rust.session, expected_version: sourceVersion, source: '// 🪴\r\n' + rsx }));
    const rebased = await read(rust.session);
    assert.deepEqual(rebased.settings, firstEdit.settings);
    assert.ok((await edit(firstEdit, { action: 'clear' })).isError);
    content(await invoke('designer_history', { session: rust.session, expected_version: rust.document.version, action: 'undo' }));
    assert.equal(rust.document.getText(), rsx);
    assert.deepEqual((await read(rust.session)).settings, firstEdit.settings);
    console.log('VS Code MCP validated scoped guides, stale epochs, atomic exact positions, and RSX source undo');
}
