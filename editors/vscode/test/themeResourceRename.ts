import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { ResourceRename } from '../src/resourceRename';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyThemeResourceRename({ invoke, content }: McpFixture): Promise<void> {
    const path = 'IncludedStylePalette.xaml';
    const root = vscode.workspace.workspaceFolders![0].uri;
    const documents = await Promise.all([path, 'Styling/Shared Palette.xaml', 'Styling/Colors/Dark.xaml']
        .map(file => vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, file))));
    const [view] = documents;
    const originals = documents.map(document => document.getText());
    const position = view.positionAt(originals[0].indexOf('AccountAccent'));
    const request = { path, position: { line: position.line, character: position.character }, new_name: 'TeamAccent' };
    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const image = async (dark: boolean) => {
        content(await invoke('designer_view', { session, dark }));
        assert.equal(content<{ status: string }>(await invoke('designer_wait', { session })).status, 'Ready');
        const screenshot = await invoke('designer_preview_image', { session });
        assert.equal(screenshot.isError, false);
        const data = screenshot.content.find(item => item.type === 'image')?.data;
        assert.ok(data);
        return data;
    };
    const light = await image(false);
    const dark = await image(true);
    assert.notEqual(light, dark);
    const reviewed = content<{ review: string; themeFamily: { members: { layers: string[] }[] }; documents: unknown[] }>(
        await invoke('ferroui_workspace_plan_resource_rename', request));
    assert.equal(reviewed.documents.length, 3);
    assert.deepEqual(reviewed.themeFamily.members.flatMap(member => member.layers).sort(), ['Dark', 'Light']);

    const { resourceRename } = await vscode.extensions.getExtension<{ resourceRename: ResourceRename }>('FerroUI.ferroui')!.activate();
    await vscode.window.showTextDocument(view);
    await resourceRename.openReview({ document: view, request: { position, version: view.version } }, request.new_name);
    await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Rename Theme Resource · AccountAccent → TeamAccent') === true);
    await vscode.commands.executeCommand('ferroui.resourceRenameReview.apply');
    const renamed = originals.map(source => source.replaceAll('AccountAccent', 'TeamAccent'));
    await until(() => documents.every((document, index) => document.getText() === renamed[index]));
    assert.equal(await image(false), light);
    assert.equal(await image(true), dark);
    assert.equal((await invoke('ferroui_workspace_rename_resource', { ...request, review: reviewed.review })).isError, true);
    content(await invoke('designer_history', { session, expected_version: view.version, action: 'undo' }));
    await until(() => documents.every((document, index) => document.getText() === originals[index]));
    assert.equal(await image(true), dark);
    const labels = new Set([path, 'Shared Palette.xaml', 'Dark.xaml']);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => labels.has(tab.label.replace(/^Design · /, '')));
    await vscode.window.tabGroups.close(tabs);
    console.log('Theme-family rename updated inline and included declarations and preserved both native frames through Apply and Undo');
}
