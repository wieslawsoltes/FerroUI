import { strict as assert } from 'node:assert';
import { realpath } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { OutlineNode } from '../src/protocol';
import type { DesignTraceSource, PropertyTrace } from '../src/propertyTrace';
import { revision } from '../src/reloadProcess';
import type { LocatedStyleSource, PaletteSheet, StyleCatalog, StyleDefinition, StyleSource } from '../src/stylesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';

export async function verifyIncludedStylesMcp({ invoke, content }: McpFixture): Promise<void> {
    const consumerPath = 'IncludedStylePalette.xaml';
    const stylePath = 'Styling/Shared Styles.xaml';
    const darkPath = 'Styling/Colors/Dark.xaml';
    const root = vscode.workspace.workspaceFolders![0].uri;
    const document = (path: string) => vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, path));
    const [consumer, stylesheet, dark] = await Promise.all([consumerPath, stylePath, darkPath].map(document));
    const original = consumer.getText();
    const originalStyle = stylesheet.getText();
    const originalDark = dark.getText();
    const source = await vscode.window.showTextDocument(stylesheet);
    const offset = originalStyle.indexOf('#FF476188');
    assert.ok(offset >= 0);
    assert.ok(await source.edit(edit => edit.replace(new vscode.Range(stylesheet.positionAt(offset), stylesheet.positionAt(offset + 9)), '#FF3D567D')));
    const dirtyStyle = stylesheet.getText();
    const { session } = content<{ session: number }>(await invoke('designer_open', { path: consumerPath }));
    const frame = async (session: number) => {
        const result = content<{ status: string; revision: number }>(await invoke('designer_wait', { session }));
        assert.equal(result.status, 'Ready');
        return result.revision;
    };
    const read = async <T>(session: number, tool: string, arguments_: object = {}) =>
        content<T>(await invoke('designer_studio_query', { session, tool, arguments: arguments_ }));
    const edit = async (session: number, tool: string, arguments_: object) => content(await invoke('designer_studio_edit', {
        session, expected_revision: await frame(session), tool, arguments: arguments_,
    }));
    const navigate = async (source: StyleSource) => invoke('designer_studio_present', {
        session, expected_revision: await frame(session), tool: 'designer_open_style_source', arguments: { source },
    });
    const open = async (source: StyleSource) => {
        const opened = content<{ opened_definition: StyleSource; opened_session: number }>(await navigate(source));
        assert.deepEqual(opened.opened_definition, source);
        assert.ok(opened.opened_session && opened.opened_session !== session);
        return opened.opened_session;
    };
    const undo = async (session: number, document: vscode.TextDocument, expected: string) => {
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), expected);
        await frame(session);
    };
    const trace = async (name: string) => {
        await frame(session);
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        return read<PropertyTrace<DesignTraceSource>>(session, 'designer_property_trace', { node, property: 'Background' });
    };
    const effective = async (name: string) => (await trace(name)).effective;
    const theme = async (dark: boolean) => {
        content(await invoke('designer_view', { session, dark }));
        await frame(session);
    };
    const designColumn = (file: string) => vscode.window.tabGroups.all
        .find(group => group.tabs.some(tab => tab.label === `Design · ${file}`))?.viewColumn;

    await theme(false);
    const catalog = await read<StyleCatalog>(session, 'designer_style_catalog');
    const style = catalog.included_sources?.find(choice => choice.label === 'Border.account')?.source;
    assert.ok(style && style.uri === vscode.Uri.file(await realpath(stylesheet.uri.fsPath)).toString());
    assert.ok(catalog.included_sources?.some(choice => choice.source.kind === 'control_theme' && choice.label === 'AccountAction'));
    assert.ok(catalog.included_sources?.some(choice => choice.source.kind === 'palette' && choice.source.uri.endsWith('/Colors/Dark.xaml')));
    const palette = () => read<PaletteSheet>(session, 'designer_palette', { owner: catalog.palette_owner });
    const sheet = await palette();
    const accent = sheet.comparison.find(color => color.key === 'AccountAccent');
    assert.equal(accent?.light?.value, '#FF3468AC');
    assert.equal(accent?.dark?.value, '#FF8E4AA3');
    assert.equal(accent?.dark?.node, undefined, 'a foreign node ID must not be usable in the consumer');
    assert.equal(sheet.comparison.find(color => color.key === 'AccountStatus')?.dark?.value, '#FF526579');
    assert.equal(sheet.comparison.find(color => color.key === 'AccountStatus')?.dark?.source, undefined, 'the local brush shadows the include');
    const activeStyle = await trace('SharedStyleCard');
    assert.equal(activeStyle.effective, '#FF3D567D', 'the include graph uses the unsaved style buffer');
    const winner = activeStyle.contributors.find(entry => entry.state === 'effective')?.source;
    assert.ok(winner?.included);
    assert.equal(winner.included.uri, style.uri);
    assert.equal(stylesheet.getText(asRange(winner.included.range)), '#FF3D567D');
    assert.equal(winner.sourceRevision, revision(Buffer.from(dirtyStyle)));
    const activeResource = await trace('SharedAccentCard');
    assert.equal(activeResource.effective, '#FF3468AC');
    const resource = activeResource.resource?.included;
    assert.ok(resource);
    const values = await document('Styling/Shared Palette.xaml');
    assert.equal(resource.uri, vscode.Uri.file(await realpath(values.uri.fsPath)).toString());
    assert.match(values.lineAt(resource.range.start.line).text, /x:Key="AccountAccent"/);
    const styleSession = await open(style);
    assert.ok(designColumn(consumerPath));
    assert.equal(designColumn('Shared Styles.xaml'), designColumn(consumerPath));
    const styleLocation = (await read<LocatedStyleSource>(styleSession, 'designer_style_source', { source: style })).editor;
    assert.equal(styleLocation.page, 'styles');
    const definition = await read<StyleDefinition>(styleSession, 'designer_style_definition', { node: styleLocation.node });
    assert.equal(definition.setters.find(setter => setter.property === 'Background')?.value, '#FF3D567D');
    await edit(styleSession, 'designer_style_edit', { action: 'setter', owner: styleLocation.node, property: 'Background', value: '#FF3C705A' });
    assert.equal(stylesheet.getText(), dirtyStyle.replace('#FF3D567D', '#FF3C705A'));
    assert.equal(await effective('SharedStyleCard'), '#FF3C705A');
    const staleResult = await navigate(style);
    assert.ok(staleResult.isError);
    assert.match(staleResult.content[0]?.text ?? '', /included source changed/i);
    assert.equal(consumer.getText(), original);
    await undo(styleSession, stylesheet, dirtyStyle);
    await undo(styleSession, stylesheet, originalStyle);
    assert.equal(await effective('SharedStyleCard'), '#FF476188');

    await theme(true);
    const color = (await palette()).comparison.find(row => row.key === 'AccountAccent')?.dark?.source;
    assert.ok(color && color.uri === vscode.Uri.file(await realpath(dark.uri.fsPath)).toString());
    const paletteSession = await open(color);
    const located = await read<LocatedStyleSource>(paletteSession, 'designer_style_source', { source: color });
    assert.equal(located.editor.page, 'palette');
    assert.equal(located.editor.layer, 'shared');
    assert.ok(located.editor.color);
    assert.equal(dark.getText(asRange(located.range)), '#FF8E4AA3', 'the emoji before the declaration must not shift the UTF-16 range');
    await edit(paletteSession, 'designer_palette_edit', { action: 'set', node: located.editor.color, color: '#FF347A71' });
    assert.equal(dark.getText(), originalDark.replace('#FF8E4AA3', '#FF347A71'));
    assert.equal(await effective('SharedAccentCard'), '#FF347A71');
    await theme(false);
    assert.equal(await effective('SharedAccentCard'), '#FF3468AC');
    await undo(paletteSession, dark, originalDark);
    await theme(true);
    assert.equal(await effective('SharedAccentCard'), '#FF8E4AA3');
    assert.equal(consumer.getText(), original);
    const labels = new Set([consumerPath, 'Shared Styles.xaml', 'Dark.xaml']);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => labels.has(tab.label.replace(/^Design · /, '')));
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code opened checked included styles and a nested palette color from MCP. Unsaved edits and Undo updated the consuming preview in both themes.');
}
