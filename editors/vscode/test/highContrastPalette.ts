import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { CanvasTheme, OutlineNode, Viewport } from '../src/protocol';
import type { LocatedStyleSource, PaletteSheet, StyleCatalog } from '../src/stylesProtocol';
import { asRange } from '../src/textDocument';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyHighContrastPalette({ invoke, content }: McpFixture): Promise<void> {
    const path = 'HighContrastPalette.xaml';
    const contrastPath = 'Styling/Colors/High Contrast.xaml';
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const documents = await Promise.all([path, 'Styling/Accessible Palette.xaml', contrastPath].map(file =>
        vscode.workspace.openTextDocument(vscode.Uri.joinPath(workspace, file))));
    const [view, , contrast] = documents;
    const originals = documents.map(document => document.getText());
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const frame = async (target = session) => {
        const result = content<{ status: string; revision: number; viewport: Viewport; available_themes: CanvasTheme[] }>(
            await invoke('designer_wait', { session: target }));
        assert.equal(result.status, 'Ready');
        return result;
    };
    const read = async <T>(tool: string, arguments_: object = {}, target = session) =>
        content<T>(await invoke('designer_studio_query', { session: target, tool, arguments: arguments_ }));
    const edit = async (arguments_: object, target = session) => invoke('designer_studio_edit', {
        session: target, expected_revision: (await frame(target)).revision, tool: 'designer_palette_edit', arguments: arguments_,
    });
    const image = async (theme: CanvasTheme, target = session) => {
        content(await invoke('designer_view', { session: target, theme }));
        await frame(target);
        const snapshot = await invoke('designer_preview_image', { session: target });
        assert.equal(snapshot.isError, false);
        const data = snapshot.content.find(item => item.type === 'image')?.data;
        assert.ok(data);
        return data;
    };
    const effective = async (name: string) => {
        await frame();
        const { nodes } = content<{ nodes: OutlineNode[] }>(await invoke('designer_outline', { session }));
        const node = nodes.find(node => node.name === name)?.id;
        assert.ok(node);
        return (await read<{ effective: string }>('designer_property_trace', { node, property: 'Background' })).effective;
    };

    await frame();
    content(await invoke('designer_view', { session, width: 640, height: 420 }));
    const themes = ['light', 'dark', 'high_contrast'] as const;
    const initial = new Map<CanvasTheme, string>();
    for (const theme of themes) initial.set(theme, await image(theme));
    assert.equal(new Set(initial.values()).size, themes.length, 'each appearance renders a different native image');
    assert.deepEqual((await frame()).available_themes, themes);
    assert.equal(await effective('ThemeWorkspace'), '#FF0A0B0D');
    assert.equal(await effective('AccentCard'), '#FF24334B');
    content(await invoke('designer_view', { session, width: 644 }));
    assert.deepEqual((await frame()).viewport, { width: 644, height: 420, dark: true, theme: 'high_contrast' });
    content(await invoke('designer_view', { session, width: 640 }));

    const { palette_owner: owner } = await read<StyleCatalog>('designer_style_catalog');
    assert.ok(owner);
    const sheet = await read<PaletteSheet>('designer_palette', { owner });
    assert.ok(sheet.contrastPreview && sheet.contrastProviders?.includes('winui'));
    const surface = sheet.comparison.find(row => row.key === 'PanelSurface')?.high_contrast;
    const accent = sheet.comparison.find(row => row.key === 'PanelAccent');
    assert.equal(surface?.value, '#FF0A0B0D');
    assert.ok(surface?.source?.uri.endsWith('/High%20Contrast.xaml'));
    assert.equal(accent?.dark?.value, '#FF24334B');
    assert.deepEqual(accent?.high_contrast, accent?.dark, 'the inherited color reveals its Dark declaration');
    content(await invoke('designer_view', { session, view: 'styles', styles_page: 'comparison' }));

    const addition = { action: 'add', owner, layer: 'high_contrast', key: 'PanelSurface', kind: 'solid_color_brush', color: '#FF10141A' };
    assert.equal((await edit(addition)).isError, true);
    assert.equal(view.getText(), originals[0]);
    content(await edit({ ...addition, provider: 'winui' }));
    assert.match(view.getText(), /x:Static \w+:WinUITheme\.HighContrast/);
    assert.equal(await effective('ThemeWorkspace'), '#FF10141A');
    content(await invoke('designer_history', { session, expected_version: view.version, action: 'undo' }));
    assert.equal(view.getText(), originals[0]);
    assert.equal(await image('high_contrast'), initial.get('high_contrast'));

    const opened = content<{ opened_session: number }>(await invoke('designer_studio_present', {
        session, expected_revision: (await frame()).revision, tool: 'designer_open_style_source', arguments: { source: surface.source },
    }));
    const located = await read<LocatedStyleSource>('designer_style_source', { source: surface.source }, opened.opened_session);
    assert.equal(contrast.getText(asRange(located.range)), '#FF0A0B0D');
    assert.equal(located.editor.page, 'palette');
    assert.ok(located.editor.color);
    content(await edit({ action: 'set', node: located.editor.color, color: '#FF171217' }, opened.opened_session));
    assert.equal(await effective('ThemeWorkspace'), '#FF171217', 'the consumer sees the unsaved nested dictionary');
    content(await invoke('designer_history', { session: opened.opened_session, expected_version: contrast.version, action: 'undo' }));
    assert.equal(contrast.getText(), originals[2]);

    const position = view.positionAt(originals[0].indexOf('PanelSurface'));
    const request = { path, position: { line: position.line, character: position.character }, new_name: 'WorkspaceSurface' };
    const review = content<{ review: string; themeFamily: { members: { layers: string[] }[] } }>(
        await invoke('ferroui_workspace_plan_resource_rename', request));
    assert.deepEqual(review.themeFamily.members.flatMap(member => member.layers).sort(), ['Dark', 'Default', 'HighContrast']);
    content(await invoke('ferroui_workspace_rename_resource', { ...request, review: review.review }));
    await until(() => documents.every((document, index) => document.getText() === originals[index].replaceAll('PanelSurface', 'WorkspaceSurface')));
    for (const theme of themes) assert.equal(await image(theme), initial.get(theme), `rename must preserve ${theme} pixels`);
    content(await invoke('designer_history', { session, expected_version: view.version, action: 'undo' }));
    await until(() => documents.every((document, index) => document.getText() === originals[index]));
    assert.equal(await image('high_contrast'), initial.get('high_contrast'));
    await until(() => vscode.languages.getDiagnostics(view.uri)
        .every(diagnostic => !diagnostic.message.startsWith('No indexed declaration for')));
    const { session: rust } = content<{ session: number }>(await invoke('designer_open', { path: 'views.rs' }));
    assert.notEqual(await image('light', rust), await image('high_contrast', rust));
    assert.equal((await frame(rust)).viewport.theme, 'high_contrast');
    const names = new Set([path, 'Accessible Palette.xaml', 'High Contrast.xaml', 'views.rs']);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => names.has(tab.label.replace(/^Design · /, '')));
    await vscode.window.tabGroups.close(tabs);
    console.log('VS Code palette and native preview agreed across all three themes, including nested source edits and grouped rename Undo');
}
