import { strict as assert } from 'node:assert';
import { realpath, rename, writeFile } from 'node:fs/promises';
import * as vscode from 'vscode';
import type { DesignerPanel } from '../src/designerPanel';
import type { DesignerWorkbench } from '../src/designerWorkbench';
import { runEditorHistory } from '../src/editorHistory';
import { until } from './wait';

export const packagedBadge = 'Assets/Approval badge.png';

export async function verifyPackagedAsset(workspace: vscode.Uri, project: vscode.Uri, designer: DesignerWorkbench,
    write: (relative: string, content: string) => Promise<vscode.Uri>): Promise<void> {
    const folder = vscode.Uri.joinPath(project, 'Assets');
    const asset = vscode.Uri.joinPath(project, ...packagedBadge.split('/'));
    await vscode.workspace.fs.createDirectory(folder);
    const originalBytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(workspace, 'Assets', 'Preview Image.png'));
    await vscode.workspace.fs.writeFile(asset, originalBytes);
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
    const replacementFile = vscode.Uri.joinPath(folder, '.approval-badge-save').fsPath;
    await writeFile(replacementFile, originalBytes);
    await rename(replacementFile, asset.fsPath);
    await until(async () => {
        const next = await panel.waitForSnapshot(AbortSignal.timeout(5_000));
        return !!next.image && next.image.png === initial.image?.png;
    });
    assert.equal(panel.document.getText(), markup);
    console.log('VS Code rendered and refreshed a published Cargo asset, and F12 resolved its physical source');

    const publishedRow = (await designer.projectAssets(panel)).find(asset => asset.published && asset.label === 'Approval badge.png');
    assert.equal(publishedRow?.id, 'ferres://audit_app/Assets/Approval%20badge.png');
    assert.equal(publishedRow?.source, publishedRow?.id);
    const address = publishedRow!.id;
    await assert.rejects(designer.projectImage(panel, address.replace('Approval', 'Unpublished')), /unavailable/);

    const xaml = '<StackPanel xmlns="https://github.com/ferroui"><Image Name="Badge" Width="120" /></StackPanel>';
    const design = await designer.openDocument(await write('PlacedBadge.xaml', xaml));
    const xamlFrame = await design.waitForSnapshot(AbortSignal.timeout(15_000));
    const image = xamlFrame.nodes.find(node => node.name === 'Badge');
    assert.ok(image);
    const target = { session: design.session, token: design.currentToken!, node: image.id };
    await design.select(target);
    const assignment = await designer.projectImageSource(design, target, address);
    await design.editNode(target, assignment);
    assert.ok(design.document.getText().includes(`Source="${address}"`));
    await assert.rejects(designer.projectImageSource(design, target, address), /current design/);
    await verifyImageAndUndo(design, file, xaml);

    const rust = 'fn badge() { rsx! { StackPanel { /* keep the child list comment 🧭 */ } } }\r\n';
    const rustDesign = await designer.openDocument(await write('src/placed_badge.rs', rust));
    const rustFrame = await rustDesign.waitForSnapshot(AbortSignal.timeout(15_000));
    await rustDesign.place({ session: rustDesign.session, token: rustDesign.currentToken! }, {
        origin: await designer.projectImage(rustDesign, address),
        target: { type: 'outline', node: rustFrame.nodes[0].id, position: 'inside' },
    });
    assert.ok(rustDesign.document.getText().includes(`::ferroui_view::asset_image("${address}")`));
    assert.ok(rustDesign.document.getText().includes('/* keep the child list comment 🧭 */'));
    await verifyImageAndUndo(rustDesign, file, rust);
    console.log('VS Code browsed published assets, assigned one in XAML, placed one in RSX, and undid each source edit');
}

async function verifyImageAndUndo(panel: DesignerPanel, file: string, original: string): Promise<void> {
    const snapshot = await panel.waitForSnapshot(AbortSignal.timeout(15_000));
    assert.ok(snapshot.image, snapshot.renderError ?? 'The edited image did not render');
    assert.ok(snapshot.dependencies?.some(uri => vscode.Uri.parse(uri, true).fsPath === file));
    const frame = { session: panel.session, token: panel.currentToken! };
    await runEditorHistory(panel.document, 'undo', () => assert.ok(panel.canUse(frame)), AbortSignal.timeout(5_000));
    assert.equal(panel.document.getText(), original);
}
