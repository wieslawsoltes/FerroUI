import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { ResourceMove, ResourceMoveDestination } from '../src/resourceMove';
import type { ReviewedDocument } from '../src/reviewedSources';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyResourceMove({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const path = 'ResourceMove/Workspace.xaml';
    const paletteUri = vscode.Uri.joinPath(root, 'ResourceMove', 'Palette.xaml');
    const view = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, path));
    const original = view.getText();
    const palette = Buffer.from(await vscode.workspace.fs.readFile(paletteUri)).toString();
    const position = view.positionAt(original.indexOf('x:Key="Accent"') + 'x:Key="'.length);
    const at = { path, position: { line: position.line, character: position.character } };
    const tool = async <T>(name: string, request: object) => content<T>(await invoke(name, request));
    const destination = async () => {
        const target = await tool<{ name: string; destinations: ResourceMoveDestination[] }>('ferroui_workspace_resource_move_targets', at);
        assert.equal(target.name, 'Accent');
        const entry = target.destinations.find(item => item.uri.endsWith('/ResourceMove/Palette.xaml'));
        assert.ok(entry);
        return { uri: entry.uri, request: entry.request };
    };
    const { session } = await tool<{ session: number }>('designer_open', { path });
    const image = async (dark: boolean) => {
        await tool('designer_view', { session, dark });
        assert.equal((await tool<{ status: string }>('designer_wait', { session })).status, 'Ready');
        const screenshot = await invoke('designer_preview_image', { session });
        assert.equal(screenshot.isError, false);
        const data = screenshot.content.find(item => item.type === 'image')?.data;
        assert.ok(data);
        return data;
    };
    const originalLight = await image(false);
    const originalDark = await image(true);
    assert.notEqual(originalLight, originalDark);
    const selected = await destination();
    assert.ok(selected.request.version < 0, 'The move can target an unopened dictionary');
    const request = { ...at, destination: selected };
    const plan = await tool<{ documents: ReviewedDocument[]; linkedDestination: boolean; review: string }>(
        'ferroui_workspace_plan_resource_move', request);
    assert.equal(plan.linkedDestination, true);
    assert.equal(plan.documents.length, 2);
    assert.match(plan.documents[0].after, /<ResourceInclude Source="Palette\.xaml"/);
    assert.match(plan.documents[1].after, /#2661A7/);
    assert.ok((await invoke('ferroui_workspace_move_resource', { ...request, review: 'retired' })).isError);
    assert.equal(view.getText(), original);

    const { resourceMove } = await vscode.extensions.getExtension<{ resourceMove: ResourceMove }>('FerroUI.ferroui')!.activate();
    const openReview = async () => {
        await vscode.window.showTextDocument(view);
        const selected = await destination();
        await resourceMove.openReview({ document: view, request: { version: view.version, position } }, selected);
        await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Move Resource · Accent') === true);
    };
    await openReview();
    await vscode.commands.executeCommand('ferroui.resourceMoveReview.cancel');
    assert.equal(view.getText(), original);
    await openReview();
    await vscode.commands.executeCommand('ferroui.resourceMoveReview.apply');
    const target = await vscode.workspace.openTextDocument(paletteUri);
    const changed = () => view.getText() === plan.documents[0].after && target.getText() === plan.documents[1].after;
    const restored = () => view.getText() === original && target.getText() === palette;
    await until(changed);
    assert.equal(await image(false), originalLight);
    assert.equal(await image(true), originalDark);
    await vscode.window.showTextDocument(target);
    await vscode.commands.executeCommand('undo');
    await until(restored);
    await vscode.commands.executeCommand('redo');
    await until(changed);
    await vscode.commands.executeCommand('undo');
    await until(restored);
    const current = { ...at, destination: await destination() };
    const reviewed = await tool<{ review: string }>('ferroui_workspace_plan_resource_move', current);
    await tool('ferroui_workspace_move_resource', { ...current, review: reviewed.review });
    await until(changed);
    assert.ok((await invoke('ferroui_workspace_move_resource', { ...current, review: reviewed.review })).isError);
    await tool('designer_history', { session, expected_version: view.version, action: 'undo' });
    await until(restored);
    const labels = new Set(['Workspace.xaml', 'Palette.xaml']);
    const tabs = vscode.window.tabGroups.all.flatMap(group => group.tabs)
        .filter(tab => labels.has(tab.label.replace(/^Design · /, '')));
    await vscode.window.tabGroups.close(tabs);
    console.log('Resource move preserved native Light and Dark frames through reviewed Apply, MCP, Undo and Redo');
}
