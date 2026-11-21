import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import { applyTextEdits, asRange } from '../src/textDocument';
import type { OutlineNode } from '../src/protocol';
import type { StyleCatalog, StyleDefinition } from '../src/stylesProtocol';
import type { McpFixture } from './mcp';

const directory = 'RustPaletteModule';
const view = `use crate::brand::{ACCENT, KEY};
fn overview() { rsx! { TextBlock { text: "Overview" } }; }
fn detail() {
    rsx! { StackPanel {
        styles: [ferroui_view::style("TextBlock")
            .resource(KEY, ACCENT)
            .set_resource(ferroui_controls::TextBlock::foreground_property(), KEY)],
        TextBlock { name: "Customer", text: "Customer accounts" },
    } };
}`;
const brand = 'use ferroui_base::media::Color as Paint;\r\npub const KEY: &str = "Accent";\r\npub const ACCENT: Paint = /* 🎨 */ Paint::from_rgb(37u8, 74, 112);\r\n';

export async function verifyRsxModuleColorsMcp({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, directory);
    const viewUri = vscode.Uri.joinPath(root, 'review.rs');
    const brandUri = vscode.Uri.joinPath(root, 'brand.rs');
    await vscode.workspace.fs.createDirectory(root);
    const write = (file: string, text: string) => vscode.workspace.fs.writeFile(vscode.Uri.joinPath(root, file), Buffer.from(text));
    await Promise.all([
        write('Cargo.toml', '[package]\nname = "palette-module-review"\nversion = "0.0.0"\nedition = "2021"\n[lib]\npath = "entry.rs"\n'),
        write('entry.rs', 'mod brand; mod review;\n'), write('review.rs', view), write('brand.rs', brand),
    ]);
    const owner = await vscode.workspace.openTextDocument(brandUri);
    const primary = await vscode.workspace.openTextDocument(viewUri);
    const initial = owner.getText();
    const draft = initial.replace('37u8', '45u8');
    const end = owner.positionAt(initial.length);
    await applyTextEdits(owner, owner.version, [{ range: { start: { line: 0, character: 0 }, end }, newText: draft }]);
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: `${directory}/review.rs`, line: 3, character: 4 }));
    const ready = async () => {
        const frame = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(frame.status, 'Ready');
        return frame.revision;
    };
    const query = <T>(tool: string, arguments_: object = {}) => invoke('designer_studio_query', { session, tool, arguments: arguments_ }).then(content<T>);
    const color = async () => {
        await ready();
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === 'Customer');
        assert.ok(node);
        return (await query<{ effective: string }>('designer_property_trace', { node: node.id, property: 'TextElement.Foreground' })).effective;
    };
    assert.equal(await color(), '#FF2D4A70');
    const { definitions } = await query<StyleCatalog>('designer_rsx_styles');
    const style = definitions[0].node;
    const { color_resources: [resource] = [] } = await query<StyleDefinition>('designer_rsx_styles', { node: style });
    assert.ok(resource?.sourceAddress);
    assert.equal(resource.shared_constant, 'ACCENT');
    assert.equal(owner.getText(asRange(resource.range)), 'Paint::from_rgb(45u8, 74, 112)');
    content(await invoke('designer_rsx_style_source', { session, expected_revision: await ready(), address: resource.sourceAddress, range: resource.range }));
    const selection = vscode.window.visibleTextEditors.find(editor => editor.document === owner)?.selection;
    assert.equal(selection && owner.getText(selection), 'Paint::from_rgb(45u8, 74, 112)');
    const request = { session, expected_revision: await ready(), tool: 'designer_rsx_style_edit', arguments: {
        action: 'color_resource', style, resource: resource.node, mode: 'declaration', value: '#FF286240',
    } };
    const result = content<{ edited_source: { uri: string; version: number } }>(await invoke('designer_studio_edit', request));
    assert.equal(result.edited_source.uri, resource.sourceAddress);
    assert.equal(owner.getText(), draft.replace('45u8, 74, 112', '40u8, 98, 64'));
    assert.equal(primary.getText(), view);
    assert.equal(await color(), '#FF286240');
    assert.equal((await invoke('designer_studio_edit', request)).isError, true);

    const path = `${directory}/brand.rs`;
    const undo = async () => {
        const { revision } = content<{ revision: string }>(await invoke('ferroui_workspace_read', { path }));
        content(await invoke('ferroui_workspace_file', { path, expected_revision: revision, action: 'undo' }));
    };
    await undo();
    assert.equal(owner.getText(), draft);
    assert.equal(await color(), '#FF2D4A70');
    await undo();
    assert.equal(owner.getText(), initial);
    assert.equal(await color(), '#FF254A70');
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => ['Design · review.rs', 'review.rs', 'brand.rs'].includes(tab.label));
    await vscode.window.tabGroups.close(tabs);
    await vscode.workspace.fs.delete(root, { recursive: true });
}
