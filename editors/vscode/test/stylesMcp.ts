import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { PaletteSheet, StyleCatalog, StyleDefinition } from '../src/stylesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyStylesMcp({ invoke, content }: McpFixture): Promise<void> {
    const path = 'StylePalette.xaml';
    const document = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, path));
    const original = document.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const wait = async () => {
        const snapshot = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(snapshot.status, 'Ready');
        return snapshot.revision;
    };
    const read = async <T>(tool: string, args: object = {}) => content<T>(await invoke('designer_studio_query', { session, tool, arguments: args }));
    const edit = (tool: string, revision: number, args: object) => invoke('designer_studio_edit', { session, expected_revision: revision, tool, arguments: args });
    const undo = async () => {
        await vscode.window.showTextDocument(document, { viewColumn: vscode.ViewColumn.One });
        await vscode.commands.executeCommand('undo');
        assert.equal(document.getText(), original);
        return wait();
    };
    let revision = await wait();
    content(await invoke('designer_view', { session, view: 'styles' }));
    const catalog = await read<StyleCatalog>('designer_style_catalog');
    const root = catalog.palette_owner!;
    assert.ok(catalog.style_hosts.some(host => host.node === root));
    const style = catalog.definitions.find(item => item.name === 'Border.card');
    const theme = catalog.definitions.find(item => item.name === 'PrimaryAction');
    assert.ok(style && theme);
    const definition = await read<StyleDefinition>('designer_style_definition', { node: style.node });
    assert.equal(definition.identity.value, 'Border.card');
    const padding = definition.setters.find(setter => setter.property === 'Padding');
    assert.ok(padding);
    assert.equal(document.getText(asRange(padding.range)), '26');
    const child = await read<StyleDefinition>('designer_style_definition', { node: theme.node });
    assert.ok(child.base_choices?.some(choice => choice.name === 'WorkspaceAction' && choice.node === child.base));

    const palette = await read<PaletteSheet>('designer_palette', { owner: root });
    const accent = palette.comparison.find(row => row.key === 'ActionSurface');
    assert.equal(accent?.light?.value, '#FF295DC0');
    assert.equal(accent?.dark?.value, '#FF537BE1');
    assert.notEqual(accent?.light?.node, accent?.dark?.node);
    const live = async (dark: boolean) => {
        content(await invoke('designer_view', { session, dark }));
        await wait();
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const button = nodes.find(node => node.name === 'ApplyPalette');
        assert.ok(button);
        return (await read<{ effective: string }>('designer_property_trace', { node: button.id, property: 'Background' })).effective;
    };
    assert.equal(await live(false), '#FF295DC0');
    assert.equal(await live(true), '#FF537BE1');
    assert.equal(document.getText(), original);
    revision = await wait();
    content(await edit('designer_palette_edit', revision, {
        action: 'set', node: accent?.dark?.node, color: '#FF40AB94',
    }));
    assert.equal(await live(true), '#FF40AB94');
    assert.equal(await live(false), '#FF295DC0');
    assert.match(document.getText(), /Color="#FF40AB94"/);
    assert.match(document.getText(), /<!-- Each theme keeps its own authored surface\. -->/);
    assert.ok((await edit('designer_palette_edit', revision, { action: 'set', node: accent?.dark?.node, color: 'Red' })).isError);
    revision = await undo();

    content(await edit('designer_style_edit', revision, { action: 'setter', owner: style.node, property: 'Padding', value: '36,20' }));
    await wait();
    assert.match(document.getText(), /Property="Padding" Value="36,20"/);
    revision = await undo();
    content(await edit('designer_style_edit', revision, { action: 'create', owner: root, selector: 'Border#ThemeCard' }));
    await wait();
    assert.match(document.getText(), /Selector="Border#ThemeCard"/);
    await undo();
    const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
    const button = nodes.find(node => node.name === 'ApplyPalette');
    assert.ok(button);
    const readPlan = async () => content<{ model: string }>(await invoke('designer_studio_query', {
        session, tool: 'designer_selection_layout', selection: [button.id], arguments: { operation: 'reset_alignment' },
    }));
    const plan = await readPlan();
    content(await invoke('designer_studio_query', { session, tool: 'designer_style_catalog', selection: [root], arguments: { target: root } }));
    assert.equal((await readPlan()).model, plan.model, 'reading the dock keeps independent plans current');
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => tab.label === path || tab.label === `Design · ${path}`);
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code edited scoped styles and palette colors, switched native Light and Dark, and preserved source and undo');
}
