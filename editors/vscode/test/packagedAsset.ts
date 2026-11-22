import { strict as assert } from 'node:assert';
import { realpath, writeFile } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { until } from './wait';

export const packagedBadge = 'Assets/Approval badge.png';

export async function verifyPackagedAsset(workspace: vscode.Uri, project: vscode.Uri, designer: DesignerWorkbench,
    write: (relative: string, content: string) => Promise<vscode.Uri>): Promise<void> {
    const folder = vscode.Uri.joinPath(project, 'Assets');
    const asset = vscode.Uri.joinPath(project, ...packagedBadge.split('/'));
    await vscode.workspace.fs.createDirectory(folder);
    await vscode.workspace.fs.copy(vscode.Uri.joinPath(workspace, 'Assets', 'Preview Image.png'), asset);
    const published = 'ferres://Audit_App/Assets/Approval%20badge.png';
    const markup = `<Image xmlns="https://github.com/ferroui" Source="${published}" Width="180" Height="90" />`;
    const panel = await designer.openDocument(await write('PackagedBadge.xaml', markup));
    const initial = await panel.waitForSnapshot(AbortSignal.timeout(15_000));
    assert.ok(initial.image, initial.renderError ?? 'The published raster did not render');
    const file = await realpath(asset.fsPath);
    assert.ok(initial.dependencies?.some(uri => vscode.Uri.parse(uri, true).fsPath === file));
    const offset = panel.document.positionAt(markup.indexOf(published) + 16);
    const definition = await vscode.commands.executeCommand<vscode.Location[]>('vscode.executeDefinitionProvider', panel.document.uri, offset);
    assert.equal(definition?.[0]?.uri.fsPath, file);

    // The same source must render a different image after an asset-only save.
    const replacement = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAYAAAADCAIAAAA/Y+msAAAAEUlEQVR4nGMwDpqKhhiIEgIALo0T1QA/I98AAAAASUVORK5CYII=', 'base64');
    await writeFile(asset.fsPath, replacement);
    await until(async () => {
        const next = await panel.waitForSnapshot(AbortSignal.timeout(5_000));
        return !!next.image && next.image.png !== initial.image?.png;
    });
    assert.equal(panel.document.getText(), markup);
    console.log('VS Code rendered and refreshed a published Cargo asset, and F12 resolved its physical source');
}
