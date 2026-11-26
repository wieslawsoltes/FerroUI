import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { ResourceRename } from '../src/resourceRename';
import type { ReviewedDocument } from '../src/reviewedSources';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyResourceRename({ invoke, content }: McpFixture): Promise<void> {
    const path = 'ResourceRename/AccountCard.xaml';
    const root = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, 'ResourceRename');
    const account = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(root, 'AccountCard.xaml'));
    const paletteUri = vscode.Uri.joinPath(root, 'Palette.xaml');
    const queueUri = vscode.Uri.joinPath(root, 'Queue.xaml');
    const queueOnDisk = Buffer.from(await vscode.workspace.fs.readFile(queueUri)).toString('utf8');
    const paletteOnDisk = Buffer.from(await vscode.workspace.fs.readFile(paletteUri)).toString('utf8');
    const original = account.getText();
    const { line, character } = account.positionAt(original.indexOf('BrandInk') + 1);
    const position = { line, character };
    const request = { path, position, new_name: 'Account Ink' };
    const plan = async () => content<{ review: string; documents: ReviewedDocument[] }>(
        await invoke('ferroui_workspace_plan_resource_rename', request));

    const { session } = content<{ session: number }>(await invoke('designer_open', { path }));
    const image = async () => {
        assert.equal(content<{ status: string }>(await invoke('designer_wait', { session })).status, 'Ready');
        const frame = await invoke('designer_preview_image', { session });
        assert.equal(frame.isError, false);
        return frame.content.find(item => item.type === 'image')?.data;
    };
    const before = await image();
    assert.ok(before);
    const stale = await plan();
    assert.equal(stale.documents.length, 3);
    assert.ok(stale.documents.find(document => document.edit.uri === queueUri.toString())!.edit.version < 0);
    const changedQueue = queueOnDisk.replace('Review queue', 'Pending work');
    await vscode.workspace.fs.writeFile(queueUri, Buffer.from(changedQueue));
    const retired = await invoke('ferroui_workspace_rename_resource', { ...request, review: stale.review });
    assert.ok(retired.isError);
    assert.match(retired.content[0]?.text ?? '', /changed/);
    assert.equal(account.getText(), original);

    const extension = vscode.extensions.getExtension<{ resourceRename: ResourceRename }>('FerroUI.ferroui')!;
    const { resourceRename } = await extension.activate();
    const editor = await vscode.window.showTextDocument(account, { preview: false });
    const openReview = () => resourceRename.openReview({ document: account, request: { position, version: account.version } }, request.new_name);
    await openReview();
    await until(() => vscode.window.tabGroups.activeTabGroup.activeTab?.label.startsWith('Rename Resource · BrandInk → Account Ink') === true);
    await vscode.commands.executeCommand('ferroui.resourceRenameReview.cancel');
    assert.equal(account.getText(), original);
    await openReview();
    await vscode.commands.executeCommand('ferroui.resourceRenameReview.apply');
    await until(() => account.getText().includes('Account Ink'));
    const text = async (uri: vscode.Uri) => (await vscode.workspace.openTextDocument(uri)).getText();
    const expectedQueue = changedQueue.replace('{DynamicResource BrandInk}', '{DynamicResource &apos;Account Ink&apos;}');
    const expectedPalette = paletteOnDisk.replace('BrandInk', 'Account Ink');
    const renamed = () => until(async () => account.getText().includes('Account Ink')
        && await text(queueUri) === expectedQueue && await text(paletteUri) === expectedPalette);
    await renamed();
    assert.equal(await image(), before, 'the renamed resource keeps the native frame identical');
    const undo = async () => {
        await vscode.window.showTextDocument(account, { viewColumn: editor.viewColumn });
        await vscode.commands.executeCommand('undo');
        await until(async () => account.getText() === original
            && await text(paletteUri) === paletteOnDisk && await text(queueUri) === changedQueue);
    };
    await undo();
    content(await invoke('ferroui_workspace_rename_resource', { ...request, review: (await plan()).review }));
    await renamed();
    await undo();
    await vscode.workspace.fs.writeFile(queueUri, Buffer.from(queueOnDisk));
    const labels = new Set(['Design · AccountCard.xaml', 'AccountCard.xaml', 'Palette.xaml', 'Queue.xaml']);
    await vscode.window.tabGroups.close(vscode.window.tabGroups.all.flatMap(group => group.tabs).filter(tab => labels.has(tab.label)));
    console.log('VS Code and MCP reviewed, applied and undid the included resource rename while preserving its native image and local shadow.');
}
