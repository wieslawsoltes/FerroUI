import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { Bounds, CanvasInteraction, Viewport } from '../src/protocol';
import type { McpFixture } from './mcp';

export async function verifyCanvasInteraction(mcp: McpFixture, session: number, document: vscode.TextDocument): Promise<void> {
    const { invoke, content } = mcp;
    const original = document.getText();
    const version = document.version;
    const canvas = async (mode: 'select' | 'interact') => content<{ mode: string }>(
        await invoke('designer_canvas', { session, mode, tab: 'preview', pan_tool: false }));
    assert.equal((await canvas('interact')).mode, 'interact');
    const preview = content<{ viewport: Viewport; revision: number }>(await invoke('designer_wait', { session }));
    const { nodes } = content<{ nodes: { bounds?: Bounds; label: string; id: number }[] }>(await invoke('designer_outline', { session }));
    const text = nodes.find(node => node.label === 'TextBox');
    assert.ok(text?.bounds, 'An authored text field has native layout');
    const { x, y, width, height } = text.bounds;
    const click: CanvasInteraction = { kind: 'click', x: (x + width / 2) / preview.viewport.width, y: (y + height / 2) / preview.viewport.height };
    let revision = preview.revision;
    for (const input of [click, { kind: 'key', key: 'Ctrl+A' }, { kind: 'text', text: 'Native preview text 🦀' }] satisfies CanvasInteraction[]) {
        const next = content<{ revision: number; version: number }>(
            await invoke('designer_canvas_input', { session, expected_revision: revision, input }));
        assert.ok(next.revision > revision, 'Input publishes a new native frame');
        assert.equal(next.version, version);
        revision = next.revision;
    }
    assert.ok((await invoke('designer_canvas_input', { session, expected_revision: preview.revision, input: click })).isError);
    const inspector = content<{ properties: { name: string; authored: string; effective: string }[] }>(
        await invoke('designer_inspect', { session, node: text.id }));
    const value = inspector.properties.find(field => field.name === 'Text');
    assert.equal(value?.authored, 'Northwind');
    assert.equal(value?.effective, 'Native preview text 🦀');
    assert.equal(document.getText(), original);
    assert.equal((await canvas('select')).mode, 'select');
    console.log(`The VS Code canvas and MCP exercised native preview input for ${document.languageId}, preserved source and rejected stale input`);
}

export async function verifyCanvasScroll({ invoke, content }: McpFixture): Promise<void> {
    const name = 'PreviewScroll.xaml';
    const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders![0].uri, name);
    const items = Array.from({ length: 30 }, (_, index) => `<TextBlock Height="42" Text="Record ${index}"/>`).join('');
    const source = '<Grid xmlns="https://github.com/ferroui"><ScrollViewer Width="300" Height="140" '
        + `HorizontalAlignment="Right" VerticalAlignment="Top"><StackPanel>${items}</StackPanel></ScrollViewer></Grid>`;
    await vscode.workspace.fs.writeFile(uri, Buffer.from(source));
    const { session, revision, viewport } = content<{ session: number; revision: number; viewport: Viewport }>(
        await invoke('designer_open', { path: name }));
    const { nodes } = content<{ nodes: { label: string; bounds?: Bounds }[] }>(await invoke('designer_outline', { session }));
    const view = nodes.find(node => node.label === 'ScrollViewer')?.bounds;
    assert.ok(view, 'The scroll viewport has measured bounds');
    const png = async () => (await invoke('designer_preview_image', { session })).content.find(item => item.type === 'image')?.data;
    const before = await png();
    assert.ok(before);
    content(await invoke('designer_canvas_input', { session, expected_revision: revision, input: {
        kind: 'scroll', x: (view.x + 40) / viewport.width, y: (view.y + 40) / viewport.height, dx: 0, dy: -3,
    } }));
    const after = await png();
    assert.ok(after && after !== before, 'An off-center wheel action scrolls the native viewport');
    console.log('VS Code MCP scrolled the native control directly under the canvas pointer');
}
