import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { McpFixture } from './mcp';
import { until } from './wait';

export async function verifyAssetImport({ invoke, content }: McpFixture): Promise<void> {
    const root = vscode.workspace.workspaceFolders![0].uri;
    const request = { source: 'Import Input.PnG', destination: 'Assets/Customer Łódź.PnG', folder: root.toString() };
    const from = vscode.Uri.joinPath(root, request.source);
    const to = vscode.Uri.joinPath(root, ...request.destination.split('/'));
    const newFolder = vscode.Uri.joinPath(root, 'Assets', 'Reviewed Imports');
    const nested = vscode.Uri.joinPath(newFolder, 'Copy.PnG');
    const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==', 'base64');
    const changed = Buffer.concat([image, Buffer.from([23])]);
    const occupant = Buffer.from('existing project file');
    const plan = async () => content<{ review: string; bytes: number; destination: string; kind: string }>(
        await invoke('ferroui_project_plan_asset_import', request));
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(root, 'Assets'));
    await vscode.workspace.fs.writeFile(from, image);
    try {
        const reviewed = await plan();
        assert.equal(reviewed.bytes, image.length);
        assert.equal(reviewed.destination, request.destination);
        assert.equal(reviewed.kind, 'Image');
        await vscode.workspace.fs.writeFile(from, changed);
        assert.match((await invoke('ferroui_project_import_asset', { ...request, review: reviewed.review })).content[0]?.text ?? '', /Review the current source/);
        await assert.rejects(async () => vscode.workspace.fs.stat(to));

        const current = await plan();
        assert.notEqual(current.review, reviewed.review);
        await vscode.workspace.fs.writeFile(to, occupant);
        assert.ok((await invoke('ferroui_project_import_asset', { ...request, review: current.review })).isError);
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(to)), occupant);
        await vscode.workspace.fs.delete(to, { useTrash: false });
        content(await invoke('ferroui_project_import_asset', { ...request, review: current.review }));
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(to)), changed);
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(from)), changed);
        await until(async () => {
            const { files } = content<{ files: { path: string }[] }>(await invoke('ferroui_project_files', { kind: 'asset', folder: request.folder }));
            return files.some(file => file.path === request.destination);
        });

        const newRequest = { ...request, destination: 'Assets/Reviewed Imports/Copy.PnG' };
        const newPlan = content<{ review: string }>(await invoke('ferroui_project_plan_asset_import', newRequest));
        content(await invoke('ferroui_project_import_asset', { ...newRequest, review: newPlan.review }));
        assert.deepEqual(Buffer.from(await vscode.workspace.fs.readFile(nested)), changed);
    } finally {
        for (const uri of [nested, newFolder, to, from]) {
            const present = await vscode.workspace.fs.stat(uri).then(() => true, () => false);
            if (present) await vscode.workspace.fs.delete(uri, { useTrash: false });
        }
    }
    console.log('Reviewed asset import preserved the binary bytes, rejected stale data and collisions, and appeared in the Assets index');
}
