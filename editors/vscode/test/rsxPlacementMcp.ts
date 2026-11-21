import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyRsxPlacement(fixture: McpFixture): Promise<void> {
    await verifyPlacement(fixture, false);
    await verifyPlacement(fixture, true);
}

async function verifyPlacement({ invoke, content }: McpFixture, collection: boolean): Promise<void> {
    const path = collection ? 'review_collection_placement.rs' : 'review_placement.rs';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path);
    const computed = collection ? 'computed_children()' : 'for item in rows()';
    const source = [
        'fn heading() { rsx! { TextBlock { text: "Heading 🧭" } }; }',
        'fn review() { rsx! { StackPanel {',
        '    StackPanel { name: "Origin",',
        '        Button { name: "Moved", content: "Move", /* keep 🦀 */ on_click: save() }',
        '        Border { width(44.0) }',
        '        { computed_origin_child() }',
        '    }',
        '    StackPanel { name: "Destination",',
        ...(collection ? [
            '        children([',
            '            rsx! { TextBlock { name: "Anchor", text("Anchor") } },',
            '            computed_children(),',
            '            rsx! { TextBlock { name: "Tail", text(message()) } },',
            '        ]),',
        ] : [
            '        TextBlock { name: "Anchor", text: "Anchor" }',
            '        for item in rows() { TextBlock { text: item.title } }',
            '        TextBlock { name: "Tail", text: message() }',
        ]),
        '    }',
        '    Border { name: "Computed", { body() } }',
        '} }; }',
        '',
    ].join('\r\n');
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const document = await vscode.workspace.openTextDocument(uri);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const outline = async () => content<{ revision: number; nodes: OutlineNode[] }>(
        await invoke('designer_outline', { session }));
    const undo = async (expected: string) => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
    };
    let frame = await outline();
    content(await invoke('designer_block', { session, expected_revision: frame.revision, index: 1 }));
    frame = await outline();
    const find = (name: string) => {
        const node = frame.nodes.find(node => node.name === name);
        assert.ok(node, `Expected ${name} in the current RSX frame`);
        return node;
    };
    const moved = find('Moved');
    const border = frame.nodes.find(node => node.parent === moved.parent && node.label === 'Border');
    assert.ok(border?.canMove && moved.canMove);
    assert.equal(find('Computed').canInsert, false);
    const request = { session, expected_revision: frame.revision, selection: [border.id, moved.id],
        moving_anchor: moved.id, anchor: find('Anchor').id, position: 'after' };
    content(await invoke('designer_placement_hint', request));
    content(await invoke('designer_place', request));
    const placed = document.getText();
    const first = placed.indexOf('name: "Moved"');
    const second = placed.indexOf('width(44.0)');
    assert.ok(placed.indexOf('name: "Anchor"') < first && first < second);
    assert.ok(second < placed.indexOf(computed));
    assert.match(placed, /\/\* keep 🦀 \*\/ on_click: save\(\)/);
    assert.ok(placed.includes('{ computed_origin_child() }'));
    if (collection) assert.match(placed, /::ferroui_view::rsx! \{ Button \{ name: "Moved"/);
    assert.equal(placed.split('\r\n')[0], source.split('\r\n')[0]);
    frame = await outline();
    const { selection } = content<{ selection: number[] }>(await invoke('designer_wait', { session }));
    const destination = find('Destination');
    const children = frame.nodes.filter(node => node.parent === destination.id);
    assert.deepEqual(selection, [children[2].id, children[1].id]);
    assert.equal((await invoke('designer_place', request)).isError, true);
    const insert = { session, expected_revision: frame.revision, asset: 'TextBlock',
        anchor: find('Tail').id, position: 'before' };
    content(await invoke('designer_placement_hint', insert));
    content(await invoke('designer_place', insert));
    const withAsset = document.getText();
    assert.match(withAsset, /text: "Text",/);
    assert.ok(withAsset.indexOf('text: "Text"') < withAsset.indexOf('name: "Tail"'));
    frame = await outline();
    const active = content<{ selection: number[] }>(await invoke('designer_wait', { session }));
    const inserted = frame.nodes.filter(node => node.parent === find('Destination').id).at(-2);
    assert.ok(inserted);
    assert.deepEqual(active.selection, [inserted.id]);
    await undo(placed);
    await undo(source);
    assert.equal(document.eol, vscode.EndOfLine.CRLF);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab =>
        tab.label === `Design · ${path}` || (tab.input instanceof vscode.TabInputText && tab.input.uri.toString() === uri.toString()));
    await vscode.window.tabGroups.close(tabs);
    await vscode.workspace.fs.delete(uri, { useTrash: false });
    console.log(`Moved an RSX selection into ${collection ? 'an explicit child collection' : 'an authored panel'} and inserted a Toolbox control in VS Code with checked revisions, retained selection, and individual Undo steps`);
}
